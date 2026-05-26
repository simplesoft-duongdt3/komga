"""FuncToWeb server — type-hinted functions get auto-generated web UI."""

import json
import os
from typing import Annotated

from func_to_web import run, ActionTable, HiddenFunction
from func_to_web.types import Label

import api
import config
import db as database
import walker
import diff as differ
import export_json


# ── Shared helpers ──────────────────────────────────────────

def _resolve_library(library_id: str) -> dict:
    libs = api.list_libraries()
    lib = next((l for l in libs if l["id"] == library_id), None)
    if not lib:
        raise ValueError(f"Library not found: {library_id}")
    root = lib.get("root", "")
    if root.startswith("file://"):
        root = root[7:]
    return {"id": lib["id"], "name": lib["name"], "root": root}


def _run_diff(library_id: str, root: str, hash_files: bool = False) -> dict:
    db_series = database.read_series(library_id)
    db_books = database.read_books(library_id)
    fs = walker.walk_library(root, hash_files=hash_files)
    d = differ.compute(db_series, db_books, fs)
    return {
        "db_series": len(db_series),
        "db_books": len(db_books),
        "fs_series": len(fs["series"]),
        "fs_books": sum(len(s["books"]) for s in fs["series"].values()) + len(fs.get("oneshots", [])),
        "diff": d,
        "fs_data": fs,
        "db_series_by_url": {s["url"]: s for s in db_series},
        "_db_series": db_series,
        "_db_books": db_books,
        "_fs": fs,
    }


def _curl_cmd(method: str, path: str, body: dict | None = None) -> str:
    """Build a curl command for the given Komga API call."""
    url = f"{config.KOMGA_URL}{path}"
    auth = f"{config.KOMGA_USER}:{config.KOMGA_PASSWORD}"
    cmd = f"curl -s -X {method} '{url}'"
    if auth != ":":
        cmd += f" \\\n  -u '{auth}'"
    cmd += " \\\n  -H 'Content-Type: application/json'"
    if body:
        body_str = json.dumps(body)
        # Escape single quotes in body for shell
        body_safe = body_str.replace("'", "'\\''")
        cmd += f" \\\n  -d '{body_safe}'"
    return cmd


def _book_body(b: dict) -> dict:
    """Build book dict for API body, including fileHash only when available."""
    body = {
        "name": b["name"],
        "url": b["url"],
        "fileSize": b["file_size"],
        "fileLastModified": b["file_last_modified"],
    }
    if b.get("file_hash"):
        body["fileHash"] = b["file_hash"]
    return body


# ── Page 1: List libraries ──────────────────────────────────

def list_libraries() -> ActionTable:
    """Select a library to scan (PDF files only)."""
    libs = api.list_libraries()
    return ActionTable(
        data=[
            {
                "library_id": l["id"],
                "library_name": l["name"],
                "library_root": l.get("root", ""),
            }
            for l in libs
        ],
        action=scan_library,
    )


# ── Page 2: Scan library (show diff) ────────────────────────

