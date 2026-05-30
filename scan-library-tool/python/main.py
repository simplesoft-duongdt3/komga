#!/usr/bin/env python3
"""Komga Smart Scanner — PDF-only diff-based scanner."""

import sys

import config
import db as database
import walker
import diff as differ
import applier
import api


def main():
    # 0. List libraries
    print("Fetching libraries...")
    libs = api.list_libraries()
    if not libs:
        print("No libraries found.")
        sys.exit(1)

    print("\nLibraries:")
    for i, lib in enumerate(libs):
        print(f"  [{i}] {lib['name']}  (id={lib['id']})")

    try:
        choice = int(input("\nSelect library: "))
        library = libs[choice]
    except (ValueError, IndexError):
        print("Invalid selection.")
        sys.exit(1)

    lid = library["id"]
    lib_name = library["name"]
    root = library.get("root", "")
    if root.startswith("file://"):
        root = root[7:]

    # 1. Read DB state
    print(f"\n── Reading DB state for '{lib_name}' ──")
    db_series = database.read_series(lid)
    db_books = database.read_books(lid)
    print(f"  DB: {len(db_series)} series, {len(db_books)} books")
    db_series_by_url = {s["url"]: s for s in db_series}

    # 2. Walk filesystem
    print("\n── Walking filesystem ──")
    fs = walker.walk_library(root)
    fs_series_count = len(fs["series"])
    fs_books_count = sum(len(s["books"]) for s in fs["series"].values()) + len(fs["oneshots"])
    print(f"  FS: {fs_series_count} series, {fs_books_count} PDF files")

    # 3. Diff
    d = differ.compute(db_series, db_books, fs)
    print(f"\n── Diff ──")
    print(f"  New series:      {len(d.new_series)}")
    print(f"  Deleted series:  {len(d.deleted_series)}")
    print(f"  New books:       {len(d.new_books)}")
    print(f"  Deleted books:   {len(d.deleted_books)}")
    print(f"  Changed books:   {len(d.changed_books)}")

    total_actions = (len(d.new_series) + len(d.deleted_series) +
                     len(d.new_books) + len(d.deleted_books) +
                     len(d.changed_books))

    if total_actions == 0:
        print("\n  No changes detected. Library is up to date.")
        return

    if config.DRY_RUN:
        print(f"\n── [DRY RUN] Preview ({total_actions} actions) ──")
    else:
        confirm = input(f"\n── Apply {total_actions} changes? [y/N] ── ")
        if confirm.lower() != "y":
            print("Aborted.")
            return

    # 4. Apply
    print(f"\n── Applying ({'DRY RUN' if config.DRY_RUN else 'live'}) ──")
    applier.apply(d, lid, fs, db_series_by_url)

    print("\nDone.")


if __name__ == "__main__":
    main()
