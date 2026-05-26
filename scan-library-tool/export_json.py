"""Exports J1 (DB snapshot) and J2 (FS snapshot) as JSON files."""

import json
import os
from datetime import datetime, timezone


EXPORT_DIR = os.environ.get("EXPORT_DIR", "/exports")


def export_snapshots(library_id: str, library_name: str, library_root: str,
                     db_series: list[dict], db_books: list[dict],
                     fs_data: dict):
    """Write J1 (DB state) and J2 (filesystem state) as pretty-printed JSON."""

    os.makedirs(EXPORT_DIR, exist_ok=True)
    ts = datetime.now(tz=timezone.utc).strftime("%Y%m%d-%H%M%S")
    safe_name = library_name.replace(" ", "_").replace("/", "_")

    j1 = {
        "type": "J1 — DB Snapshot",
        "library": {"id": library_id, "name": library_name, "root": library_root},
        "exported_at": ts,
        "series_count": len(db_series),
        "books_count": len(db_books),
        "series": db_series,
        "books": db_books,
    }

    j1_path = os.path.join(EXPORT_DIR, f"{safe_name}_J1_db_{ts}.json")
    with open(j1_path, "w") as f:
        json.dump(j1, f, indent=2, default=str)

    # Convert FS data to a serializable structure
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
        "library": {"root": library_root},
        "exported_at": ts,
        "series_count": len(fs_series_list),
        "books_count": sum(len(s["books"]) for s in fs_series_list) + len(fs_data.get("oneshots", [])),
        "series": fs_series_list,
        "oneshots": fs_data.get("oneshots", []),
    }

    j2_path = os.path.join(EXPORT_DIR, f"{safe_name}_J2_fs_{ts}.json")
    with open(j2_path, "w") as f:
        json.dump(j2, f, indent=2, default=str)

    return j1_path, j2_path
