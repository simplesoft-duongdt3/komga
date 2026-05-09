"""Generate SQL statements from a DiffResult. Outputs valid PostgreSQL SQL to a file."""

from __future__ import annotations

import datetime
import logging
from typing import Any, TextIO

from syncer import DiffResult

logger = logging.getLogger(__name__)

_TS = "%Y-%m-%d %H:%M:%S.%f"

_SQL_BOOK_COLS = (
    '"ID"', '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"FILE_LAST_MODIFIED"', '"NAME"', '"URL"', '"SERIES_ID"',
    '"FILE_SIZE"', '"NUMBER"', '"LIBRARY_ID"', '"FILE_HASH"',
)

_SQL_BOOK_META_COLS = (
    '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"NUMBER"', '"NUMBER_LOCK"', '"NUMBER_SORT"', '"NUMBER_SORT_LOCK"',
    '"RELEASE_DATE"', '"RELEASE_DATE_LOCK"', '"SUMMARY"', '"SUMMARY_LOCK"',
    '"TITLE"', '"TITLE_LOCK"', '"AUTHORS_LOCK"', '"TAGS_LOCK"',
    '"BOOK_ID"', '"ISBN"', '"ISBN_LOCK"', '"LINKS_LOCK"',
)

_SQL_MEDIA_COLS = (
    '"MEDIA_TYPE"', '"STATUS"', '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"COMMENT"', '"BOOK_ID"', '"PAGE_COUNT"',
    '"EXTENSION_CLASS"', '"EXTENSION_VALUE_BLOB"',
    '"EPUB_DIVINA_COMPATIBLE"', '"EPUB_IS_KEPUB"',
)

_SQL_SERIES_COLS = (
    '"ID"', '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"FILE_LAST_MODIFIED"', '"NAME"', '"URL"', '"LIBRARY_ID"', '"BOOK_COUNT"',
)

_SQL_SERIES_META_COLS = (
    '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"STATUS"', '"STATUS_LOCK"',
    '"TITLE"', '"TITLE_LOCK"', '"TITLE_SORT"', '"TITLE_SORT_LOCK"',
    '"SERIES_ID"',
    '"PUBLISHER"', '"PUBLISHER_LOCK"',
    '"READING_DIRECTION"', '"READING_DIRECTION_LOCK"',
    '"AGE_RATING"', '"AGE_RATING_LOCK"',
    '"SUMMARY"', '"SUMMARY_LOCK"',
    '"LANGUAGE"', '"LANGUAGE_LOCK"',
    '"GENRES_LOCK"', '"TAGS_LOCK"',
    '"TOTAL_BOOK_COUNT"', '"TOTAL_BOOK_COUNT_LOCK"',
    '"SHARING_LABELS_LOCK"', '"LINKS_LOCK"', '"ALTERNATE_TITLES_LOCK"',
)

_SQL_SERIES_THUMB_COLS = (
    '"ID"', '"SERIES_ID"', '"THUMBNAIL"', '"URL"', '"SELECTED"', '"TYPE"',
    '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"WIDTH"', '"HEIGHT"', '"MEDIA_TYPE"', '"FILE_SIZE"',
)

_SQL_BOOK_THUMB_COLS = (
    '"ID"', '"BOOK_ID"', '"THUMBNAIL"', '"URL"', '"SELECTED"', '"TYPE"',
    '"CREATED_DATE"', '"LAST_MODIFIED_DATE"',
    '"WIDTH"', '"HEIGHT"', '"MEDIA_TYPE"', '"FILE_SIZE"',
)


def _esc(val: Any) -> str:
    """Escape a value for SQL. Returns NULL for None, quoted string for str/int/float/bool/bool."""
    if val is None:
        return "NULL"
    if isinstance(val, bool):
        return "true" if val else "false"
    if isinstance(val, (int, float)):
        return str(val)
    if isinstance(val, datetime.datetime):
        # PostgreSQL timestamp literal: 'YYYY-MM-DD HH:MM:SS.ffffff'
        return f"'{val.strftime(_TS)}'"
    if isinstance(val, datetime.date):
        return f"'{val.isoformat()}'"
    s = str(val).replace("'", "''")
    return f"'{s}'"


