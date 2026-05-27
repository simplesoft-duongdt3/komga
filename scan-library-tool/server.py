"""FastAPI server — step-by-step web UI for smart scanning."""

import asyncio
import json
import os
import time
from datetime import datetime, timezone

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse, StreamingResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

import api
import db as database
import walker
import diff as differ
import export_json
from applier import generate_curl_scripts, execute_curl_script
from config import EXPORT_DIR, HASH_CACHE, SCAN_THREADS
from perf import ScanTimer

app = FastAPI(title="Komga Smart Scanner")

VERSION_FILE = os.path.join(os.path.dirname(__file__), "VERSION")


def _read_version() -> str:
    try:
        with open(VERSION_FILE) as f:
            return f.read().strip()
    except (FileNotFoundError, OSError):
        return "dev"


VERSION = _read_version()


@app.get("/api/version")
async def version():
    return {"version": VERSION, "title": "Komga Smart Scanner"}

STATIC_DIR = os.path.join(os.path.dirname(__file__), "static")
app.mount("/static", StaticFiles(directory=STATIC_DIR), name="static")


# ── SPA ─────────────────────────────────────────────────────

@app.get("/")
async def root():
    return FileResponse(os.path.join(STATIC_DIR, "index.html"))


# ── Pydantic schemas ────────────────────────────────────────

class ScanRequest(BaseModel):
    library_id: str
    hash_files: bool = True

class CurlRequest(BaseModel):
    request_id: str
    categories: list[str]
    analyze: bool = True
    refresh: bool = True

class ExecuteRequest(BaseModel):
    request_id: str
    script_name: str


# ── API: list libraries ────────────────────────────────────

@app.get("/api/libraries")
async def get_libraries():
    try:
        libs = api.list_libraries()
        return libs
    except Exception as e:
        raise HTTPException(500, str(e))


# ── API: list requests (history) ────────────────────────────

@app.get("/api/list-requests")
async def list_requests():
    export_path = EXPORT_DIR
    if not os.path.isdir(export_path):
        return []
    reqs = []
    for name in sorted(os.listdir(export_path), reverse=True):
        folder = os.path.join(export_path, name)
        if os.path.isdir(folder) and name.count("-") == 1 and len(name) >= 13:
            reqs.append({"request_id": name})
    return reqs


# ── API: scan (step 2) ─────────────────────────────────────

