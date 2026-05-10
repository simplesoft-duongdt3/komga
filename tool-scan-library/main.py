#!/usr/bin/env python3
"""Komga fast library scan tool.

Syncs PDF books and JPG thumbnails from the filesystem into Komga's PostgreSQL database.
Parses Mylar-format series.json for series metadata.

Usage:
    # Dry-run (scan only, no DB writes)
    python main.py --dry-run

    # Sync with defaults from env or config.py
    python main.py

    # Export SQL to file instead of writing directly to DB
    python main.py --export-sql sync.sql

    # Sync with explicit path
    python main.py --library-root /data/library --library-id XYZ

    # Sync with write credentials
    python main.py --write-user komga_admin --write-pass secret

    # Sync with specific settings
    python main.py --workers 16 --batch-size 5000

    # Full pipeline: scan + analyze (page count, dimensions, file hash)
    python main.py --analyze

    # Export scan SQL only
    python main.py --export-sql scan.sql

    # Export analyze SQL only (run after scan.sql is applied)
    python main.py --analyze --analyze-sql analyze.sql

    # Full pipeline with SQL export for both phases
    python main.py --analyze --export-sql scan.sql --analyze-sql analyze.sql

    # Analyze at most 100 books, skip hashing for speed
    python main.py --analyze --analyze-limit 100 --no-hash

Environment variables:
    KOMGA_DB_HOST          PostgreSQL host (default: 192.168.1.169)
    KOMGA_DB_PORT          PostgreSQL port (default: 5433)
    KOMGA_DB_NAME          Database name (default: komga)
    KOMGA_DB_USER          Read-only user (default: ai_readonly)
    KOMGA_DB_PASS          Read-only password (default: ai_readonly_pass)
    KOMGA_DB_WRITE_USER    Write user (falls back to KOMGA_DB_USER)
    KOMGA_DB_WRITE_PASS    Write password (falls back to KOMGA_DB_PASS)
    KOMGA_DB_MIN_CONN      Min pool connections (default: 2)
    KOMGA_DB_MAX_CONN      Max pool connections (default: 10)
    KOMGA_LIBRARY_ID       Library ID (default: 0Q3CKC76902B7)
    KOMGA_LIBRARY_ROOT     Library root path (same as mounted in Komga)
    KOMGA_SCAN_WORKERS     Scanner threads (default: cpu_count * 2, max 32)
    KOMGA_BATCH_SIZE       DB commit batch size (default: 5000)
"""

from __future__ import annotations

import argparse
import datetime
import logging
import os
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Any

from analyzer import PdfAnalyzer
from config import Config
from db import KomgaDb
from scanner import scan_library
from sql_exporter import export_sql, export_analyze_sql
from syncer import diff, DiffResult


def _configure_logging(verbose: bool) -> logging.Logger:
    logger = logging.getLogger("komga_scan")
    handler = logging.StreamHandler(sys.stderr)
    handler.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
    logger.addHandler(handler)
    logger.setLevel(logging.DEBUG if verbose else logging.INFO)
    return logger


def _summarize_diff(dr: DiffResult, logger: logging.Logger) -> None:
    new_series = len(dr.new_series)
    new_books = len(dr.new_books)
    deleted_series = len(dr.deleted_series_ids)
    deleted_books = len(dr.deleted_book_ids)
    series_updates = len(dr.series_count_updates)
    metadata_updates = len(dr.series_metadata_updates)
    new_sthumb = len(dr.new_series_thumbnails)
    new_bthumb = len(dr.new_book_thumbnails)
    st_updates = len(dr.series_thumbnail_updates)
    bt_updates = len(dr.book_thumbnail_updates)

    logger.info("─" * 60)
    logger.info("Diff summary:")
    logger.info(f"  New series:       {new_series}")
    logger.info(f"  New books:        {new_books}")
    logger.info(f"  Deleted series:   {deleted_series}")
    logger.info(f"  Deleted books:    {deleted_books}")
    logger.info(f"  Series count updates: {series_updates}")
    logger.info(f"  Metadata updates:     {metadata_updates}")
    logger.info(f"  New series thumbs:    {new_sthumb}")
    logger.info(f"  New book thumbs:      {new_bthumb}")
    logger.info(f"  Series thumb updates: {st_updates}")
    logger.info(f"  Book thumb updates:   {bt_updates}")
    logger.info(f"  Deleted series thumbs: {len(dr.deleted_series_thumb_series_ids)}")
    logger.info(f"  Deleted book thumbs:   {len(dr.deleted_book_thumb_book_ids)}")
    logger.info(f"  Reactivated books:     {len(dr.reactivate_book_ids)}")
    logger.info("─" * 60)


