"""Orchestrator — applies diff results to Komga via API calls.
   Also generates curl scripts and executes them via subprocess."""

import json
import os
import subprocess
from datetime import datetime, timezone

from diff import Diff, group_new_books_by_series
from api import (
    create_series, delete_book, delete_series, empty_trash,
    analyze_book, refresh_book_metadata, refresh_series_metadata,
    get_series_books,
)
from config import DRY_RUN, KOMGA_URL, KOMGA_USER, KOMGA_PASSWORD


# ── Apply via API (existing) ──────────────────────────────

def apply(diff: Diff, library_id: str, fs_data: dict,
          db_series_by_url: dict):
    """Apply diff results to Komga via API calls."""
    def _log(action: str, detail: str):
        if DRY_RUN:
            print(f"  [DRY RUN] {action}: {detail}")
        else:
            print(f"  {action}: {detail}")

    for s in diff.new_series:
        _log("+ Creating series", s["name"])
        if not DRY_RUN:
            created = create_series(
                library_id=library_id, name=s["name"], url=s["url"],
                file_last_modified=s["file_last_modified"], books=s["books"],
            )
            for b in get_series_books(created["id"]):
                analyze_book(b["id"])
                refresh_book_metadata(b["id"])

    for s in diff.deleted_series:
        _log("- Deleting series", s["name"])
        if not DRY_RUN:
            delete_series(s["id"])

    if diff.deleted_books or diff.deleted_series:
        if not DRY_RUN:
            empty_trash(library_id)

    for b in diff.changed_books:
        _log("~ Analyzing", b["name"])
        if not DRY_RUN:
            analyze_book(b["id"])
            refresh_book_metadata(b["id"])

    affected = set()
    for b in diff.changed_books:
        if b.get("series_id"):
            affected.add(b["series_id"])
    for sid in affected:
        if not DRY_RUN:
            refresh_series_metadata(sid)


# ── Curl generation helpers ───────────────────────────────

def _curl_cmd(method: str, path: str, body: dict | None = None) -> str:
    """Build a curl command string."""
    url = f"{KOMGA_URL}{path}"
    auth = f"{KOMGA_USER}:{KOMGA_PASSWORD}"
    cmd = f"curl -s -X {method} '{url}'"
    if auth != ":":
        cmd += f" \\\n  -u '{auth}'"
    cmd += " \\\n  -H 'Content-Type: application/json'"
    if body:
        body_str = json.dumps(body)
        body_safe = body_str.replace("'", "'\\''")
        cmd += f" \\\n  -d '{body_safe}'"
    return cmd


def _book_body(b: dict) -> dict:
    body = {
        "name": b["name"], "url": b["url"],
        "fileSize": b["file_size"], "fileLastModified": b["file_last_modified"],
    }
    if b.get("file_hash"):
        body["fileHash"] = b["file_hash"]
    return body


# ── Curl script generation ────────────────────────────────

