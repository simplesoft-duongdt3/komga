"""Filesystem walker — scans library root for .pdf files only.
   Supports parallel directory scanning via ThreadPoolExecutor.
   Optional XXH3_128 hash cache to skip inline hash computation."""

import json
import os
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path

try:
    import xxhash
    _has_xxhash = True
except ImportError:
    _has_xxhash = False

_HASH_CACHE: dict[str, str] | None = None


def load_hash_cache(path: str | None = None) -> int:
    """Load the Rust-generated XXH3_128 hash cache.

    Pass path directly. No-op if file doesn't exist. The env var HASH_CACHE_DIR
    is used only as a fallback when path is None (for CLI usage).
    Returns number of entries loaded (0 if cache not found or corrupted).
    """
    global _HASH_CACHE
    if path is None:
        path = os.environ.get("HASH_CACHE_DIR", "")
    if not path or not os.path.isfile(path):
        return 0
    try:
        with open(path) as f:
            data = json.load(f)
        entries = data.get("entries", {})
        _HASH_CACHE = {uri: e["hash"] for uri, e in entries.items()}
        print(f"[walker] Loaded hash cache: {len(_HASH_CACHE)} entries from {path}")
        return len(_HASH_CACHE)
    except Exception as e:
        print(f"[walker] Failed to load hash cache from {path}: {e}")
        return 0


def _mtime(path: Path) -> str:
    return datetime.fromtimestamp(
        path.stat().st_mtime, tz=timezone.utc
    ).isoformat()


def _hash(path: Path) -> str:
    if _HASH_CACHE is not None:
        uri = path.as_uri()
        if uri in _HASH_CACHE:
            return _HASH_CACHE[uri]
    if not _has_xxhash:
        raise RuntimeError("xxhash package required for file hashing: pip install xxhash")
    h = xxhash.xxh3_128(seed=0)
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def _walk_one_dir(entry: Path, hash_files: bool) -> tuple[str, dict | None]:
    """Process one series directory. Thread-safe — no shared mutable state."""
    try:
        books = []
        for f in sorted(entry.rglob("*"), key=lambda e: e.name.lower()):
            if f.is_file() and f.suffix.lower() == ".pdf":
                book = {
                    "url": f.as_uri(),
                    "name": f.stem,
                    "file_size": f.stat().st_size,
                    "file_last_modified": _mtime(f),
                }
                if hash_files:
                    book["file_hash"] = _hash(f)
                books.append(book)
        if not books:
            return (entry.as_uri(), None)
        return (entry.as_uri(), {
            "url": entry.as_uri(),
            "name": entry.name,
            "file_last_modified": _mtime(entry),
            "books": books,
        })
    except PermissionError as e:
        return (entry.as_uri(), None)


def walk_library(root: str, *, exclusions: set[str] | None = None,
                 oneshots_dir: str | None = None,
                 hash_files: bool = False,
                 max_workers: int | None = None,
                  on_progress: Callable | None = None) -> dict:
    """
    Walk a library root directory scanning for .pdf files only.

    Args:
        root: Filesystem path to the library root
        exclusions: Set of directory names to skip
        oneshots_dir: Subdirectory name for oneshots (e.g. "Oneshots")
        hash_files: If True, compute XXH3_128 hash for each PDF file
        max_workers: Number of parallel threads for directory scanning.
                     0 or None = auto (CPU count × 2). 1 = sequential.
        on_progress: Optional callback(completed, total) called after each
                     directory is processed.

    Returns:
    {
      "series": { "file:///root/SeriesA": { ... } },
      "oneshots": [...]
    }
    """
    if max_workers is None or max_workers == 0:
        max_workers = max((os.cpu_count() or 4) * 2, 1)

    root_path = Path(root).resolve()
    exclusions = exclusions or set()

    # Collect directories to scan
    dirs = [
        e for e in root_path.iterdir()
        if e.is_dir() and e.name not in exclusions
    ]

    # Parallel walk directories
    series: dict[str, dict] = {}
    if dirs:
        with ThreadPoolExecutor(max_workers=max_workers) as pool:
            futures = {
                pool.submit(_walk_one_dir, d, hash_files): d
                for d in dirs
            }
            completed = 0
            for future in as_completed(futures):
                completed += 1
                if on_progress:
                    on_progress(completed, len(dirs), futures[future].name)
                series_url, series_data = future.result()
                if series_data:
                    series[series_url] = series_data

    # Oneshots at root level (PDF only)
    oneshots: list[dict] = []
    for entry in sorted(root_path.iterdir(), key=lambda e: e.name.lower()):
        if entry.is_file() and entry.suffix.lower() == ".pdf":
            oneshot = {
                "url": entry.as_uri(),
                "name": entry.stem,
                "file_size": entry.stat().st_size,
                "file_last_modified": _mtime(entry),
            }
            if hash_files:
                oneshot["file_hash"] = _hash(entry)
            oneshots.append(oneshot)

    # Oneshots in dedicated oneshots directory
    if oneshots_dir:
        oneshots_path = root_path / oneshots_dir
        if oneshots_path.is_dir():
            for f in sorted(oneshots_path.iterdir(), key=lambda e: e.name.lower()):
                if f.is_file() and f.suffix.lower() == ".pdf":
                    oneshot = {
                        "url": f.as_uri(),
                        "name": f.stem,
                        "file_size": f.stat().st_size,
                        "file_last_modified": _mtime(f),
                    }
                    if hash_files:
                        oneshot["file_hash"] = _hash(f)
                    oneshots.append(oneshot)

    return {"series": series, "oneshots": oneshots}
