import json
import os
import re
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from typing import Any, Optional

from config import BOOK_EXTENSIONS, SERIES_THUMBNAIL_NAMES, Config


def _utcnow() -> datetime:
    return datetime.utcnow()


def _to_docker_path(real_path: str, real_root: str, docker_root: str) -> str:
    real_root = real_root.rstrip("/")
    docker_root = docker_root.rstrip("/")
    if real_path.startswith(real_root):
        return docker_root + real_path[len(real_root):]
    return real_path


def _natural_sort_key(name: str) -> tuple:
    """Split a string into text and number parts for natural sorting."""
    parts = re.split(r"(\d+)", name.lower())
    result = []
    for part in parts:
        if part.isdigit():
            result.append((0, int(part)))
        else:
            result.append((1, part))
    return tuple(result)


def _parse_series_json(filepath: str) -> dict[str, Any] | None:
    """Parse Mylar-style series.json. Returns metadata dict or None."""
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            data = json.load(f)
        if not isinstance(data, dict) or "metadata" not in data:
            return None
        meta = data["metadata"]
        if not isinstance(meta, dict):
            return None
        result: dict[str, Any] = {}
        if "name" in meta and meta["name"]:
            result["title"] = str(meta["name"])
        if "description_text" in meta and meta["description_text"]:
            result["summary"] = str(meta["description_text"])
        if "status" in meta and meta["status"]:
            status = str(meta["status"])
            if status == "Ended":
                result["status"] = "ENDED"
            elif status == "Continuing":
                result["status"] = "ONGOING"
            else:
                result["status"] = "ONGOING"
        if "publisher" in meta and meta["publisher"]:
            result["publisher"] = str(meta["publisher"])
        age_map = {"All": 0, "9+": 9, "12+": 12, "15+": 15, "17+": 17, "Adult": 18}
        if "age_rating" in meta and meta["age_rating"] in age_map:
            result["age_rating"] = age_map[meta["age_rating"]]
        if "total_issues" in meta and meta["total_issues"] is not None:
            try:
                result["total_book_count"] = int(meta["total_issues"])
            except (TypeError, ValueError):
                pass
        return result if result else None
    except (json.JSONDecodeError, OSError):
        return None