def _apply_diff(db: KomgaDb, dr: DiffResult, logger: logging.Logger) -> None:
    total_ops = (
        (1 if dr.new_series else 0) + (1 if dr.new_books else 0) +
        (1 if dr.deleted_series_ids else 0) + (1 if dr.deleted_book_ids else 0) +
        (1 if dr.series_count_updates else 0) + (1 if dr.series_metadata_updates else 0) +
        (1 if dr.new_series_thumbnails else 0) + (1 if dr.new_book_thumbnails else 0) +
        (1 if dr.series_thumbnail_updates else 0) + (1 if dr.book_thumbnail_updates else 0) +
        (1 if dr.deleted_series_thumb_series_ids else 0) + (1 if dr.deleted_book_thumb_book_ids else 0)
    )
    if total_ops == 0:
        logger.info("No changes to apply. Database is in sync.")
        return

    logger.info("Applying changes...")

    # 1. Delete operations first
    if dr.deleted_book_thumb_book_ids:
        logger.info("  Deleting %d book thumbnails...", len(dr.deleted_book_thumb_book_ids))
        db.delete_book_thumbnails_for_books(dr.deleted_book_thumb_book_ids)

    if dr.reactivate_book_ids:
        logger.info("  Reactivating %d books...", len(dr.reactivate_book_ids))
        db.reactivate_books(dr.reactivate_book_ids)

    if dr.deleted_series_thumb_series_ids:
        logger.info("  Deleting %d series thumbnails...", len(dr.deleted_series_thumb_series_ids))
        db.delete_series_thumbnails_for_series(dr.deleted_series_thumb_series_ids)

    if dr.deleted_book_ids:
        logger.info("  Soft-deleting %d books...", len(dr.deleted_book_ids))
        db.soft_delete_books(dr.deleted_book_ids)

    if dr.deleted_series_ids:
        logger.info("  Soft-deleting %d series...", len(dr.deleted_series_ids))
        db.soft_delete_series(dr.deleted_series_ids)

    # 2. Insert new series and books
    # Assign series_id to new series thumbnails and books
    if dr.new_series:
        logger.info("  Creating %d new series...", len(dr.new_series))
        series_ids = db.insert_series_batch(dr.new_series)

        for book in dr.new_books:
            if "_series_idx" in book and "series_id" not in book:
                book["series_id"] = series_ids[book["_series_idx"]]

        for thumb in dr.new_series_thumbnails:
            if "_series_idx" in thumb:
                thumb["series_id"] = series_ids[thumb["_series_idx"]]

    # 3. Insert all new books
    all_new_books = [b for b in dr.new_books if "series_id" in b]
    if all_new_books:
        logger.info("  Creating %d new books...", len(all_new_books))
        book_ids = db.insert_books_batch(all_new_books)

        # Assign book_id to new book thumbnails using _book_idx
        for thumb in dr.new_book_thumbnails:
            if "_book_idx" in thumb and "book_id" not in thumb:
                idx = thumb["_book_idx"]
                if idx < len(book_ids):
                    thumb["book_id"] = book_ids[idx]

    # 4. Insert/update thumbnails
    if dr.new_series_thumbnails:
        valid = [t for t in dr.new_series_thumbnails if "series_id" in t]
        if valid:
            logger.info("  Creating %d series thumbnails...", len(valid))
            db.insert_series_thumbnails(valid)

    if dr.series_thumbnail_updates:
        logger.info("  Updating %d series thumbnails...", len(dr.series_thumbnail_updates))
        db.update_series_thumbnails(dr.series_thumbnail_updates)

    if dr.new_book_thumbnails:
        valid_bts = [t for t in dr.new_book_thumbnails if t.get("book_id")]
        if valid_bts:
            logger.info("  Creating %d book thumbnails...", len(valid_bts))
            db.insert_book_thumbnails(valid_bts)

    if dr.book_thumbnail_updates:
        logger.info("  Updating %d book thumbnails...", len(dr.book_thumbnail_updates))
        db.update_book_thumbnails(dr.book_thumbnail_updates)

    # 5. Update series counts and metadata
    if dr.series_count_updates:
        logger.info("  Updating %d series counts...", len(dr.series_count_updates))
        db.update_series_counts(dr.series_count_updates)

    if dr.series_metadata_updates:
        logger.info("  Updating %d series metadata...", len(dr.series_metadata_updates))
        db.update_series_metadata(dr.series_metadata_updates)

    logger.info("Sync complete.")


