"""FuncToWeb server — type-hinted functions get auto-generated web UI."""

import os
from typing import Annotated

from func_to_web import run, ActionTable, HiddenFunction
from func_to_web.types import Label

import api
import db as database
import walker
import diff as differ


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


def _run_diff(library_id: str, root: str) -> dict:
    db_series = database.read_series(library_id)
    db_books = database.read_books(library_id)
    fs = walker.walk_library(root)
    d = differ.compute(db_series, db_books, fs)
    return {
        "db_series": len(db_series),
        "db_books": len(db_books),
        "fs_series": len(fs["series"]),
        "fs_books": sum(len(s["books"]) for s in fs["series"].values()) + len(fs.get("oneshots", [])),
        "diff": d,
        "fs_data": fs,
        "db_series_by_url": {s["url"]: s for s in db_series},
    }


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
):
    """Scan a library and show what changed."""
    if library_root.startswith("file://"):
        library_root = library_root[7:]

    result = _run_diff(library_id, library_root)
    d = result["diff"]

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

    apply_row = [{
        "apply_library_id": library_id,
        "apply_library_name": library_name,
        "apply_library_root": library_root,
        "apply_total": str(total_actions),
    }]

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

    outputs.append(
        ActionTable(
            data=apply_row,
            action=apply_scan,
            headers=list(apply_row[0].keys()),
        )
    )

    return tuple(outputs)


# ── Page 3: Apply changes (hidden, reached via ActionTable) ─

def apply_scan(
    apply_library_id: Annotated[str, Label("Library ID")],
    apply_library_name: Annotated[str, Label("Name")] = "",
    apply_library_root: Annotated[str, Label("Root")] = "",
    apply_total: Annotated[str, Label("Total changes")] = "",
):
    """Apply the detected changes to Komga."""
    if apply_library_root.startswith("file://"):
        apply_library_root = apply_library_root[7:]

    # Re-run diff
    result = _run_diff(apply_library_id, apply_library_root)
    d = result["diff"]
    fs_data = result["fs_data"]
    db_series_by_url = result["db_series_by_url"]

    # ── Step 1: Create new series + books ──────────────────
    for s in d.new_series:
        print(f"+ Creating series: {s['name']}")
        try:
            created = api.create_series(
                library_id=apply_library_id,
                name=s["name"],
                url=s["url"],
                file_last_modified=s["file_last_modified"],
                books=s.get("books", []),
            )
            series_id = created["id"]
            books = api.get_series_books(series_id)
            for b in books:
                api.analyze_book(b["id"])
                api.refresh_book_metadata(b["id"])
            print(f"  Analyzed {len(books)} books")
        except Exception as e:
            print(f"  ERROR: {e}")

    # ── Step 2: Delete books ───────────────────────────────
    for b in d.deleted_books:
        print(f"- Deleting book: {b['name']}")
        try:
            api.delete_book(b["id"])
        except Exception as e:
            print(f"  ERROR: {e}")

    # ── Step 3: Delete series ──────────────────────────────
    for s in d.deleted_series:
        print(f"- Deleting series: {s['name']}")
        try:
            api.delete_series(s["id"])
        except Exception as e:
            print(f"  ERROR: {e}")

    if d.deleted_books or d.deleted_series:
        print("Emptying trash...")
        try:
            api.empty_trash(apply_library_id)
        except Exception as e:
            print(f"  ERROR: {e}")

    # ── Step 4: Analyze changed books ──────────────────────
    for b in d.changed_books:
        print(f"~ Analyzing: {b['name']}")
        try:
            api.analyze_book(b["id"])
            api.refresh_book_metadata(b["id"])
        except Exception as e:
            print(f"  ERROR: {e}")

    # ── Step 5: Refresh series metadata ────────────────────
    affected_series_ids = set()
    for b in d.changed_books:
        if b.get("series_id"):
            affected_series_ids.add(b["series_id"])
    for sid in affected_series_ids:
        try:
            api.refresh_series_metadata(sid)
        except Exception as e:
            print(f"  ERROR refreshing series {sid}: {e}")

    print("Done.")
    return f"Applied changes to **{apply_library_name}**"


# ── Run ─────────────────────────────────────────────────────

if __name__ == "__main__":
    port = int(os.environ.get("PORT", "5050"))
    run(
        [
            list_libraries,
            scan_library,
            HiddenFunction(apply_scan),
        ],
        app_title="Komga Smart Scanner",
        host="0.0.0.0",
        port=port,
    )