def generate_curl_scripts(
    diff: Diff, library_id: str, categories: list[str],
    analyze: bool, refresh: bool, output_dir: str,
    request_id: str,
    db_series_by_url: dict | None = None,
    fs_data: dict | None = None,
) -> dict[str, str]:
    """Generate one bash script per selected category.

    Returns: {category: file_path} mapping.
    """
    os.makedirs(output_dir, exist_ok=True)
    ts = datetime.now(tz=timezone.utc).isoformat()

    # Generate per-category
    builders = {
        "new_series": _bld_new_series(diff, library_id, analyze, refresh),
        "deleted_series": _bld_deleted_series(diff, library_id),
        "new_books": _bld_new_books(diff, library_id, db_series_by_url, fs_data),
        "deleted_books": _bld_deleted_books(diff, library_id),
        "changed_books": _bld_changed_books(diff, analyze, refresh),
        "pending_hash": _bld_pending_hash(diff),
    }

    scripts: dict[str, str] = {}
    for cat in categories:
        lines = builders.get(cat)
        if not lines:
            continue
        script = (
            "#!/bin/bash\n"
            f"# Komga Smart Scanner — script for: {cat}\n"
            f"# Request: {request_id} Generated: {ts}\n"
            f"# analyze={analyze} refresh={refresh}\n"
            f"\n"
        ) + "\n".join(lines)
        fname = f"{request_id}_{cat}.sh"
        fpath = os.path.join(output_dir, fname)
        with open(fpath, "w") as f:
            f.write(script)
        os.chmod(fpath, 0o755)
        scripts[cat] = fpath

    # Combined all.sh if >1 category selected
    if len(categories) > 1:
        combined = []
        for cat in categories:
            lines = builders.get(cat)
            if lines:
                combined.append(f"# ── {cat} ──")
                combined.extend(lines)
                combined.append("")
        if combined:
            script = (
                "#!/bin/bash\n"
                f"# Komga Smart Scanner — combined script\n"
                f"# Request: {request_id} Generated: {ts}\n"
                f"# Categories: {', '.join(categories)}\n"
                f"# analyze={analyze} refresh={refresh}\n"
                f"\n"
            ) + "\n".join(combined)
            fname = f"{request_id}_all.sh"
            fpath = os.path.join(output_dir, fname)
            with open(fpath, "w") as f:
                f.write(script)
            os.chmod(fpath, 0o755)
            scripts["all"] = fpath

    return scripts


def _bld_new_series(diff: Diff, library_id: str, analyze: bool, refresh: bool) -> list[str]:
    lines = []
    for s in diff.new_series:
        lines.append(f"# Create series: {s['name']}")
        lines.append(_curl_cmd("POST", "/api/v1/series", {
            "libraryId": library_id, "name": s["name"], "url": s["url"],
            "fileLastModified": s["file_last_modified"],
            "books": [_book_body(b) for b in s.get("books", [])],
        }))
        if analyze:
            lines.append(f"# Note: analyze books in new series via GET /api/v1/series/<ID>/books")
    return lines


def _bld_new_books(diff: Diff, library_id: str,
                   db_series_by_url: dict | None,
                   fs_data: dict | None) -> list[str]:
    """Generate curl commands to add new books to existing series.

    Uses POST /api/v1/series/{seriesId}/books (the new Komga endpoint).
    """
    if not diff.new_books:
        return []

    lines = [f"# New books in existing series ({len(diff.new_books)} total)"]

    if db_series_by_url and fs_data:
        from diff import group_new_books_by_series
        by_series = group_new_books_by_series(
            diff.new_books, fs_data.get("series", {}), db_series_by_url,
        )
        for series_id, books in by_series.items():
            lines.append(f"#  → series {series_id} ({len(books)} books)")
            lines.append(_curl_cmd("POST", f"/api/v1/series/{series_id}/books", {
                "libraryId": library_id,
                "books": [
                    {
                        "name": b["name"],
                        "url": b["url"],
                        "fileSize": b["file_size"],
                        "fileLastModified": b["file_last_modified"],
                    }
                    for b in books
                ],
            }))
    else:
        # Fallback: no series mapping data — trigger library scan
        lines.append("# No series mapping available — triggering library scan")
        lines.append(_curl_cmd("POST", f"/api/v1/libraries/{library_id}/scan?deep=false"))

    return lines


def _bld_deleted_series(diff: Diff, library_id: str) -> list[str]:
    lines = [_curl_cmd("POST", f"/api/v1/libraries/{library_id}/empty-trash")]
    for s in diff.deleted_series:
        lines.insert(0, f"# Delete series: {s['name']}")
        lines.insert(0, _curl_cmd("DELETE", f"/api/v1/series/{s['id']}/file"))
    return lines


def _bld_deleted_books(diff: Diff, library_id: str) -> list[str]:
    lines = [_curl_cmd("POST", f"/api/v1/libraries/{library_id}/empty-trash")]
    for b in diff.deleted_books:
        lines.insert(0, f"# Delete book: {b['name']}")
        lines.insert(0, _curl_cmd("DELETE", f"/api/v1/books/{b['id']}/file"))
    return lines


