"""Filesystem walker — scans library root for .pdf files only."""

import hashlib
from pathlib import Path
from datetime import datetime, timezone


def walk_library(root: str, *, exclusions: set[str] | None = None,
                 oneshots_dir: str | None = None,
                 hash_files: bool = False) -> dict:
    """
    Walk a library root directory scanning for .pdf files only.

    Args:
        hash_files: If True, compute SHA-256 hash for each PDF file.

    Returns:
    {
      "series": { "file:///root/SeriesA": {
          "url": "...", "name": "SeriesA",
          "file_last_modified": "...",
          "books": [{"url": "...", "name": "Ch01", "file_size": 1234,
                     "file_last_modified": "...", "file_hash": "abc123"}]}},
      "oneshots": [...]
    }
    """
    root_path = Path(root).resolve()
    exclusions = exclusions or set()

    def _mtime(path: Path) -> str:
        return datetime.fromtimestamp(
            path.stat().st_mtime, tz=timezone.utc
        ).isoformat()

    def _hash(path: Path) -> str:
        sha = hashlib.sha256()
        with open(path, "rb") as f:
            while chunk := f.read(65536):
                sha.update(chunk)
        return sha.hexdigest()

    series = {}
    oneshots = []

    for entry in sorted(root_path.iterdir(), key=lambda e: e.name.lower()):
        if entry.name in exclusions:
            continue

        if entry.is_dir():
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
            if books:
                series[entry.as_uri()] = {
                    "url": entry.as_uri(),
                    "name": entry.name,
                    "file_last_modified": _mtime(entry),
                    "books": books,
                }

    # Oneshots at root level (PDF only)
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
