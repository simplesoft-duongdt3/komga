"""Orchestrator — applies diff results to Komga via API calls."""

from diff import Diff, group_new_books_by_series
from api import (
    create_series, delete_book, delete_series, empty_trash,
    analyze_book, refresh_book_metadata, refresh_series_metadata,
    get_series_books,
)
from config import DRY_RUN


def apply(diff: Diff, library_id: str, fs_data: dict,
          db_series_by_url: dict):
    """Apply diff results to Komga via API calls."""

    def _log(action: str, detail: str):
        if DRY_RUN:
            print(f"  [DRY RUN] {action}: {detail}")
        else:
            print(f"  {action}: {detail}")

    # ── Step 1: Create new series + their books ───────────
    for s in diff.new_series:
        _log("+ Creating series", s["name"])
        if not DRY_RUN:
            created = create_series(
                library_id=library_id,
                name=s["name"],
                url=s["url"],
                file_last_modified=s["file_last_modified"],
                books=s["books"],
            )
            series_id = created["id"]
            # Analyze + refresh each new book
            for b in get_series_books(series_id):
                analyze_book(b["id"])
                refresh_book_metadata(b["id"])

    # ── Step 2: Add new books to existing series ──────────
    by_series = group_new_books_by_series(
        diff.new_books, fs_data["series"], db_series_by_url,
    )
    for series_id, books in by_series.items():
        _log("+ Adding books", f"{len(books)} to series {series_id}")
        if not DRY_RUN:
            # We can't add books via API to existing series yet.
            # Fall back to triggering a fast scan for that series.
            _log("  (skipped)", "add-books-to-existing-series API not available")

    # For new books that could NOT be mapped to existing series
    unmapped = [b for b in diff.new_books
                if not any(b in blist for blist in by_series.values())]
    if unmapped:
        _log("! Unmapped new books", f"{len(unmapped)} could not be matched to existing series")
        _log("  Hint", "these may be oneshots or new series — run a full scan to register them")

    # ── Step 3: Delete removed books ──────────────────────
    for b in diff.deleted_books:
        _log("- Deleting book", b["name"])
        if not DRY_RUN:
            delete_book(b["id"])

    # ── Step 4: Delete removed series ─────────────────────
    for s in diff.deleted_series:
        _log("- Deleting series", s["name"])
        if not DRY_RUN:
            delete_series(s["id"])

    if diff.deleted_books or diff.deleted_series:
        if not DRY_RUN:
            empty_trash(library_id)

    # ── Step 5: Analyze + refresh changed books ───────────
    for i, b in enumerate(diff.changed_books, 1):
        _log("~ Analyzing", f"{b['name']} ({i}/{len(diff.changed_books)})")
        if not DRY_RUN:
            analyze_book(b["id"])
            refresh_book_metadata(b["id"])

    # ── Step 6: Refresh series metadata for affected series ──
    affected_series_ids: set[str] = set()
    for b in diff.changed_books:
        if b.get("series_id"):
            affected_series_ids.add(b["series_id"])
    for sid in affected_series_ids:
        _log("~ Refreshing series", sid)
        if not DRY_RUN:
            refresh_series_metadata(sid)
