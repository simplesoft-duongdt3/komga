"""FastAPI server — step-by-step web UI for smart scanning."""

import json
import os
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
from config import EXPORT_DIR

app = FastAPI(title="Komga Smart Scanner")

VERSION_FILE = os.path.join(os.path.dirname(__file__), "version.txt")


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

    # 1. DB snapshot
    db_series = database.read_series(req.library_id)
    db_books = database.read_books(req.library_id)

    # 2. FS walk (with optional hashing)
    fs_data = walker.walk_library(root, hash_files=req.hash_files)

    # 3. Diff
    d = differ.compute(db_series, db_books, fs_data)

    # 4. Export JSONs to request_id folder
    paths = export_json.export_snapshots(
        req.library_id, lib["name"], root,
        db_series, db_books, fs_data, request_id,
    )

    # 5. Save diff as J3
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
        },
        "_raw_diff": {
            "new_series": d.new_series,
            "deleted_series": d.deleted_series,
            "new_books": d.new_books,
            "deleted_books": d.deleted_books,
            "changed_books": d.changed_books,
            "pending_hash": d.pending_hash,
        },
        "_raw_fs_data": fs_data,
        "_raw_db_series_by_url": {s["url"]: s for s in db_series},
    }

    diff_path = os.path.join(folder, f"{request_id}_diff.json")
    with open(diff_path, "w") as f:
        json.dump(diff_data, f, indent=2, default=str)

    totals = {
        "new_series": len(d.new_series),
        "deleted_series": len(d.deleted_series),
        "new_books": len(d.new_books),
        "deleted_books": len(d.deleted_books),
        "changed_books": len(d.changed_books),
        "pending_hash": len(d.pending_hash),
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
        "total_actions": sum(totals.values()),
        "files": {
            "db": paths["db"],
            "fs": paths["fs"],
            "diff": diff_path,
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
