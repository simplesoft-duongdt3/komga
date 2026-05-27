"""Diff engine — compares DB state vs filesystem state."""

import re
from dataclasses import dataclass, field
from pathlib import PurePosixPath
from urllib.parse import unquote, urlparse


def _normalize_file_url(url: str) -> str:
    """Normalize file:// URLs for consistent comparison.

    Java URL.toString() may produce 'file:/path' (JDK 21+).
    Python Path.as_uri() produces 'file:///path'.
    Java Path.toUri() adds trailing slash for directories.

    Canonical form: 'file:///absolute/path' (no trailing slash).
    """
    if "file:" not in url:
        return url
    # strip scheme (file:, file://, file:///) and trailing slashes
    path = re.sub(r"^file:(//?)?", "", url).strip("/")
    return f"file:///{path}"


def _mtime_equals(fs_mtime: str | None, db_mtime: str | None) -> bool | None:
    """Compare mtime strings tolerating sub-second and timezone differences.

    Returns:
      True   — mtimes match (same second)
      False  — mtimes differ
      None   — one or both missing, can't determine
    """
    if not fs_mtime or not db_mtime:
        return None
    # Trim to second-level precision, strip timezone
    fs_trimmed = fs_mtime.split(".")[0].replace("Z", "").replace("z", "")
    db_trimmed = db_mtime.split(".")[0].replace("Z", "").replace("z", "")
    return fs_trimmed == db_trimmed


@dataclass
class Diff:
    new_series: list[dict] = field(default_factory=list)
    deleted_series: list[dict] = field(default_factory=list)
    new_books: list[dict] = field(default_factory=list)
    deleted_books: list[dict] = field(default_factory=list)
    changed_books: list[dict] = field(default_factory=list)
    pending_hash: list[dict] = field(default_factory=list)


def compute(series_from_db: list[dict], books_from_db: list[dict],
            fs_data: dict) -> Diff:
    """Compare DB state vs filesystem state."""

    # Normalize URLs for consistent key matching across Java (DB) and Python (FS)
    db_series_by_url = {_normalize_file_url(s["url"]): s for s in series_from_db}
    db_books_by_url = {_normalize_file_url(b["url"]): b for b in books_from_db}

    fs_series = fs_data["series"]
    fs_series_normalized = {}  # normalized url → original data
    for url, s in fs_series.items():
        fs_series_normalized[_normalize_file_url(url)] = s

    fs_books_by_url = {}
    for s in fs_series.values():
        for b in s["books"]:
            fs_books_by_url[_normalize_file_url(b["url"])] = b
    for oneshot in fs_data.get("oneshots", []):
        fs_books_by_url[_normalize_file_url(oneshot["url"])] = oneshot

    db_urls_s = set(db_series_by_url)
    fs_urls_s = set(fs_series_normalized)
    db_urls_b = set(db_books_by_url)
    fs_urls_b = set(fs_books_by_url)

    diff = Diff()

    # New series (directory exists on disk but not in DB)
    for url in fs_urls_s - db_urls_s:
        diff.new_series.append(fs_series_normalized[url])

    # Deleted series (in DB but directory not on disk)
    for url in db_urls_s - fs_urls_s:
        diff.deleted_series.append(db_series_by_url[url])

    # New books (book exists on disk but not in DB), excluding books
    # that belong to new series (handled together with series creation)
    new_series_urls = {_normalize_file_url(s["url"]) for s in diff.new_series}
    for url in fs_urls_b - db_urls_b:
        parent = _get_parent_series_url(url, fs_series_normalized)
        if parent in new_series_urls:
            continue
        diff.new_books.append(fs_books_by_url[url])

    # Deleted books (in DB but file not on disk)
    for url in db_urls_b - fs_urls_b:
        diff.deleted_books.append(db_books_by_url[url])

    # Changed books (same URL, different content)
    # Prefer hash when both sides have it; fall back to mtime/size
    for url in db_urls_b & fs_urls_b:
        fs_b = fs_books_by_url[url]
        db_b = db_books_by_url[url]

        changed = False

        fs_hash = fs_b.get("file_hash", "")
        db_hash = db_b.get("file_hash", "")

        if fs_hash and db_hash:
            changed = fs_hash != db_hash
        elif db_hash and not fs_hash:
            # DB has hash but FS doesn't (hash_files=False this run)
            changed = (_mtime_equals(fs_b["file_last_modified"], db_b.get("file_last_modified")) is False or
                       fs_b["file_size"] != db_b.get("file_size"))
        elif fs_hash and not db_hash:
            # FS has hash but DB doesn't (analyzer hasn't computed it yet)
            diff.pending_hash.append(dict(
                id=db_b["id"],
                name=db_b["name"],
                series_id=db_b.get("series_id"),
                url=url,
                file_hash=fs_hash,
            ))
        else:
            changed = (_mtime_equals(fs_b["file_last_modified"], db_b.get("file_last_modified")) is False or
                       fs_b["file_size"] != db_b.get("file_size"))

        if changed:
            diff.changed_books.append(dict(
                id=db_b["id"],
                name=db_b["name"],
                series_id=db_b.get("series_id"),
                url=url,
            ))

    return diff


def group_new_books_by_series(
    books: list[dict], fs_series: dict,
    db_series_by_url: dict,
) -> dict[str, list[dict]]:
    """Map new books to their parent series ID by matching the series URL."""
    result: dict[str, list[dict]] = {}
    fs_series_norm = {_normalize_file_url(u): o for u, o in fs_series.items()}
    db_series_norm = {_normalize_file_url(u): d for u, d in db_series_by_url.items()}
    for b in books:
        parent_url = _get_parent_series_url(b["url"], fs_series_norm)
        if parent_url and parent_url in db_series_norm:
            sid = db_series_norm[parent_url]["id"]
            result.setdefault(sid, []).append(b)
    return result


def _get_parent_series_url(book_url: str, fs_series: dict) -> str | None:
    """Find which series directory contains this book URL."""
    book_path = PurePosixPath(unquote(urlparse(book_url).path))
    for series_url in fs_series:
        series_path = PurePosixPath(unquote(urlparse(series_url).path))
        try:
            book_path.relative_to(series_path)
            return series_url
        except ValueError:
            continue
    return None