def scan_library(
    library_id: Annotated[str, Label("Library ID")],
    library_name: Annotated[str, Label("Name")] = "",
    library_root: Annotated[str, Label("Root")] = "",
    hash_files: Annotated[bool, Label("🔐 Compute file hashes (slower, more accurate)")] = False,
):
    """Scan a library and show what changed."""
    if library_root.startswith("file://"):
        library_root = library_root[7:]

    result = _run_diff(library_id, library_root, hash_files=hash_files)
    d = result["diff"]

    j1_path, j2_path = export_json.export_snapshots(
        library_id, library_name, library_root,
        result["_db_series"], result["_db_books"], result["_fs"],
    )

    # ── Build output tables ───────────────────────────────

    new_series_rows = [
        {"Series": s.get("name", ""),
         "Books": len(s.get("books", [])),
         "URL": s.get("url", "")}
        for s in d.new_series
    ]

    deleted_series_rows = [
        {"Series": s.get("name", ""),
         "ID": s.get("id", ""),
         "URL": s.get("url", "")}
        for s in d.deleted_series
    ]

    new_books_rows = [
        {"Book": b.get("name", ""),
         "URL": b.get("url", "")}
        for b in d.new_books
    ]

    deleted_books_rows = [
        {"Book": b.get("name", ""),
         "ID": b.get("id", ""),
         "URL": b.get("url", "")}
        for b in d.deleted_books
    ]

    changed_books_rows = [
        {"Book": b.get("name", ""),
         "ID": b.get("id", ""),
         "URL": b.get("url", "")}
        for b in d.changed_books
    ]

    totals = (
        f"Library: **{library_name}**\n\n"
        f"DB state: {result['db_series']} series, {result['db_books']} books\n"
        f"Filesystem: {result['fs_series']} series, {result['fs_books']} PDF files\n\n"
        f"📁 Exported: `{j1_path}` and `{j2_path}`\n\n"
        f"### Changes\n"
        f"| Category | Count |\n|----------|------|\n"
        f"| ✨ New series | {len(d.new_series)} |\n"
        f"| 🗑 Deleted series | {len(d.deleted_series)} |\n"
        f"| 📄 New books | {len(d.new_books)} |\n"
        f"| 🗑 Deleted books | {len(d.deleted_books)} |\n"
        f"| ✏ Changed books | {len(d.changed_books)} |"
    )

    total_actions = (len(d.new_series) + len(d.deleted_series) +
                     len(d.new_books) + len(d.deleted_books) +
                     len(d.changed_books))

    if total_actions == 0:
        return (totals, "✅ No changes detected — library is up to date.")

    outputs = [totals]

    if new_series_rows:
        outputs.append(new_series_rows)
    if deleted_series_rows:
        outputs.append(deleted_series_rows)
    if new_books_rows:
        outputs.append(new_books_rows)
    if deleted_books_rows:
        outputs.append(deleted_books_rows)
    if changed_books_rows:
        outputs.append(changed_books_rows)

    # ── Action options: export curl or apply via API ──────

    action_options = [
        {
            "action_mode": "export",
            "action_label": "📋 Export curl commands",
            "action_library_id": library_id,
            "action_library_name": library_name,
            "action_library_root": library_root,
            "action_hash_files": str(hash_files),
        },
        {
            "action_mode": "apply",
            "action_label": "🚀 Apply via API",
            "action_library_id": library_id,
            "action_library_name": library_name,
            "action_library_root": library_root,
            "action_hash_files": str(hash_files),
        },
    ]

    outputs.append(
        ActionTable(
            data=action_options,
            action=run_action,
            headers=["action_mode", "action_label", "action_library_id",
                     "action_library_name", "action_library_root",
                     "action_hash_files"],
        )
    )

    return tuple(outputs)


# ── Page 3: Run action (export curl or apply via API) ───────

def run_action(
    action_mode: Annotated[str, Label("Mode")],
    action_label: Annotated[str, Label("Action")] = "",
    action_library_id: Annotated[str, Label("Library ID")] = "",
    action_library_name: Annotated[str, Label("Name")] = "",
    action_library_root: Annotated[str, Label("Root")] = "",
    action_hash_files: Annotated[str, Label("Hash")] = "False",
    analyze: Annotated[bool, Label("📖 Analyze books")] = True,
    refresh: Annotated[bool, Label("🔄 Refresh metadata (books + series)")] = True,
):
    """Export curl commands or apply changes via API."""
    if action_library_root.startswith("file://"):
        action_library_root = action_library_root[7:]

    hash_files = action_hash_files.lower() == "true"
    result = _run_diff(action_library_id, action_library_root, hash_files=hash_files)
    d = result["diff"]

    if action_mode == "export":
        return _build_curl_export(d, action_library_id, action_library_name,
                                  analyze=analyze, refresh=refresh)
    else:
        return _apply_changes(d, action_library_id, action_library_name,
                              analyze=analyze, refresh=refresh)


# ── Curl export mode ────────────────────────────────────────