def _run_analyze(
    db: KomgaDb,
    config: Config,
    limit: int | None,
    skip_hash: bool,
    skip_dimensions: bool,
    logger: logging.Logger,
    export_file: str | None = None,
) -> None:
    """Analyze all books with MEDIA.STATUS='UNKNOWN', updating MEDIA, MEDIA_PAGE, and BOOK.FILE_HASH.

    If export_file is set, SQL is written to that file instead of executing against the DB.
    """
    analyzer = PdfAnalyzer(config.library.library_root_path)
    max_workers = config.library.scan_workers or max(1, (os.cpu_count() or 4) * 2)
    max_workers = min(max_workers, 32)
    batch_size = config.sync.commit_batch_size

    books = db.fetch_unanalyzed_books(limit=limit)
    total = len(books)
    if total == 0:
        logger.info("No unanalyzed books found. All books have been analyzed.")
        return

    logger.info("Found %d books with UNKNOWN media status. Using %d workers.", total, max_workers)

    processed = 0
    errors = 0

    def _analyze_one(book_row: dict) -> tuple[str, str | None, dict | None]:
        book_id = book_row["ID"]
        docker_url = book_row["URL"]
        try:
            result = analyzer.analyze(docker_url, skip_dimensions=skip_dimensions, skip_hash=skip_hash)
            return (book_id, None, result)
        except Exception as e:
            return (book_id, str(e), None)

    f: Any = None
    if export_file:
        f = open(export_file, "w", encoding="utf-8")
        export_now = datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
        f.write("-- Komga analyze — generated SQL\n")
        f.write(f"-- Generated at: {export_now}\n")
        f.write(f"-- Books to analyze: {total}\n")
        f.write("BEGIN;\n\n")

    try:
        for batch_start in range(0, total, batch_size):
            batch = books[batch_start:batch_start + batch_size]
            batch_num = batch_start // batch_size + 1
            total_batches = (total + batch_size - 1) // batch_size
            logger.info("Analyzing batch %d/%d (%d books)...", batch_num, total_batches, len(batch))

            media_updates: list[dict] = []
            book_hash_updates: list[dict] = []
            all_pages: list[dict] = []

            with ThreadPoolExecutor(max_workers=max_workers) as executor:
                futures = {executor.submit(_analyze_one, b): b["ID"] for b in batch}
                for future in as_completed(futures):
                    book_id, error, result = future.result()
                    if error:
                        media_updates.append({
                            "book_id": book_id,
                            "status": "ERROR",
                            "page_count": 0,
                            "media_type": None,
                            "comment": error[:2000],
                        })
                        errors += 1
                    else:
                        media_updates.append({
                            "book_id": book_id,
                            "status": "READY",
                            "page_count": result["page_count"],
                            "media_type": "application/pdf",
                            "comment": None,
                        })
                        if result["file_hash"]:
                            book_hash_updates.append({
                                "book_id": book_id,
                                "file_hash": result["file_hash"],
                            })
                        for page in result["pages"]:
                            all_pages.append({
                                "book_id": book_id,
                                **page,
                            })

            if f is not None:
                batch_now = datetime.datetime.now(datetime.UTC).replace(tzinfo=None)
                export_analyze_sql(f, media_updates, book_hash_updates, all_pages, batch_now, logger)
            else:
                if media_updates:
                    db.update_media_analyzed(media_updates)
                if book_hash_updates:
                    db.update_book_hashes(book_hash_updates)
                if all_pages:
                    db.insert_media_pages_batch(all_pages)

            processed += len(batch)
            logger.info("  Progress: %d/%d (errors: %d)", processed, total, errors)

    finally:
        if f is not None:
            f.write("COMMIT;\n")
            f.close()
            logger.info("Analyze SQL written to %s", export_file)

    logger.info("Analysis complete: %d processed, %d errors, %d OK", processed, errors, processed - errors)