@app.post("/api/scan")
async def run_scan(req: ScanRequest):
    libs = api.list_libraries()
    lib = next((l for l in libs if l["id"] == req.library_id), None)
    if not lib:
        raise HTTPException(404, "Library not found")

    root = lib.get("root", "")
    if root.startswith("file://"):
        root = root[7:]

    request_id = datetime.now(tz=timezone.utc).strftime("%Y%m%d-%H%M%S")
    timer = ScanTimer(request_id, EXPORT_DIR)

    print(f"[scan] {request_id} — Starting scan for library '{lib['name']}' (id={req.library_id})")

    # 1. DB snapshot
    print(f"[scan] {request_id} — Phase: reading DB series...")
    timer.begin("db_series")
    db_series = database.read_series(req.library_id)
    timer.end()
    print(f"[scan] {request_id} — DB series: {len(db_series)} rows in {next(p['elapsed_ms'] for p in timer.phases if p['phase']=='db_series')}ms")

    # 2. Query DB-only categories (unanalyzed books, missing thumbnails)
    timer.begin("db_extra")
    db_unanalyzed = database.read_unanalyzed_books(req.library_id)
    db_no_thumbnail = database.read_books_missing_thumbnail(req.library_id)
    timer.end()
    print(f"[scan] {request_id} — DB extra: {len(db_unanalyzed)} unanalyzed, {len(db_no_thumbnail)} no thumbnail in {next(p['elapsed_ms'] for p in timer.phases if p['phase']=='db_extra')}ms")

    # 3. Load hash cache (if available)
    if req.hash_files:
        cached = walker.load_hash_cache(HASH_CACHE)
        print(f"[scan] {request_id} — Hash cache: {cached} entries loaded from {HASH_CACHE}")

    # 3. Run DB books + FS walk in parallel (independent operations)
    print(f"[scan] {request_id} — Phase: reading DB books + walking FS in parallel "
          f"(threads={SCAN_THREADS or 'auto'}, hash={req.hash_files})...")

    def _read_db_books():
        t0 = time.monotonic()
        books = database.read_books(req.library_id)
        elapsed_ms = round((time.monotonic() - t0) * 1000)
        timer.record_phase("db_books", elapsed_ms)
        print(f"[scan] {request_id} — DB books: {len(books)} rows in {elapsed_ms}ms")
        return books

    def _walk_fs():
        t0 = time.monotonic()
        fs_walked = [0]
        _last_progress_time = [time.monotonic()]
        _scan_start_time = time.monotonic()
        def _progress(current, total, dir_name=""):
            fs_walked[0] = current
            now = time.monotonic()
            gap = now - _last_progress_time[0]
            elapsed_total = now - _scan_start_time
            _last_progress_time[0] = now
            print(f"[scan] {request_id} — FS walk: [{current}/{total}] {dir_name}  "
                  f"(gap={gap:.1f}s, elapsed={elapsed_total:.0f}s)")

        fs_data = walker.walk_library(root, hash_files=req.hash_files,
                                       max_workers=SCAN_THREADS or None,
                                       on_progress=_progress)
        elapsed_ms = round((time.monotonic() - t0) * 1000)
        timer.record_phase("fs_walk", elapsed_ms, meta={"hash_files": req.hash_files})
        series_count = len(fs_data['series'])
        file_count = sum(len(s['books']) for s in fs_data['series'].values())
        print(f"[scan] {request_id} — FS walk: {series_count} series, {file_count} files in {elapsed_ms}ms")
        return fs_data

    db_task = asyncio.to_thread(_read_db_books)
    fs_task = asyncio.to_thread(_walk_fs)
    db_books, fs_data = await asyncio.gather(db_task, fs_task)
    print(f"[scan] {request_id} — Parallel phase complete: db_books={len(db_books)} rows, "
          f"fs_walk={len(fs_data['series'])} dirs")

    # 4. Diff
    print(f"[scan] {request_id} — Phase: computing diff...")
    timer.begin("diff")
    d = differ.compute(db_series, db_books, fs_data)
    d.to_be_analyzed = db_unanalyzed
    d.no_metadata = db_no_thumbnail
    timer.end()
    print(f"[scan] {request_id} — Diff: {len(d.new_series)} new series, {len(d.deleted_series)} del series, "
          f"{len(d.new_books)} new books, {len(d.deleted_books)} del books, "
          f"{len(d.changed_books)} changed, {len(d.pending_hash)} pending hash, "
          f"{len(d.to_be_analyzed)} to be analyzed, {len(d.no_metadata)} no thumbnail")

    # 5. Export JSONs to request_id folder
    print(f"[scan] {request_id} — Phase: exporting JSONs...")
    timer.begin("export_jsons")
    paths = export_json.export_snapshots(
        req.library_id, lib["name"], root,
        db_series, db_books, fs_data, request_id,
    )
    timer.end()

    # 6. Save diff as J3
    folder = os.path.join(EXPORT_DIR, request_id)
    os.makedirs(folder, exist_ok=True)

    diff_data = {
        "type": "J3 — Diff Result",
        "request_id": request_id,
        "library": {"id": req.library_id, "name": lib["name"], "root": root},
        "hash_files": req.hash_files,
        "exported_at": datetime.now(tz=timezone.utc).isoformat(),
        "db": {"series": len(db_series), "books": len(db_books)},
        "fs": {
            "series": len(fs_data["series"]),
            "files": (
                sum(len(s["books"]) for s in fs_data["series"].values())
                + len(fs_data.get("oneshots", []))
            ),
        },
        "diff": {
            "new_series": _ser_to_dict(d.new_series),
            "deleted_series": _ser_to_dict(d.deleted_series),
            "new_books": _book_to_dict(d.new_books),
            "deleted_books": _book_to_dict(d.deleted_books),
            "changed_books": _book_to_dict(d.changed_books),
            "pending_hash": _book_to_dict(d.pending_hash),
            "to_be_analyzed": _book_to_dict(d.to_be_analyzed),
            "no_metadata": _book_to_dict(d.no_metadata),
        },
        "_raw_diff": {
            "new_series": d.new_series,
            "deleted_series": d.deleted_series,
            "new_books": d.new_books,
            "deleted_books": d.deleted_books,
            "changed_books": d.changed_books,
            "pending_hash": d.pending_hash,
            "to_be_analyzed": d.to_be_analyzed,
            "no_metadata": d.no_metadata,
        },
        "_raw_fs_data": fs_data,
        "_raw_db_series_by_url": {s["url"]: s for s in db_series},
    }

    diff_path = os.path.join(folder, f"{request_id}_diff.json")
    with open(diff_path, "w") as f:
        json.dump(diff_data, f, indent=2, default=str)

    # 7. Write performance log
    perf = timer.finish()
    timer.write_log(folder, perf)

    print(f"[scan] {request_id} — total={perf['total_ms']}ms "
          f"db_series={next(p['elapsed_ms'] for p in perf['phases'] if p['phase']=='db_series')}ms "
          f"db_books={next(p['elapsed_ms'] for p in perf['phases'] if p['phase']=='db_books')}ms "
          f"fs_walk={next(p['elapsed_ms'] for p in perf['phases'] if p['phase']=='fs_walk')}ms "
          f"diff={next(p['elapsed_ms'] for p in perf['phases'] if p['phase']=='diff')}ms "
          f"export={next(p['elapsed_ms'] for p in perf['phases'] if p['phase']=='export_jsons')}ms")

    totals = {
        "new_series": len(d.new_series),
        "deleted_series": len(d.deleted_series),
        "new_books": len(d.new_books),
        "deleted_books": len(d.deleted_books),
        "changed_books": len(d.changed_books),
        "pending_hash": len(d.pending_hash),
        "to_be_analyzed": len(d.to_be_analyzed),
        "no_metadata": len(d.no_metadata),
    }

    return {
        "request_id": request_id,
        "library": {"id": req.library_id, "name": lib["name"], "root": root},
        "db": {"series": len(db_series), "books": len(db_books)},
        "fs": {
            "series": len(fs_data["series"]),
            "files": (
                sum(len(s["books"]) for s in fs_data["series"].values())
                + len(fs_data.get("oneshots", []))
            ),
        },
        "diff": totals,
        "has_new_series": len(d.new_series) > 0,
        "has_deleted_series": len(d.deleted_series) > 0,
        "has_new_books": len(d.new_books) > 0,
        "has_deleted_books": len(d.deleted_books) > 0,
        "has_changed_books": len(d.changed_books) > 0,
        "has_pending_hash": len(d.pending_hash) > 0,
        "has_to_be_analyzed": len(d.to_be_analyzed) > 0,
        "has_no_metadata": len(d.no_metadata) > 0,
        "total_actions": sum(totals.values()),
        "perf": {
            "total_ms": perf["total_ms"],
            "phases": perf["phases"],
        },
        "files": {
            "db": paths["db"],
            "fs": paths["fs"],
            "diff": diff_path,
            "perf": os.path.join(folder, f"{request_id}_perf.json"),
        },
    }