def _build_curl_export(d: differ.Diff, library_id: str, library_name: str,
                       analyze: bool = True, refresh: bool = True):
    """Generate curl commands for all detected changes."""
    lines = [f"#!/bin/bash",
             f"# Komga Smart Scanner — API calls for library: {library_name}",
             f"# Generated: {export_json.datetime.now(export_json.timezone.utc).isoformat()}",
             f"# analyze={analyze} refresh={refresh}",
             f""]

    # Create new series + books
    for s in d.new_series:
        lines.append(f"# Create series: {s['name']}")
        lines.append(_curl_cmd("POST", "/api/v1/series", {
            "libraryId": library_id,
            "name": s["name"],
            "url": s["url"],
            "fileLastModified": s["file_last_modified"],
            "books": [
                _book_body(b)
                for b in s.get("books", [])
            ],
        }))
        if analyze:
            lines.append(f"# Analyze books in new series: {s['name']}")
            for b in s.get("books", []):
                lines.append(f"#   {b['name']}")
            lines.append(f"# Use GET /api/v1/series/<ID>/books to get book IDs, then POST each")
        lines.append("")

    # Delete books
    for b in d.deleted_books:
        lines.append(f"# Delete book: {b['name']}")
        lines.append(_curl_cmd("DELETE", f"/api/v1/books/{b['id']}/file"))
        lines.append("")

    # Delete series
    for s in d.deleted_series:
        lines.append(f"# Delete series: {s['name']}")
        lines.append(_curl_cmd("DELETE", f"/api/v1/series/{s['id']}/file"))
        lines.append("")

    if d.deleted_books or d.deleted_series:
        lines.append("# Empty trash")
        lines.append(_curl_cmd("POST", f"/api/v1/libraries/{library_id}/empty-trash"))
        lines.append("")

    # Analyze changed books
    if analyze and d.changed_books:
        for b in d.changed_books:
            lines.append(f"# Analyze book: {b['name']}")
            lines.append(_curl_cmd("POST", f"/api/v1/books/{b['id']}/analyze"))
            lines.append("")

    # Refresh book metadata
    if refresh and d.changed_books:
        for b in d.changed_books:
            lines.append(f"# Refresh book metadata: {b['name']}")
            lines.append(_curl_cmd("POST", f"/api/v1/books/{b['id']}/metadata/refresh"))
            lines.append("")

    # Refresh series metadata for affected series
    if refresh and d.changed_books:
        affected = set()
        for b in d.changed_books:
            if b.get("series_id"):
                affected.add(b["series_id"])
        for sid in affected:
            lines.append(f"# Refresh series metadata: {sid}")
            lines.append(_curl_cmd("POST", f"/api/v1/series/{sid}/metadata/refresh"))
            lines.append("")

    script = "\n".join(lines)

    path = os.path.join(export_json.EXPORT_DIR,
                        f"{library_name.replace(' ', '_')}_curl_{_timestamp()}.sh")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(script)

    return (
        f"Exported curl commands to `{path}`\n\n"
        f"Run with: `bash {path}`\n\n"
        f"{len(d.new_series)} new series · "
        f"{len(d.deleted_series)} deleted series · "
        f"{len(d.deleted_books)} deleted books · "
        f"{len(d.changed_books)} changed books"
    )


# ── API apply mode ──────────────────────────────────────────

def _apply_changes(d: differ.Diff, library_id: str, library_name: str):
    """Apply diff via Komga API calls with print() streaming progress."""

    for s in d.new_series:
        print(f"+ Creating series: {s['name']}")
        try:
            created = api.create_series(
                library_id=library_id, name=s["name"], url=s["url"],
                file_last_modified=s["file_last_modified"],
                books=s.get("books", []),
            )
            books = api.get_series_books(created["id"])
            for b in books:
                api.analyze_book(b["id"])
                api.refresh_book_metadata(b["id"])
            print(f"  Analyzed {len(books)} books")
        except Exception as e:
            print(f"  ERROR: {e}")

    for b in d.deleted_books:
        print(f"- Deleting book: {b['name']}")
        try:
            api.delete_book(b["id"])
        except Exception as e:
            print(f"  ERROR: {e}")

    for s in d.deleted_series:
        print(f"- Deleting series: {s['name']}")
        try:
            api.delete_series(s["id"])
        except Exception as e:
            print(f"  ERROR: {e}")

    if d.deleted_books or d.deleted_series:
        print("Emptying trash...")
        try:
            api.empty_trash(library_id)
        except Exception as e:
            print(f"  ERROR: {e}")

    for b in d.changed_books:
        print(f"~ Analyzing: {b['name']}")
        try:
            api.analyze_book(b["id"])
            api.refresh_book_metadata(b["id"])
        except Exception as e:
            print(f"  ERROR: {e}")

    affected = set()
    for b in d.changed_books:
        if b.get("series_id"):
            affected.add(b["series_id"])
    for sid in affected:
        try:
            api.refresh_series_metadata(sid)
        except Exception as e:
            print(f"  ERROR refreshing series {sid}: {e}")

    print("Done.")
    return f"Applied changes to **{library_name}**"


def _timestamp() -> str:
    from datetime import datetime, timezone
    return datetime.now(tz=timezone.utc).strftime("%Y%m%d-%H%M%S")


# ── Run ─────────────────────────────────────────────────────

if __name__ == "__main__":
    port = int(os.environ.get("PORT", "5050"))
    run(
        [
            list_libraries,
            scan_library,
            HiddenFunction(run_action),
        ],
        app_title="Komga Smart Scanner",
        host="0.0.0.0",
        port=port,
    )