def main():
    parser = argparse.ArgumentParser(
        description="Komga fast library scan and sync tool (PDF + JPG thumbnails + Mylar series.json)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--dry-run", action="store_true", help="Scan only, do not write to DB")
    parser.add_argument("--export-sql", metavar="FILE", help="Export SQL to file instead of writing to DB")
    parser.add_argument("--db-host", help="PostgreSQL host")
    parser.add_argument("--db-port", type=int, help="PostgreSQL port")
    parser.add_argument("--db-name", help="Database name")
    parser.add_argument("--db-user", help="Read-only user")
    parser.add_argument("--db-pass", help="Read-only password")
    parser.add_argument("--write-user", help="Write user")
    parser.add_argument("--write-pass", help="Write password")
    parser.add_argument("--library-id", help="Library ID")
    parser.add_argument("--library-root", help="Library root path (same as mounted in Komga)")
    parser.add_argument("--workers", type=int, help="Scanner thread count")
    parser.add_argument("--batch-size", type=int, help="DB commit batch size")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose logging")
    parser.add_argument("--analyze", action="store_true", help="Analyze UNKNOWN books (page count, dimensions, file hash)")
    parser.add_argument("--analyze-limit", type=int, default=None, help="Max books to analyze")
    parser.add_argument("--no-hash", action="store_true", help="Skip SHA-256 hashing during analysis")
    parser.add_argument("--no-dimensions", action="store_true", help="Skip page dimensions during analysis")
    parser.add_argument("--analyze-sql", metavar="FILE", help="Export analyze SQL to file instead of writing to DB")
    args = parser.parse_args()

    logger = _configure_logging(args.verbose)
    config = Config.from_env()

    # Override from CLI args
    if args.db_host:
        config.db.host = args.db_host
    if args.db_port:
        config.db.port = args.db_port
    if args.db_name:
        config.db.database = args.db_name
    if args.db_user:
        config.db.user = args.db_user
    if args.db_pass:
        config.db.password = args.db_pass
    if args.write_user:
        config.db.write_user = args.write_user
    if args.write_pass:
        config.db.write_password = args.write_pass
    if args.library_id:
        config.library.library_id = args.library_id
    if args.library_root:
        config.library.library_root_path = args.library_root
    if args.workers is not None:
        config.library.scan_workers = args.workers
    if args.batch_size is not None:
        config.sync.commit_batch_size = args.batch_size

    if not config.library.library_root_path:
        logger.error("--library-root is required.")
        sys.exit(1)

    logger.info("Library root: %s", config.library.library_root_path)
    logger.info("Library ID:  %s", config.library.library_id)
    logger.info("DB:          %s:%d/%s", config.db.host, config.db.port, config.db.database)
    logger.info("Workers:     %d", config.library.scan_workers or max(1, (__import__("os").cpu_count() or 4) * 2))

    # Phase 1: Scan filesystem
    t0 = time.monotonic()
    logger.info("Phase 1: Scanning filesystem...")
    disk_series = scan_library(config)
    t1 = time.monotonic()
    total_books = sum(s["book_count"] for s in disk_series.values())
    logger.info("Found %d series, %d books on disk (%.2fs)",
                len(disk_series), total_books, t1 - t0)

    if args.dry_run:
        logger.info("Dry-run mode. Showing top-level results:")
        sorted_series = sorted(disk_series.values(), key=lambda s: s["title"])
        for s in sorted_series[:20]:
            thumb = " [poster]" if "series_thumbnail" in s else ""
            logger.info("  %s (%d books)%s", s["title"], s["book_count"], thumb)
        if len(sorted_series) > 20:
            logger.info("  ... and %d more series", len(sorted_series) - 20)
        return

    # Connect to DB (read-only for diff; write-capable for apply; export-sql only needs read)
    db = KomgaDb(config)

    try:
        library = db.fetch_library()
        if library is None:
            logger.error("Library %s not found in database.", config.library.library_id)
            sys.exit(1)
        logger.info("Library: %s", library["NAME"])

        # Phase 2: Diff
        t2 = time.monotonic()
        logger.info("Phase 2: Computing diff...")
        dr = diff(db, disk_series, config.library.library_id)
        t3 = time.monotonic()
        logger.info("Diff computed in %.2fs", t3 - t2)
        _summarize_diff(dr, logger)

        # Phase 3: Apply or Export
        if args.export_sql:
            logger.info("Phase 3: Exporting SQL to %s...", args.export_sql)
            with open(args.export_sql, "w", encoding="utf-8") as f:
                export_sql(f, dr, logger)
        else:
            logger.info("Phase 3: Applying changes...")
            _apply_diff(db, dr, logger)

        # Phase 4: Analyze (only if --analyze is set)
        if args.analyze:
            t4a = time.monotonic()
            logger.info("Phase 4: Analyzing UNKNOWN books...")
            _run_analyze(
                db, config,
                limit=args.analyze_limit,
                skip_hash=args.no_hash,
                skip_dimensions=args.no_dimensions,
                logger=logger,
                export_file=args.analyze_sql,
            )
            t4b = time.monotonic()
            logger.info("Analysis completed in %.2fs", t4b - t4a)

        t4 = time.monotonic()
        logger.info("Total time: %.2fs", t4 - t0)
    finally:
        db.close()


if __name__ == "__main__":
    main()