def _bld_changed_books(diff: Diff, analyze: bool, refresh: bool) -> list[str]:
    lines = []
    for b in diff.changed_books:
        if analyze:
            lines.append(f"# Analyze: {b['name']}")
            lines.append(_curl_cmd("POST", f"/api/v1/books/{b['id']}/analyze"))
        if refresh:
            lines.append(f"# Refresh metadata: {b['name']}")
            lines.append(_curl_cmd("POST", f"/api/v1/books/{b['id']}/metadata/refresh"))
    affected = set()
    for b in diff.changed_books:
        if b.get("series_id"):
            affected.add(b["series_id"])
    if refresh:
        for sid in affected:
            lines.append(f"# Refresh series metadata: {sid}")
            lines.append(_curl_cmd("POST", f"/api/v1/series/{sid}/metadata/refresh"))
    return lines


def _bld_pending_hash(diff: Diff) -> list[str]:
    """Generate analyze commands for books missing DB hash.

    POST /api/v1/books/{id}/analyze triggers Komga's analyzer to
    compute and store the XXH3_128 hash in the DB.
    """
    lines = []
    for b in diff.pending_hash:
        lines.append(f"# Hash book: {b['name']}")
        lines.append(_curl_cmd("POST", f"/api/v1/books/{b['id']}/analyze"))
    return lines


# ── Curl script execution ─────────────────────────────────

def parse_curl_commands(script_path: str) -> list[str]:
    """Parse a bash script into individual curl commands.

    Handles continuation lines ending with backslash.
    """
    with open(script_path) as f:
        text = f.read()
    text = text.replace("\\\n", " ")
    commands = []
    for line in text.split("\n"):
        line = line.strip()
        if line.startswith("curl "):
            commands.append(line)
    return commands


def execute_curl_script(script_path: str, log_path: str | None = None):
    """Execute each curl command in a script, yielding per-command results.

    Yields: dict type events for SSE streaming.
    """
    commands = parse_curl_commands(script_path)
    total = len(commands)
    log_entries: list[dict] = []

    yield {"type": "start", "total": total}

    for i, cmd in enumerate(commands):
        short = cmd[:120] + "..." if len(cmd) > 120 else cmd
        try:
            proc = subprocess.Popen(
                cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                text=True,
            )
            stdout, stderr = proc.communicate(timeout=300)
        except subprocess.TimeoutExpired:
            proc.kill()
            stdout, stderr = proc.communicate()
            entry = {"index": i + 1, "total": total, "command": short,
                     "success": False, "output": "", "error": "Timeout (300s)"}
            log_entries.append(entry)
            yield {"type": "progress", **entry}
            continue
        except Exception as e:
            entry = {"index": i + 1, "total": total, "command": short,
                     "success": False, "output": "", "error": str(e)}
            log_entries.append(entry)
            yield {"type": "progress", **entry}
            continue

        success = proc.returncode == 0
        entry = {"index": i + 1, "total": total, "command": short,
                 "success": success,
                 "output": (stdout or "").strip(),
                 "error": (stderr or "").strip() if not success else ""}
        log_entries.append(entry)
        yield {"type": "progress", **entry}

    succeeded = sum(1 for e in log_entries if e["success"])
    failed = total - succeeded

    # Write log if path provided
    if log_path:
        with open(log_path, "w") as f:
            f.write(f"Execution log — {datetime.now(tz=timezone.utc).isoformat()}\n")
            f.write(f"Total: {total}  Succeeded: {succeeded}  Failed: {failed}\n\n")
            for e in log_entries:
                status = "✓" if e["success"] else "✗"
                f.write(f"[{status}] ({e['index']}/{total}) {e['command']}\n")
                if e["output"]:
                    f.write(f"  {e['output']}\n")
                if e["error"]:
                    f.write(f"  ERROR: {e['error']}\n")
                f.write("\n")

    yield {"type": "done", "total": total, "succeeded": succeeded, "failed": failed}
