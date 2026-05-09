from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import Any

from db import KomgaDb

logger = logging.getLogger(__name__)


@dataclass
class DiffResult:
    new_series: list[dict[str, Any]] = field(default_factory=list)
    new_books: list[dict[str, Any]] = field(default_factory=list)
    new_series_thumbnails: list[dict[str, Any]] = field(default_factory=list)
    new_book_thumbnails: list[dict[str, Any]] = field(default_factory=list)

    series_count_updates: list[dict[str, Any]] = field(default_factory=list)
    series_metadata_updates: list[dict[str, Any]] = field(default_factory=list)
    series_thumbnail_updates: list[dict[str, Any]] = field(default_factory=list)
    book_thumbnail_updates: list[dict[str, Any]] = field(default_factory=list)

    reactivate_book_ids: list[str] = field(default_factory=list)

    deleted_series_ids: list[str] = field(default_factory=list)
    deleted_book_ids: list[str] = field(default_factory=list)
    deleted_series_thumb_series_ids: list[str] = field(default_factory=list)
    deleted_book_thumb_book_ids: list[str] = field(default_factory=list)


def diff(
    db: KomgaDb,
    disk_series: dict[str, dict[str, Any]],
    library_id: str,
) -> DiffResult:
    """Compare filesystem state with DB state and produce a DiffResult."""
    result = DiffResult()

    db_series = db.fetch_existing_series()
    db_series_meta = db.fetch_existing_series_metadata()
    db_books = db.fetch_existing_books()
    db_soft_deleted_books = db.fetch_soft_deleted_books()
    db_series_meta = db.fetch_existing_series_metadata()
    db_series_thumbs = db.fetch_existing_series_thumbnails()
    db_book_thumbs = db.fetch_existing_book_thumbnails()

    db_books_by_series: dict[str, list[dict[str, Any]]] = {}
    for book_url, book_row in db_books.items():
        sid = book_row["SERIES_ID"]
        db_books_by_series.setdefault(sid, []).append(book_row)

    disk_urls = set(disk_series.keys())
    db_urls = set(db_series.keys())

    # Global counter for new book index
    _book_counter = [0]

    # ── New series (on disk, not in DB) ─────────────────────────────────
    new_series_urls = disk_urls - db_urls
    for url in new_series_urls:
        s = disk_series[url]
        series_idx = len(result.new_series)
        result.new_series.append(s)
        for book in s.get("books", []):
            # Check if this book was previously soft-deleted
            if book["url"] in db_soft_deleted_books:
                result.reactivate_book_ids.append(db_soft_deleted_books[book["url"]]["ID"])
                if "book_thumbnail" in book:
                    result.new_book_thumbnails.append({
                        **book["book_thumbnail"],
                        "book_id": db_soft_deleted_books[book["url"]]["ID"],
                    })
                continue
            book_idx = _book_counter[0]
            _book_counter[0] += 1
            book["_book_idx"] = book_idx
            book["_series_idx"] = series_idx
            result.new_books.append(book)
            if "book_thumbnail" in book:
                result.new_book_thumbnails.append({
                    **book["book_thumbnail"],
                    "_book_idx": book_idx,
                    "_series_idx": series_idx,
                })
        if "series_thumbnail" in s:
            result.new_series_thumbnails.append({
                **s["series_thumbnail"],
                "_series_idx": series_idx,
            })

    # ── Deleted series (in DB, not on disk) ─────────────────────────────
    deleted_series_urls = db_urls - disk_urls
    for url in deleted_series_urls:
        series_row = db_series[url]
        sid = series_row["ID"]
        result.deleted_series_ids.append(sid)
        if sid in db_series_thumbs:
            result.deleted_series_thumb_series_ids.append(sid)
        for book_row in db_books_by_series.get(sid, []):
            bid = book_row["ID"]
            result.deleted_book_ids.append(bid)
            if bid in db_book_thumbs:
                result.deleted_book_thumb_book_ids.append(bid)

    # ── Existing series (in both) ───────────────────────────────────────
    existing_urls = disk_urls & db_urls
    for url in existing_urls:
        disk_s = disk_series[url]
        db_s = db_series[url]
        series_id = db_s["ID"]
        db_sm = db_series_meta.get(series_id)

        disk_books = disk_s.get("books", [])
        disk_book_urls = {b["url"] for b in disk_books}
        existing_db_books = db_books_by_series.get(series_id, [])
        db_book_urls = {b["URL"] for b in existing_db_books}
        db_book_by_url = {b["URL"]: b for b in existing_db_books}

        # Update series counts
        book_count = len(disk_books)
        disk_file_last_modified = disk_s["file_last_modified"]

        if (book_count != (db_s.get("BOOK_COUNT") or 0) or
                disk_file_last_modified > db_s.get("FILE_LAST_MODIFIED", disk_file_last_modified)):
            result.series_count_updates.append({
                "series_id": series_id,
                "book_count": book_count,
                "file_last_modified": disk_file_last_modified,
            })

        # Update series metadata if series.json has data differing from DB
        disk_title = disk_s.get("title", disk_s["name"])
        disk_status = disk_s.get("status", "ONGOING")
        disk_summary = disk_s.get("summary", "")
        if disk_title != disk_s["name"]:
            db_title = db_sm.get("TITLE", "") if db_sm is not None and db_sm.get("TITLE") is not None else ""
            db_status = db_sm.get("STATUS", "ONGOING") if db_sm is not None and db_sm.get("STATUS") is not None else "ONGOING"
            db_summary = db_sm.get("SUMMARY", "") if db_sm is not None and db_sm.get("SUMMARY") is not None else ""
            if (disk_title != db_title or disk_status != db_status or disk_summary != db_summary):
                result.series_metadata_updates.append({
                    "series_id": series_id,
                    "title": disk_title,
                    "title_sort": disk_s.get("title_sort", disk_title),
                    "status": disk_status,
                    "summary": disk_summary,
                })

        # Series thumbnail
        disk_series_thumb = disk_s.get("series_thumbnail")
        db_series_thumb = db_series_thumbs.get(series_id)
        if disk_series_thumb:
            if db_series_thumb is None:
                result.new_series_thumbnails.append({
                    **disk_series_thumb,
                    "series_id": series_id,
                })
            elif disk_series_thumb["url"] != db_series_thumb.get("URL", ""):
                result.series_thumbnail_updates.append({
                    "series_id": series_id,
                    "url": disk_series_thumb["url"],
                    "file_size": disk_series_thumb.get("file_size", 0),
                })
        elif db_series_thumb:
            result.deleted_series_thumb_series_ids.append(series_id)

        # New books within existing series
        new_book_urls = disk_book_urls - db_book_urls
        for book in disk_books:
            if book["url"] in new_book_urls:
                # Check if previously soft-deleted
                if book["url"] in db_soft_deleted_books:
                    result.reactivate_book_ids.append(db_soft_deleted_books[book["url"]]["ID"])
                    if "book_thumbnail" in book:
                        result.new_book_thumbnails.append({
                            **book["book_thumbnail"],
                            "book_id": db_soft_deleted_books[book["url"]]["ID"],
                        })
                    continue
                book_idx = _book_counter[0]
                _book_counter[0] += 1
                book["series_id"] = series_id
                book["_book_idx"] = book_idx
                result.new_books.append(book)
                if "book_thumbnail" in book:
                    result.new_book_thumbnails.append({
                        **book["book_thumbnail"],
                        "_book_idx": book_idx,
                    })

        # Deleted books from existing series
        deleted_book_urls = db_book_urls - disk_book_urls
        for url_b in deleted_book_urls:
            db_book_row = db_book_by_url[url_b]
            bid = db_book_row["ID"]
            result.deleted_book_ids.append(bid)
            if bid in db_book_thumbs:
                result.deleted_book_thumb_book_ids.append(bid)

        # Book thumbnails for existing books
        for book in disk_books:
            if book["url"] in db_book_urls and "book_thumbnail" in book:
                book_row = db_book_by_url[book["url"]]
                bid = book_row["ID"]
                disk_thumb = book["book_thumbnail"]
                db_thumb = db_book_thumbs.get(bid)
                if db_thumb is None:
                    result.new_book_thumbnails.append({**disk_thumb, "book_id": bid})
                elif disk_thumb["url"] != db_thumb.get("URL", ""):
                    result.book_thumbnail_updates.append({
                        "book_id": bid,
                        "url": disk_thumb["url"],
                        "file_size": disk_thumb.get("file_size", 0),
                    })

    return result
