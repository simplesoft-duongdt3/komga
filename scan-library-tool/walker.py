"""Filesystem walker — scans library root for .pdf files only."""

from pathlib import Path
from datetime import datetime, timezone


def walk_library(root: str, *, exclusions: set[str] | None = None,
                 oneshots_dir: str | None = None) -> dict:
    """
    Walk a library root directory scanning for .pdf files only.

    Returns:
    {
      "series": {
        "file:///root/SeriesA": {
          "url": "file:///root/SeriesA",
          "name": "SeriesA",
          "file_last_modified": "2026-01-01T00:00:00+00:00",
          "books": [
            {"url": "file:///...", "name": "Ch01", "file_size": 1234,
             "file_last_modified": "..."}
          ]
        }
      },
      "oneshots": [...]   # root-level .pdf files
    }
    """
    root_path = Path(root).resolve()
    exclusions = exclusions or set()

    def _mtime(path: Path) -> str:
        return datetime.fromtimestamp(
            path.stat().st_mtime, tz=timezone.utc
        ).isoformat()

    series = {}
    oneshots = []

    for entry in sorted(root_path.iterdir(), key=lambda e: e.name.lower()):
        if entry.name in exclusions:
            continue

        if entry.is_dir():
            books = []
            for f in sorted(entry.rglob("*"), key=lambda e: e.name.lower()):
                if f.is_file() and f.suffix.lower() == ".pdf":
                    books.append({
                        "url": f.as_uri(),
                        "name": f.stem,
                        "file_size": f.stat().st_size,
                        "file_last_modified": _mtime(f),
                    })
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
            oneshots.append({
                "url": entry.as_uri(),
                "name": entry.stem,
                "file_size": entry.stat().st_size,
                "file_last_modified": _mtime(entry),
            })

    # Oneshots in dedicated oneshots directory
    if oneshots_dir:
        oneshots_path = root_path / oneshots_dir
        if oneshots_path.is_dir():
            for f in sorted(oneshots_path.iterdir(), key=lambda e: e.name.lower()):
                if f.is_file() and f.suffix.lower() == ".pdf":
                    oneshots.append({
                        "url": f.as_uri(),
                        "name": f.stem,
                        "file_size": f.stat().st_size,
                        "file_last_modified": _mtime(f),
                    })

    return {"series": series, "oneshots": oneshots}