def _ser_to_dict(items: list) -> list[dict]:
    return [{"name": s.get("name", ""), "url": s.get("url", ""),
             "books": len(s.get("books", []))} for s in items]

def _book_to_dict(items: list) -> list[dict]:
    return [{"name": b.get("name", ""), "url": b.get("url", ""),
             "id": b.get("id", "")} for b in items]


# ── API: generate curl scripts (step 3) ────────────────────

@app.post("/api/curl")
async def gen_curl(req: CurlRequest):
    folder = os.path.join(EXPORT_DIR, req.request_id)
    diff_path = os.path.join(folder, f"{req.request_id}_diff.json")
    if not os.path.exists(diff_path):
        raise HTTPException(404, f"Scan result not found: {req.request_id}")

    with open(diff_path) as f:
        diff_data = json.load(f)

    raw = diff_data.get("_raw_diff", {})
    fs_data = diff_data.get("_raw_fs_data", {"series": {}, "oneshots": []})

    d = differ.Diff(
        new_series=raw.get("new_series", []),
        deleted_series=raw.get("deleted_series", []),
        new_books=raw.get("new_books", []),
        deleted_books=raw.get("deleted_books", []),
        changed_books=raw.get("changed_books", []),
        pending_hash=raw.get("pending_hash", []),
        to_be_analyzed=raw.get("to_be_analyzed", []),
        no_metadata=raw.get("no_metadata", []),
    )

    lib_id = diff_data.get("library", {}).get("id", "")
    db_series_by_url = diff_data.get("_raw_db_series_by_url", {})
    fs_data = diff_data.get("_raw_fs_data", {"series": {}, "oneshots": []})

    scripts = generate_curl_scripts(
        d, lib_id, req.categories,
        req.analyze, req.refresh,
        folder, req.request_id,
        db_series_by_url=db_series_by_url,
        fs_data=fs_data,
    )

    return {"scripts": scripts}


# ── API: execute curl script (step 4, SSE stream) ──────────

@app.post("/api/execute")
async def exec_script(req: ExecuteRequest):
    folder = os.path.join(EXPORT_DIR, req.request_id)
    script_path = os.path.join(folder, f"{req.request_id}_{req.script_name}.sh")
    log_path = os.path.join(folder, f"{req.request_id}_{req.script_name}.log")

    if not os.path.exists(script_path):
        raise HTTPException(404, f"Script not found: {req.script_name}")

    async def event_stream():
        for event in execute_curl_script(str(script_path), str(log_path) if log_path else None):
            yield f"data: {json.dumps(event)}\n\n"

    return StreamingResponse(
        event_stream(),
        media_type="text/event-stream",
        headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"},
    )


# ── API: download exported files ───────────────────────────

@app.get("/api/exports/{request_id}/{filename:path}")
async def download_file(request_id: str, filename: str):
    file_path = os.path.join(EXPORT_DIR, request_id, filename)
    abs_path = os.path.normpath(file_path)
    allowed_dir = os.path.normpath(EXPORT_DIR)
    if not abs_path.startswith(allowed_dir):
        raise HTTPException(403, "Forbidden")
    if not os.path.isfile(abs_path):
        raise HTTPException(404, "File not found")
    return FileResponse(abs_path)


# ── Main ────────────────────────────────────────────────────

if __name__ == "__main__":
    import uvicorn
    port = int(os.environ.get("PORT", "5050"))
    uvicorn.run("server:app", host="0.0.0.0", port=port, log_level="info")