def _format_row(cols: tuple[str, ...], vals: tuple[Any, ...]) -> str:
    return f"({', '.join(_esc(v) for v in vals)})"


def _values_block(rows: list[tuple[Any, ...]]) -> str:
    lines = [_format_row((), r) for r in rows]
    return ",\n".join(lines)


def _write_insert(f: TextIO, table: str, cols: tuple[str, ...], rows: list[tuple[Any, ...]]) -> None:
    if not rows:
        return
    col_list = ", ".join(cols)
    f.write(f'INSERT INTO "{table}" ({col_list}) VALUES\n')
    f.write(_values_block(rows))
    f.write(";\n\n")


def _write_update(f: TextIO, sql: str) -> None:
    f.write(sql)
    f.write(";\n\n")


def _write_delete(f: TextIO, sql: str) -> None:
    f.write(sql)
    f.write(";\n\n")


def _generate_ids(count: int) -> list[str]:
    from id_generator import generate_id
    return [generate_id() for _ in range(count)]


def export_sql(f: TextIO, dr: DiffResult, logger: logging.Logger) -> None:
    now = datetime.datetime.utcnow()

    total_ops = (
        (1 if dr.new_series else 0) + (1 if dr.new_books else 0) +
        (1 if dr.deleted_series_ids else 0) + (1 if dr.deleted_book_ids else 0) +
        (1 if dr.series_count_updates else 0) + (1 if dr.series_metadata_updates else 0) +
        (1 if dr.new_series_thumbnails else 0) + (1 if dr.new_book_thumbnails else 0) +
        (1 if dr.series_thumbnail_updates else 0) + (1 if dr.book_thumbnail_updates else 0) +
        (1 if dr.deleted_series_thumb_series_ids else 0) + (1 if dr.deleted_book_thumb_book_ids else 0)
    )
    if total_ops == 0:
        logger.info("No changes to export. Database is in sync.")
        return

    logger.info("Exporting SQL...")

    f.write("-- Komga library scan diff — generated SQL\n")
    f.write(f"-- Generated at: {now}\n")
    f.write("BEGIN;\n\n")

    # ── 1. Deletes ─────────────────────────────────────────────────────

    if dr.reactivate_book_ids:
        ids = ", ".join(_esc(bid) for bid in dr.reactivate_book_ids)
        logger.info("  Exporting reactivate %d books...", len(dr.reactivate_book_ids))
        _write_update(f, f'UPDATE "BOOK" SET "DELETED_DATE" = NULL, "LAST_MODIFIED_DATE" = {_esc(now)} WHERE "ID" IN ({ids})')

    if dr.deleted_book_thumb_book_ids:
        ids = ", ".join(_esc(bid) for bid in dr.deleted_book_thumb_book_ids)
        logger.info("  Exporting delete %d book thumbnails...", len(dr.deleted_book_thumb_book_ids))
        _write_delete(f, f'DELETE FROM "THUMBNAIL_BOOK" WHERE "BOOK_ID" IN ({ids}) AND "TYPE" = \'SIDECAR\'')

    if dr.deleted_series_thumb_series_ids:
        ids = ", ".join(_esc(sid) for sid in dr.deleted_series_thumb_series_ids)
        logger.info("  Exporting delete %d series thumbnails...", len(dr.deleted_series_thumb_series_ids))
        _write_delete(f, f'DELETE FROM "THUMBNAIL_SERIES" WHERE "SERIES_ID" IN ({ids}) AND "TYPE" = \'SIDECAR\'')

    if dr.deleted_book_ids:
        ids = ", ".join(_esc(bid) for bid in dr.deleted_book_ids)
        logger.info("  Exporting soft-delete %d books...", len(dr.deleted_book_ids))
        _write_update(f, f'UPDATE "BOOK" SET "DELETED_DATE" = {_esc(now)} WHERE "ID" IN ({ids}) AND "DELETED_DATE" IS NULL')

    if dr.deleted_series_ids:
        ids = ", ".join(_esc(sid) for sid in dr.deleted_series_ids)
        logger.info("  Exporting soft-delete %d series...", len(dr.deleted_series_ids))
        _write_update(f, f'UPDATE "BOOK" SET "DELETED_DATE" = {_esc(now)} WHERE "SERIES_ID" IN ({ids}) AND "DELETED_DATE" IS NULL')
        _write_update(f, f'UPDATE "SERIES" SET "DELETED_DATE" = {_esc(now)} WHERE "ID" IN ({ids}) AND "DELETED_DATE" IS NULL')

    # ── 2. New series ──────────────────────────────────────────────────

    series_ids: list[str] = _generate_ids(len(dr.new_series))
    if dr.new_series:
        logger.info("  Exporting %d new series...", len(dr.new_series))
        s_rows: list[tuple[Any, ...]] = []
        sm_rows: list[tuple[Any, ...]] = []
        for i, s in enumerate(dr.new_series):
            sid = series_ids[i]
            s_rows.append((
                sid, now, now,
                s["file_last_modified"], s["name"], s["url"],
                s["library_id"], s.get("book_count", 0),
            ))
            sm_rows.append((
                now, now,
                s.get("status", "ONGOING"), False,
                s.get("title", s["name"]), False,
                s.get("title_sort", s.get("title", s["name"])), False,
                sid,
                s.get("publisher", ""), False,
                s.get("reading_direction"), False,
                s.get("age_rating"), False,
                s.get("summary", ""), False,
                s.get("language", ""), False,
                False, False,
                s.get("total_book_count"), False,
                False, False, False,
            ))
        _write_insert(f, "SERIES", _SQL_SERIES_COLS, s_rows)
        _write_insert(f, "SERIES_METADATA", _SQL_SERIES_META_COLS, sm_rows)

        # Assign series_id to new books
        for book in dr.new_books:
            if "_series_idx" in book and "series_id" not in book:
                book["series_id"] = series_ids[book["_series_idx"]]

        # Assign series_id to new series thumbnails
        for thumb in dr.new_series_thumbnails:
            if "_series_idx" in thumb:
                thumb["series_id"] = series_ids[thumb["_series_idx"]]

    # ── 3. New books ───────────────────────────────────────────────────

    all_new_books = [b for b in dr.new_books if "series_id" in b]
    book_ids: list[str] = _generate_ids(len(all_new_books))
    if all_new_books:
        logger.info("  Exporting %d new books...", len(all_new_books))
        b_rows: list[tuple[Any, ...]] = []
        bm_rows: list[tuple[Any, ...]] = []
        m_rows: list[tuple[Any, ...]] = []
        for i, b in enumerate(all_new_books):
            bid = book_ids[i]
            b_rows.append((
                bid, now, now,
                b["file_last_modified"], b["name"], b["url"],
                b["series_id"], b.get("file_size", 0),
                b.get("number", 0), b["library_id"], b.get("file_hash", ""),
            ))
            number_s = b.get("number_str", str(b.get("number", 0)))
            number_sort = b.get("number_sort", float(b.get("number", 0)))
            bm_rows.append((
                now, now,
                number_s, False, number_sort, False,
                None, False, "", False,
                b.get("title", b["name"]), False,
                False, False,
                bid, "", False, False,
            ))
            m_rows.append((
                None, "UNKNOWN", now, now,
                None, bid, 0, None, None, False, False,
            ))
        _write_insert(f, "BOOK", _SQL_BOOK_COLS, b_rows)
        _write_insert(f, "BOOK_METADATA", _SQL_BOOK_META_COLS, bm_rows)
        _write_insert(f, "MEDIA", _SQL_MEDIA_COLS, m_rows)

        # Assign book_id to new book thumbnails
        for thumb in dr.new_book_thumbnails:
            if "_book_idx" in thumb and "book_id" not in thumb:
                idx = thumb["_book_idx"]
                if idx < len(book_ids):
                    thumb["book_id"] = book_ids[idx]

    # ── 4. Thumbnails ──────────────────────────────────────────────────

    if dr.new_series_thumbnails:
        valid = [t for t in dr.new_series_thumbnails if "series_id" in t]
        if valid:
            logger.info("  Exporting %d new series thumbnails...", len(valid))
            series_thumb_ids = _generate_ids(len(valid))
            ts_rows: list[tuple[Any, ...]] = [
                (series_thumb_ids[i], t["series_id"], None, t["url"], True, "SIDECAR",
                 now, now, t.get("width", 0), t.get("height", 0),
                 t.get("media_type", "image/jpeg"), t.get("file_size", 0))
                for i, t in enumerate(valid)
            ]
            _write_insert(f, "THUMBNAIL_SERIES", _SQL_SERIES_THUMB_COLS, ts_rows)

    if dr.series_thumbnail_updates:
        logger.info("  Exporting %d series thumbnail updates...", len(dr.series_thumbnail_updates))
        for t in dr.series_thumbnail_updates:
            f.write(
                f'UPDATE "THUMBNAIL_SERIES" SET "URL" = {_esc(t["url"])}, '
                f'"LAST_MODIFIED_DATE" = {_esc(now)}, '
                f'"FILE_SIZE" = {_esc(t.get("file_size", 0))} '
                f'WHERE "SERIES_ID" = {_esc(t["series_id"])} AND "TYPE" = \'SIDECAR\' AND "SELECTED" = true;\n\n'
            )

    if dr.new_book_thumbnails:
        valid = [t for t in dr.new_book_thumbnails if t.get("book_id")]
        if valid:
            logger.info("  Exporting %d new book thumbnails...", len(valid))
            tb_ids = _generate_ids(len(valid))
            tb_rows: list[tuple[Any, ...]] = [
                (tb_ids[i], t.get("book_id"), None, t["url"], True, "SIDECAR",
                 now, now, t.get("width", 0), t.get("height", 0),
                 t.get("media_type", "image/jpeg"), t.get("file_size", 0))
                for i, t in enumerate(valid)
            ]
            _write_insert(f, "THUMBNAIL_BOOK", _SQL_BOOK_THUMB_COLS, tb_rows)

    if dr.book_thumbnail_updates:
        logger.info("  Exporting %d book thumbnail updates...", len(dr.book_thumbnail_updates))
        for t in dr.book_thumbnail_updates:
            f.write(
                f'UPDATE "THUMBNAIL_BOOK" SET "URL" = {_esc(t["url"])}, '
                f'"LAST_MODIFIED_DATE" = {_esc(now)}, '
                f'"FILE_SIZE" = {_esc(t.get("file_size", 0))} '
                f'WHERE "BOOK_ID" = {_esc(t["book_id"])} AND "TYPE" = \'SIDECAR\' AND "SELECTED" = true;\n\n'
            )

    # ── 5. Updates ─────────────────────────────────────────────────────

    if dr.series_count_updates:
        logger.info("  Exporting %d series count updates...", len(dr.series_count_updates))
        for u in dr.series_count_updates:
            f.write(
                f'UPDATE "SERIES" SET "BOOK_COUNT" = {_esc(u["book_count"])}, '
                f'"FILE_LAST_MODIFIED" = {_esc(u["file_last_modified"])}, '
                f'"LAST_MODIFIED_DATE" = {_esc(now)} '
                f'WHERE "ID" = {_esc(u["series_id"])};\n\n'
            )

    if dr.series_metadata_updates:
        logger.info("  Exporting %d series metadata updates...", len(dr.series_metadata_updates))
        for u in dr.series_metadata_updates:
            f.write(
                f'UPDATE "SERIES_METADATA" SET '
                f'"TITLE" = {_esc(u["title"])}, '
                f'"TITLE_SORT" = {_esc(u.get("title_sort", u["title"]))}, '
                f'"STATUS" = {_esc(u["status"])}, '
                f'"SUMMARY" = {_esc(u.get("summary", ""))}, '
                f'"LAST_MODIFIED_DATE" = {_esc(now)} '
                f'WHERE "SERIES_ID" = {_esc(u["series_id"])};\n\n'
            )

    f.write("COMMIT;\n")
    logger.info("SQL export complete.")
