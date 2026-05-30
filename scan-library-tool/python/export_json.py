"""Exports J1 (DB snapshot), J2 (FS snapshot), and J3 (diff) as JSON files."""

import json
import os
from datetime import datetime, timezone

from config import EXPORT_DIR


def export_snapshots(library_id: str, library_name: str, library_root: str,
                     db_series: list[dict], db_books: list[dict],
                     fs_data: dict, request_id: str) -> dict[str, str]:
    """Write J1 (DB), J2 (FS), J3 (diff-ready data) into request_id subfolder.

    Returns: {db, fs} file paths.
    """
    folder = os.path.join(EXPORT_DIR, request_id)
    os.makedirs(folder, exist_ok=True)

    j1 = {
        "type": "J1 — DB Snapshot",
        "request_id": request_id,
        "library": {"id": library_id, "name": library_name, "root": library_root},
        "exported_at": datetime.now(tz=timezone.utc).isoformat(),
        "series_count": len(db_series),
        "books_count": len(db_books),
        "series": db_series,
        "books": db_books,
    }

    j1_path = os.path.join(folder, f"{request_id}_db.json")
    with open(j1_path, "w") as f:
        json.dump(j1, f, indent=2, default=str)

    fs_series_list = []
    for url, s in fs_data.get("series", {}).items():
        fs_series_list.append({
            "url": s["url"],
            "name": s["name"],
            "file_last_modified": s["file_last_modified"],
            "books": s["books"],
        })

    j2 = {
        "type": "J2 — Filesystem Snapshot",
        "request_id": request_id,
        "library": {"root": library_root},
        "exported_at": datetime.now(tz=timezone.utc).isoformat(),
        "series_count": len(fs_series_list),
        "books_count": sum(len(s["books"]) for s in fs_series_list) + len(fs_data.get("oneshots", [])),
        "series": fs_series_list,
        "oneshots": fs_data.get("oneshots", []),
    }

    j2_path = os.path.join(folder, f"{request_id}_fs.json")
    with open(j2_path, "w") as f:
        json.dump(j2, f, indent=2, default=str)

    return {"db": j1_path, "fs": j2_path}
