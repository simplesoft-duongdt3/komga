import datetime
from typing import Any

import psycopg2
import psycopg2.extras
import psycopg2.pool

from config import Config
from id_generator import generate_id

def _utcnow() -> datetime.datetime:
    return datetime.datetime.utcnow()


class KomgaDb:
    def __init__(self, config: Config):
        self._cfg = config
        user = config.db.write_user or config.db.user
        password = config.db.write_password or config.db.password
        self._pool = psycopg2.pool.ThreadedConnectionPool(
            minconn=config.db.min_connections,
            maxconn=config.db.max_connections,
            host=config.db.host,
            port=config.db.port,
            database=config.db.database,
            user=user,
            password=password,
        )

    def read_only_connection(self):
        conn = psycopg2.connect(
            host=self._cfg.db.host,
            port=self._cfg.db.port,
            database=self._cfg.db.database,
            user=self._cfg.db.user,
            password=self._cfg.db.password,
        )
        conn.set_session(readonly=True, autocommit=True)
        return conn

    def get_conn(self):
        return self._pool.getconn()

    def put_conn(self, conn):
        self._pool.putconn(conn)

    def close(self):
        self._pool.closeall()

    # ── READ operations (read-only connection) ────────────────────────────

    def fetch_library(self) -> dict[str, Any] | None:
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    'SELECT * FROM "LIBRARY" WHERE "ID" = %s',
                    (self._cfg.library.library_id,),
                )
                return cur.fetchone()
        finally:
            conn.close()

    def fetch_existing_series(self) -> dict[str, dict[str, Any]]:
        """Return dict of {docker_url: series_row} for non-deleted series."""
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT * FROM "SERIES"
                    WHERE "LIBRARY_ID" = %s AND "DELETED_DATE" IS NULL
                    """,
                    (self._cfg.library.library_id,),
                )
                rows = cur.fetchall()
                return {r["URL"]: dict(r) for r in rows}
        finally:
            conn.close()

    def fetch_existing_books(self) -> dict[str, dict[str, Any]]:
        """Return dict of {docker_url: book_row} for non-deleted books."""
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT * FROM "BOOK"
                    WHERE "LIBRARY_ID" = %s AND "DELETED_DATE" IS NULL
                    """,
                    (self._cfg.library.library_id,),
                )
                rows = cur.fetchall()
                return {r["URL"]: dict(r) for r in rows}
        finally:
            conn.close()

    def fetch_soft_deleted_books(self) -> dict[str, dict[str, Any]]:
        """Return dict of {docker_url: book_row} for soft-deleted books in this library."""
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT * FROM "BOOK"
                    WHERE "LIBRARY_ID" = %s AND "DELETED_DATE" IS NOT NULL
                    """,
                    (self._cfg.library.library_id,),
                )
                rows = cur.fetchall()
                return {r["URL"]: dict(r) for r in rows}
        finally:
            conn.close()

    def reactivate_books(self, book_ids: list[str]) -> None:
        """Set DELETED_DATE = NULL for the given book IDs, re-activating them."""
        if not book_ids:
            return
        conn = self.get_conn()
        try:
            with conn.cursor() as cur:
                cur.execute(
                    """
                    UPDATE "BOOK" SET "DELETED_DATE" = NULL, "LAST_MODIFIED_DATE" = %s
                    WHERE "ID" = ANY(%s)
                    """,
                    (_utcnow(), book_ids),
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def fetch_existing_series_metadata(self) -> dict[str, dict[str, Any]]:
        """Return dict of {series_id: series_metadata_row}."""
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT sm.* FROM "SERIES_METADATA" sm
                    JOIN "SERIES" s ON s."ID" = sm."SERIES_ID"
                    WHERE s."LIBRARY_ID" = %s AND s."DELETED_DATE" IS NULL
                    """,
                    (self._cfg.library.library_id,),
                )
                rows = cur.fetchall()
                return {r["SERIES_ID"]: dict(r) for r in rows}
        finally:
            conn.close()

    def fetch_existing_series_thumbnails(self) -> dict[str, dict[str, Any]]:
        """Return dict of {series_id: thumbnail_row} for SIDECAR series thumbnails."""
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT ts.* FROM "THUMBNAIL_SERIES" ts
                    JOIN "SERIES" s ON s."ID" = ts."SERIES_ID"
                    WHERE s."LIBRARY_ID" = %s
                      AND s."DELETED_DATE" IS NULL
                      AND ts."TYPE" = 'SIDECAR'
                      AND ts."SELECTED" = true
                    """,
                    (self._cfg.library.library_id,),
                )
                rows = cur.fetchall()
                return {r["SERIES_ID"]: dict(r) for r in rows}
        finally:
            conn.close()

    def fetch_existing_book_thumbnails(self) -> dict[str, dict[str, Any]]:
        """Return dict of {book_id: thumbnail_row} for SIDECAR book thumbnails."""
        conn = self.read_only_connection()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                cur.execute(
                    """
                    SELECT tb.* FROM "THUMBNAIL_BOOK" tb
                    JOIN "BOOK" b ON b."ID" = tb."BOOK_ID"
                    WHERE b."LIBRARY_ID" = %s
                      AND b."DELETED_DATE" IS NULL
                      AND tb."TYPE" = 'SIDECAR'
                      AND tb."SELECTED" = true
                    """,
                    (self._cfg.library.library_id,),
                )
                rows = cur.fetchall()
                return {r["BOOK_ID"]: dict(r) for r in rows}
        finally:
            conn.close()

    # ── WRITE operations ──────────────────────────────────────────────────

    def insert_series_batch(self, series_list: list[dict]) -> list[str]:
        """Insert new SERIES + SERIES_METADATA rows. Returns list of series IDs."""
        conn = self.get_conn()
        try:
            now = _utcnow()
            series_ids: list[str] = []
            series_rows: list[tuple] = []
            metadata_rows: list[tuple] = []

            for s in series_list:
                sid = generate_id()
                series_ids.append(sid)
                series_rows.append((
                    sid,
                    now,
                    now,
                    s["file_last_modified"],
                    s["name"],
                    s["url"],
                    s["library_id"],
                    s.get("book_count", 0),
                ))
                metadata_rows.append((
                    now,
                    now,
                    s.get("status", "ONGOING"),
                    False,  # status_lock
                    s.get("title", s["name"]),
                    False,  # title_lock
                    s.get("title_sort", s.get("title", s["name"])),
                    False,  # title_sort_lock
                    sid,
                    s.get("publisher", ""),
                    False,  # publisher_lock
                    s.get("reading_direction"),
                    False,  # reading_direction_lock
                    s.get("age_rating"),
                    False,  # age_rating_lock
                    s.get("summary", ""),
                    False,  # summary_lock
                    s.get("language", ""),
                    False,  # language_lock
                    False,  # genres_lock
                    False,  # tags_lock
                    s.get("total_book_count"),
                    False,  # total_book_count_lock
                    False,  # sharing_labels_lock
                    False,  # links_lock
                    False,  # alternate_titles_lock
                ))

            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "SERIES" (
                        "ID", "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "FILE_LAST_MODIFIED", "NAME", "URL", "LIBRARY_ID", "BOOK_COUNT"
                    ) VALUES %s
                    """,
                    series_rows,
                    page_size=len(series_rows),
                )
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "SERIES_METADATA" (
                        "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "STATUS", "STATUS_LOCK",
                        "TITLE", "TITLE_LOCK",
                        "TITLE_SORT", "TITLE_SORT_LOCK",
                        "SERIES_ID",
                        "PUBLISHER", "PUBLISHER_LOCK",
                        "READING_DIRECTION", "READING_DIRECTION_LOCK",
                        "AGE_RATING", "AGE_RATING_LOCK",
                        "SUMMARY", "SUMMARY_LOCK",
                        "LANGUAGE", "LANGUAGE_LOCK",
                        "GENRES_LOCK", "TAGS_LOCK",
                        "TOTAL_BOOK_COUNT", "TOTAL_BOOK_COUNT_LOCK",
                        "SHARING_LABELS_LOCK", "LINKS_LOCK", "ALTERNATE_TITLES_LOCK"
                    ) VALUES %s
                    """,
                    metadata_rows,
                    page_size=len(metadata_rows),
                )
            conn.commit()
            return series_ids
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def insert_books_batch(self, books: list[dict]) -> list[str]:
        """Insert new BOOK + BOOK_METADATA + MEDIA rows. Returns list of book IDs."""
        conn = self.get_conn()
        try:
            now = _utcnow()
            book_ids: list[str] = []
            book_rows: list[tuple] = []
            metadata_rows: list[tuple] = []
            media_rows: list[tuple] = []

            for b in books:
                bid = generate_id()
                book_ids.append(bid)
                book_rows.append((
                    bid,
                    now,
                    now,
                    b["file_last_modified"],
                    b["name"],
                    b["url"],
                    b["series_id"],
                    b["file_size"],
                    b.get("number", 0),
                    b["library_id"],
                    b.get("file_hash", ""),
                ))
                number_s = b.get("number_str", str(b.get("number", 0)))
                number_sort = b.get("number_sort", float(b.get("number", 0)))
                metadata_rows.append((
                    now,
                    now,
                    number_s,
                    False,
                    number_sort,
                    False,
                    None,
                    False,
                    "",
                    False,
                    b.get("title", b["name"]),
                    False,
                    False,
                    False,
                    bid,
                    "",
                    False,
                    False,
                ))
                media_rows.append((
                    None,
                    "UNKNOWN",
                    now,
                    now,
                    None,
                    bid,
                    0,
                    None,
                    None,
                    False,
                    False,
                ))

            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "BOOK" (
                        "ID", "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "FILE_LAST_MODIFIED", "NAME", "URL", "SERIES_ID",
                        "FILE_SIZE", "NUMBER", "LIBRARY_ID", "FILE_HASH"
                    ) VALUES %s
                    """,
                    book_rows,
                    page_size=self._cfg.sync.commit_batch_size,
                )
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "BOOK_METADATA" (
                        "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "NUMBER", "NUMBER_LOCK",
                        "NUMBER_SORT", "NUMBER_SORT_LOCK",
                        "RELEASE_DATE", "RELEASE_DATE_LOCK",
                        "SUMMARY", "SUMMARY_LOCK",
                        "TITLE", "TITLE_LOCK",
                        "AUTHORS_LOCK", "TAGS_LOCK",
                        "BOOK_ID",
                        "ISBN", "ISBN_LOCK",
                        "LINKS_LOCK"
                    ) VALUES %s
                    """,
                    metadata_rows,
                    page_size=self._cfg.sync.commit_batch_size,
                )
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "MEDIA" (
                        "MEDIA_TYPE", "STATUS",
                        "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "COMMENT", "BOOK_ID", "PAGE_COUNT",
                        "EXTENSION_CLASS", "EXTENSION_VALUE_BLOB",
                        "EPUB_DIVINA_COMPATIBLE", "EPUB_IS_KEPUB"
                    ) VALUES %s
                    """,
                    media_rows,
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
            return book_ids
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def update_series_counts(self, updates: list[dict]) -> None:
        """Update BOOK_COUNT, FILE_LAST_MODIFIED, LAST_MODIFIED_DATE for existing series."""
        if not updates:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            now_str = now.strftime("%Y-%m-%d %H:%M:%S.%f")
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    f"""
                    UPDATE "SERIES" SET
                        "BOOK_COUNT" = data."book_count",
                        "FILE_LAST_MODIFIED" = data."file_last_modified",
                        "LAST_MODIFIED_DATE" = '{now_str}'
                    FROM (VALUES %s) AS data("series_id", "book_count", "file_last_modified")
                    WHERE "SERIES"."ID" = data."series_id"
                    """,
                    [(d["series_id"], d["book_count"], d["file_last_modified"]) for d in updates],
                    template="(%s, %s::integer, %s::timestamp)",
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def update_series_metadata(self, updates: list[dict]) -> None:
        """Update SERIES_METADATA for existing series where series.json changed."""
        if not updates:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            now_str = now.strftime("%Y-%m-%d %H:%M:%S.%f")
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    f"""
                    UPDATE "SERIES_METADATA" SET
                        "TITLE" = data."title",
                        "TITLE_SORT" = data."title_sort",
                        "STATUS" = data."status",
                        "SUMMARY" = data."summary",
                        "LAST_MODIFIED_DATE" = '{now_str}'
                    FROM (VALUES %s) AS data("series_id", "title", "title_sort", "status", "summary")
                    WHERE "SERIES_METADATA"."SERIES_ID" = data."series_id"
                    """,
                    [(d["series_id"], d["title"], d.get("title_sort", d["title"]), d["status"], d.get("summary", "")) for d in updates],
                    template="(%s, %s, %s, %s, %s)",
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def soft_delete_series(self, series_ids: list[str]) -> None:
        """Mark series (and their books cascading) as deleted."""
        if not series_ids:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            with conn.cursor() as cur:
                cur.execute(
                    """
                    UPDATE "BOOK" SET "DELETED_DATE" = %s
                    WHERE "SERIES_ID" = ANY(%s) AND "DELETED_DATE" IS NULL
                    """,
                    (now, series_ids),
                )
                cur.execute(
                    """
                    UPDATE "SERIES" SET "DELETED_DATE" = %s
                    WHERE "ID" = ANY(%s) AND "DELETED_DATE" IS NULL
                    """,
                    (now, series_ids),
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def soft_delete_books(self, book_ids: list[str]) -> None:
        """Mark books as deleted."""
        if not book_ids:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            with conn.cursor() as cur:
                cur.execute(
                    """
                    UPDATE "BOOK" SET "DELETED_DATE" = %s
                    WHERE "ID" = ANY(%s) AND "DELETED_DATE" IS NULL
                    """,
                    (now, book_ids),
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def insert_series_thumbnails(self, thumbnails: list[dict]) -> None:
        if not thumbnails:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            rows = []
            for t in thumbnails:
                tid = generate_id()
                rows.append((
                    tid,
                    t["series_id"],
                    None,  # THUMBNAIL bytea (SIDECAR has no blob)
                    t["url"],
                    True,  # SELECTED
                    "SIDECAR",
                    now,
                    now,
                    t.get("width", 0),
                    t.get("height", 0),
                    t.get("media_type", "image/jpeg"),
                    t.get("file_size", 0),
                ))
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "THUMBNAIL_SERIES" (
                        "ID", "SERIES_ID", "THUMBNAIL", "URL",
                        "SELECTED", "TYPE",
                        "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "WIDTH", "HEIGHT", "MEDIA_TYPE", "FILE_SIZE"
                    ) VALUES %s
                    """,
                    rows,
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def insert_book_thumbnails(self, thumbnails: list[dict]) -> None:
        if not thumbnails:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            rows = []
            for t in thumbnails:
                tid = generate_id()
                rows.append((
                    tid,
                    t["book_id"],
                    None,  # THUMBNAIL bytea
                    t["url"],
                    True,  # SELECTED
                    "SIDECAR",
                    now,
                    now,
                    t.get("width", 0),
                    t.get("height", 0),
                    t.get("media_type", "image/jpeg"),
                    t.get("file_size", 0),
                ))
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "THUMBNAIL_BOOK" (
                        "ID", "BOOK_ID", "THUMBNAIL", "URL",
                        "SELECTED", "TYPE",
                        "CREATED_DATE", "LAST_MODIFIED_DATE",
                        "WIDTH", "HEIGHT", "MEDIA_TYPE", "FILE_SIZE"
                    ) VALUES %s
                    """,
                    rows,
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def update_series_thumbnails(self, thumbnails: list[dict]) -> None:
        """Update the URL of existing SIDECAR series thumbnails."""
        if not thumbnails:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            now_str = now.strftime("%Y-%m-%d %H:%M:%S.%f")
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    f"""
                    UPDATE "THUMBNAIL_SERIES" SET
                        "URL" = data."url",
                        "LAST_MODIFIED_DATE" = '{now_str}',
                        "FILE_SIZE" = data."file_size"
                    FROM (VALUES %s) AS data("series_id", "url", "file_size")
                    WHERE "THUMBNAIL_SERIES"."SERIES_ID" = data."series_id"
                      AND "THUMBNAIL_SERIES"."TYPE" = 'SIDECAR'
                      AND "THUMBNAIL_SERIES"."SELECTED" = true
                    """,
                    [(t["series_id"], t["url"], t.get("file_size", 0)) for t in thumbnails],
                    template="(%s, %s, %s::bigint)",
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def update_book_thumbnails(self, thumbnails: list[dict]) -> None:
        if not thumbnails:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            now_str = now.strftime("%Y-%m-%d %H:%M:%S.%f")
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    f"""
                    UPDATE "THUMBNAIL_BOOK" SET
                        "URL" = data."url",
                        "LAST_MODIFIED_DATE" = '{now_str}',
                        "FILE_SIZE" = data."file_size"
                    FROM (VALUES %s) AS data("book_id", "url", "file_size")
                    WHERE "THUMBNAIL_BOOK"."BOOK_ID" = data."book_id"
                      AND "THUMBNAIL_BOOK"."TYPE" = 'SIDECAR'
                      AND "THUMBNAIL_BOOK"."SELECTED" = true
                    """,
                    [(t["book_id"], t["url"], t.get("file_size", 0)) for t in thumbnails],
                    template="(%s, %s, %s::bigint)",
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def delete_series_thumbnails_for_series(self, series_ids: list[str]) -> None:
        """Delete SIDECAR thumbnails for given series IDs."""
        if not series_ids:
            return
        conn = self.get_conn()
        try:
            with conn.cursor() as cur:
                cur.execute(
                    """
                    DELETE FROM "THUMBNAIL_SERIES"
                    WHERE "SERIES_ID" = ANY(%s) AND "TYPE" = 'SIDECAR'
                    """,
                    (series_ids,),
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def delete_book_thumbnails_for_books(self, book_ids: list[str]) -> None:
        if not book_ids:
            return
        conn = self.get_conn()
        try:
            with conn.cursor() as cur:
                cur.execute(
                    """
                    DELETE FROM "THUMBNAIL_BOOK"
                    WHERE "BOOK_ID" = ANY(%s) AND "TYPE" = 'SIDECAR'
                    """,
                    (book_ids,),
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    # ── ANALYSIS operations ──────────────────────────────────────────────

    def fetch_unanalyzed_books(self, limit: int | None = None) -> list[dict[str, Any]]:
        """Return books with MEDIA.STATUS='UNKNOWN'. Returns [{ID, URL}, ...]."""
        conn = self.get_conn()
        try:
            with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
                sql = """
                    SELECT b."ID", b."URL"
                    FROM "BOOK" b
                    JOIN "MEDIA" m ON m."BOOK_ID" = b."ID"
                    WHERE b."LIBRARY_ID" = %s
                      AND m."STATUS" = 'UNKNOWN'
                      AND b."DELETED_DATE" IS NULL
                    ORDER BY b."CREATED_DATE"
                """
                params: list[Any] = [self._cfg.library.library_id]
                if limit is not None:
                    sql += " LIMIT %s"
                    params.append(limit)
                cur.execute(sql, params)
                return [dict(r) for r in cur.fetchall()]
        finally:
            self.put_conn(conn)

    def update_media_analyzed(self, updates: list[dict]) -> None:
        """Batch UPDATE MEDIA: STATUS, PAGE_COUNT, MEDIA_TYPE, LAST_MODIFIED_DATE, COMMENT."""
        if not updates:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            now_str = now.strftime("%Y-%m-%d %H:%M:%S.%f")
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    f"""
                    UPDATE "MEDIA" SET
                        "STATUS" = data."status",
                        "PAGE_COUNT" = data."page_count",
                        "MEDIA_TYPE" = data."media_type",
                        "LAST_MODIFIED_DATE" = '{now_str}',
                        "COMMENT" = data."comment"
                    FROM (VALUES %s) AS data("book_id", "status", "page_count", "media_type", "comment")
                    WHERE "MEDIA"."BOOK_ID" = data."book_id"
                    """,
                    [(u["book_id"], u["status"], u["page_count"], u["media_type"], u.get("comment")) for u in updates],
                    template="(%s, %s, %s::integer, %s, %s)",
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def insert_media_pages_batch(self, pages: list[dict]) -> None:
        """Batch INSERT/upsert MEDIA_PAGE rows. Uses ON CONFLICT DO UPDATE."""
        if not pages:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            rows = [
                (
                    p["book_id"],
                    p["number"],
                    p["file_name"],
                    p.get("media_type", ""),
                    p.get("width", 0),
                    p.get("height", 0),
                    now,
                    now,
                )
                for p in pages
            ]
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    """
                    INSERT INTO "MEDIA_PAGE" (
                        "BOOK_ID", "NUMBER", "FILE_NAME", "MEDIA_TYPE",
                        "WIDTH", "HEIGHT", "CREATED_DATE", "LAST_MODIFIED_DATE"
                    ) VALUES %s
                    ON CONFLICT ("BOOK_ID", "NUMBER") DO UPDATE SET
                        "FILE_NAME" = EXCLUDED."FILE_NAME",
                        "MEDIA_TYPE" = EXCLUDED."MEDIA_TYPE",
                        "WIDTH" = EXCLUDED."WIDTH",
                        "HEIGHT" = EXCLUDED."HEIGHT",
                        "LAST_MODIFIED_DATE" = EXCLUDED."LAST_MODIFIED_DATE"
                    """,
                    rows,
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)

    def update_book_hashes(self, updates: list[dict]) -> None:
        """Batch UPDATE BOOK.FILE_HASH and LAST_MODIFIED_DATE."""
        if not updates:
            return
        conn = self.get_conn()
        try:
            now = _utcnow()
            now_str = now.strftime("%Y-%m-%d %H:%M:%S.%f")
            with conn.cursor() as cur:
                psycopg2.extras.execute_values(
                    cur,
                    f"""
                    UPDATE "BOOK" SET
                        "FILE_HASH" = data."file_hash",
                        "LAST_MODIFIED_DATE" = '{now_str}'
                    FROM (VALUES %s) AS data("book_id", "file_hash")
                    WHERE "BOOK"."ID" = data."book_id"
                    """,
                    [(u["book_id"], u["file_hash"]) for u in updates],
                    template="(%s, %s)",
                    page_size=self._cfg.sync.commit_batch_size,
                )
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            self.put_conn(conn)
