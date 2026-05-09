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

    # Sync with explicit paths
    python main.py --real-root /path/to/library --docker-root /data/library --library-id XYZ

    # Sync with write credentials
    python main.py --write-user komga_admin --write-pass secret

    # Sync with specific settings
    python main.py --workers 16 --batch-size 5000

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
    KOMGA_REAL_ROOT        Real filesystem root path
    KOMGA_DOCKER_ROOT      Docker container root path
    KOMGA_SCAN_WORKERS     Scanner threads (default: cpu_count * 2, max 32)
    KOMGA_BATCH_SIZE       DB commit batch size (default: 5000)
"""

from __future__ import annotations

import argparse
import logging
import sys
import time
from typing import Any

from config import Config
from db import KomgaDb
from scanner import scan_library
from sql_exporter import export_sql
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
    parser.add_argument("--real-root", help="Real filesystem root path")
    parser.add_argument("--docker-root", help="Docker container root path")
    parser.add_argument("--workers", type=int, help="Scanner thread count")
    parser.add_argument("--batch-size", type=int, help="DB commit batch size")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose logging")
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
    if args.real_root:
        config.library.real_root_path = args.real_root
    if args.docker_root:
        config.library.docker_root_path = args.docker_root
    if args.workers is not None:
        config.library.scan_workers = args.workers
    if args.batch_size is not None:
        config.sync.commit_batch_size = args.batch_size

    if not config.library.real_root_path or not config.library.docker_root_path:
        logger.error("Both --real-root and --docker-root are required.")
        sys.exit(1)

    logger.info("Real root:   %s", config.library.real_root_path)
    logger.info("Docker root: %s", config.library.docker_root_path)
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

        t4 = time.monotonic()
        logger.info("Total time: %.2fs", t4 - t0)
    finally:
        db.close()


if __name__ == "__main__":
    main()