def _scan_single_series(
    dir_entry: os.DirEntry,
    real_root: str,
    docker_root: str,
    library_id: str,
) -> dict[str, Any] | None:
    """Scan a single series directory. Returns a series dict or None if skipped."""
    if not dir_entry.is_dir():
        return None
    dir_name = dir_entry.name
    if dir_name.startswith("."):
        return None

    dir_path = dir_entry.path
    series_url = _to_docker_path(dir_path, real_root, docker_root)

    # Collect book files (PDFs) and thumbnails
    book_files: list[dict] = []
    series_thumbnail: dict[str, Any] | None = None
    book_thumbnails_by_basename: dict[str, str] = {}  # book_basename -> jpg_path
    dir_mtime = 0.0
    series_json_path: str | None = None

    try:
        with os.scandir(dir_path) as entries:
            for entry in entries:
                if entry.is_dir():
                    continue
                fname = entry.name
                ext = os.path.splitext(fname)[1].lower()
                try:
                    stat = entry.stat()
                except OSError:
                    continue
                mtime_float = stat.st_mtime
                mtime_dt = datetime.utcfromtimestamp(mtime_float)
                if mtime_float > dir_mtime:
                    dir_mtime = mtime_float

                if ext in BOOK_EXTENSIONS:
                    basename = os.path.splitext(fname)[0]
                    book_files.append({
                        "name": basename,
                        "filename": fname,
                        "path": entry.path,
                        "file_size": stat.st_size,
                        "file_last_modified": mtime_dt,
                    })
                elif ext == ".jpg" or ext == ".jpeg":
                    basename_lower = fname.lower()
                    if basename_lower in SERIES_THUMBNAIL_NAMES:
                        if series_thumbnail is None:
                            series_thumbnail = {
                                "path": entry.path,
                                "file_size": stat.st_size,
                                "file_last_modified": mtime_float,
                                "media_type": "image/jpeg",
                            }
                    else:
                        # Check if it matches a book basename
                        # Pattern: <book_basename>[-<number>].jpg
                        jpg_basename = os.path.splitext(fname)[0]
                        book_thumbnails_by_basename[jpg_basename] = entry.path
                elif fname == "series.json":
                    series_json_path = entry.path
                    if stat.st_mtime > dir_mtime:
                        dir_mtime = stat.st_mtime
    except OSError:
        return None

    if not book_files:
        return None

    # Sort books and assign numbers
    book_files.sort(key=lambda b: _natural_sort_key(b["name"]))
    for i, b in enumerate(book_files):
        num = i + 1
        b["number"] = num
        b["number_str"] = str(num)
        b["number_sort"] = float(num)
        b["library_id"] = library_id
        b["url"] = _to_docker_path(b["path"], real_root, docker_root)
        b["title"] = b["name"]

    # Build series metadata from series.json
    metadata_from_json: dict[str, Any] = {}
    if series_json_path:
        metadata_from_json = _parse_series_json(series_json_path) or {}

    series_name = dir_name
    series_title = metadata_from_json.get("title", series_name)
    series_title_sort = series_title
    series_status = metadata_from_json.get("status", "ONGOING")
    series_summary = metadata_from_json.get("summary", "")
    series_publisher = metadata_from_json.get("publisher", "")
    series_age_rating = metadata_from_json.get("age_rating")
    series_total_book_count = metadata_from_json.get("total_book_count")

    file_last_modified = datetime.utcfromtimestamp(dir_mtime)
    book_count = len(book_files)

    result: dict[str, Any] = {
        "name": series_name,
        "url": series_url,
        "library_id": library_id,
        "file_last_modified": file_last_modified,
        "book_count": book_count,
        "title": series_title,
        "title_sort": series_title_sort,
        "status": series_status,
        "summary": series_summary,
        "publisher": series_publisher,
        "age_rating": series_age_rating,
        "total_book_count": series_total_book_count,
        "books": book_files,
    }

    # Add series thumbnail
    if series_thumbnail:
        result["series_thumbnail"] = {
            "path": series_thumbnail["path"],
            "url": _to_docker_path(series_thumbnail["path"], real_root, docker_root),
            "file_size": series_thumbnail["file_size"],
            "file_last_modified": series_thumbnail["file_last_modified"],
            "media_type": series_thumbnail["media_type"],
        }

    # Match book thumbnails
    for b in book_files:
        book_basename = b["name"]
        # Try exact match first
        thumb_path = book_thumbnails_by_basename.get(book_basename)
        if not thumb_path:
            # Try <basename>-<number>
            thumb_key = f"{book_basename}-{str(b['number']).zfill(4)}"
            thumb_path = book_thumbnails_by_basename.get(thumb_key)
        if thumb_path:
            try:
                thumb_stat = os.stat(thumb_path)
            except OSError:
                continue
            b["book_thumbnail"] = {
                "path": thumb_path,
                "url": _to_docker_path(thumb_path, real_root, docker_root),
                "file_size": thumb_stat.st_size,
                "file_last_modified": thumb_stat.st_mtime,
                "media_type": "image/jpeg",
            }

    return result


def scan_library(config: Config) -> dict[str, dict[str, Any]]:
    """Scan the library directory.

    Returns a dict of {docker_url: series_dict} for all series found on disk.
    """
    real_root = config.library.real_root_path
    docker_root = config.library.docker_root_path
    library_id = config.library.library_id
    max_workers = config.library.scan_workers or max(1, (os.cpu_count() or 4) * 2)
    max_workers = min(max_workers, 32)

    # Collect all subdirectory entries
    dir_entries: list[os.DirEntry] = []
    try:
        with os.scandir(real_root) as entries:
            for entry in entries:
                if entry.is_dir() and not entry.name.startswith("."):
                    dir_entries.append(entry)
    except OSError as e:
        raise RuntimeError(f"Cannot scan library root {real_root}: {e}") from e

    series_map: dict[str, dict[str, Any]] = {}

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = {
            executor.submit(_scan_single_series, entry, real_root, docker_root, library_id): entry.name
            for entry in dir_entries
        }
        for future in as_completed(futures):
            series_name = futures[future]
            try:
                result = future.result()
                if result and result.get("url"):
                    series_map[result["url"]] = result
            except Exception:
                pass

    return series_map


if __name__ == "__main__":
    cfg = Config()
    cfg.library.real_root_path = "/Users/teamcumahay/Downloads/ThienThaiTruyen"
    cfg.library.docker_root_path = "/data/data-books-audiobooks/Manga_Ebook/Manhwa"
    cfg.library.library_id = "TEST_LIBRARY"
    result = scan_library(cfg)
    print(f"Found {len(result)} series")
    for url, s in sorted(result.items()):
        print(f"  {s['title']}: {s['book_count']} books")
        if "series_thumbnail" in s:
            print(f"    Series thumbnail: {s['series_thumbnail']['url']}")
        for b in s["books"]:
            thumb = f" [thumb: {b['book_thumbnail']['url']}]" if "book_thumbnail" in b else ""
            print(f"    #{b['number']} {b['name']} ({b['file_size']} bytes){thumb}")
