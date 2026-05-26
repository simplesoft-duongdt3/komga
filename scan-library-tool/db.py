"""PostgreSQL reader — fetches active series and books for a library."""

import psycopg2
from config import PG_HOST, PG_PORT, PG_DB, PG_USER, PG_PASSWORD


def _conn():
    return psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD,
        options="-c default_transaction_read_only=on",
    )


def read_libraries() -> list[dict]:
    """Return all libraries with id, name, root."""
    with _conn() as conn, conn.cursor() as cur:
        cur.execute('SELECT "ID", "NAME", "ROOT" FROM "LIBRARY"')
        return [{"id": r[0], "name": r[1], "root": r[2]} for r in cur]


def read_series(library_id: str) -> list[dict]:
    """Return active (not soft-deleted) series for a library, keyed by URL."""
    with _conn() as conn, conn.cursor() as cur:
        cur.execute(
            'SELECT "ID", "NAME", "URL", "FILE_LAST_MODIFIED" '
            'FROM "SERIES" '
            'WHERE "LIBRARY_ID" = %s AND "DELETED_DATE" IS NULL',
            (library_id,),
        )
        return [
            {
                "id": r[0],
                "name": r[1],
                "url": r[2],
                "file_last_modified": r[3].isoformat() if r[3] else None,
            }
            for r in cur
        ]


def read_books(library_id: str) -> list[dict]:
    """Return active books for a library, keyed by URL."""
    with _conn() as conn, conn.cursor() as cur:
        cur.execute(
            'SELECT "ID", "NAME", "URL", "FILE_LAST_MODIFIED", "FILE_SIZE", '
            '"FILE_HASH", "SERIES_ID" '
            'FROM "BOOK" '
            'WHERE "LIBRARY_ID" = %s AND "DELETED_DATE" IS NULL',
            (library_id,),
        )
        return [
            {
                "id": r[0],
                "name": r[1],
                "url": r[2],
                "file_last_modified": r[3].isoformat() if r[3] else None,
                "file_size": r[4],
                "file_hash": r[5],
                "series_id": r[6],
            }
            for r in cur
        ]
