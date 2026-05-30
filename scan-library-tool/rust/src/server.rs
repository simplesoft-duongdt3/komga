use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use futures::{Stream, StreamExt};
use serde::Deserialize;
use tokio::sync::Mutex as TokioMutex;
use tokio_stream::wrappers::ReceiverStream;


use crate::api::KomgaClient;
use crate::config::Config;
use crate::db;
use crate::diff as diff_engine;
use crate::export;
use crate::hasher;
use crate::models::*;
use crate::perf::ScanTimer;
use crate::walker;

// ── App state ───────────────────────────────────────────────

const GENERATING_TIMEOUT: Duration = Duration::from_secs(300); // 5 min

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub api_client: KomgaClient,
    pub pg_pool: sqlx::PgPool,
    pub generating_libs: Arc<Mutex<std::collections::HashMap<String, Instant>>>,
    pub library_root_cache: Arc<TokioMutex<HashMap<String, (String, Instant)>>>,
}

/// RAII guard that removes a library ID from generating_libs on drop,
/// even if the enclosing task panics.
struct GeneratingGuard {
    libs: Arc<Mutex<std::collections::HashMap<String, Instant>>>,
    library_id: String,
}

impl GeneratingGuard {
    fn new(libs: Arc<Mutex<std::collections::HashMap<String, Instant>>>, library_id: String) -> Self {
        Self { libs, library_id }
    }
}

impl Drop for GeneratingGuard {
    fn drop(&mut self) {
        self.libs.lock().unwrap().remove(&self.library_id);
    }
}

// ── Request types ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct ScanRequest {
    pub library_id: String,
    pub hash_files: Option<bool>,
}

#[derive(Deserialize)]
pub struct CurlRequest {
    pub request_id: String,
    pub categories: Vec<String>,
    pub analyze: Option<bool>,
    pub refresh: Option<bool>,
}

#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub request_id: String,
    pub script_name: String,
}

#[derive(Deserialize)]
pub struct HashCacheRequest {
    pub threads: Option<usize>,
}

// ── Router ──────────────────────────────────────────────────

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/static/app.js", get(app_js_handler))
        .route("/api/version", get(version_handler))
        .route("/api/libraries", get(libraries_handler))
        .route("/api/list-requests", get(list_requests_handler))
        .route("/api/scan", post(scan_handler))
        .route("/api/curl", post(curl_handler))
        .route("/api/execute", post(execute_handler))
        .route("/api/exports/{*path}", get(exports_handler))
        .route("/api/hash-cache/{library_id}", get(hash_cache_info_handler))
        .route("/api/hash-cache/{library_id}", post(hash_cache_generate_handler))
        .route("/api/hash-cache/{library_id}/download", get(hash_cache_download_handler))
        .with_state(state)
}

async fn app_js_handler() -> Response {
    let js = include_str!("../static/app.js");
    (
        [("content-type", "application/javascript; charset=utf-8")],
        js,
    )
        .into_response()
}

// ── Handlers ────────────────────────────────────────────────

async fn root_handler() -> Response {
    let html = include_str!("../static/index.html");
    (
        [("content-type", "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

async fn version_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": state.config.version,
        "title": "Komga Smart Scanner"
    }))
}

async fn libraries_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<Library>>, AppError> {
    let libs = state.api_client.list_libraries().await.map_err(AppError::from)?;
    Ok(Json(libs))
}

async fn list_requests_handler(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let export_path = Path::new(&state.config.export_dir);
    if !export_path.is_dir() {
        return Json(vec![]);
    }

    let mut reqs = Vec::new();
    if let Ok(entries) = fs::read_dir(export_path) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name.len() >= 13 && name.matches('-').count() == 1)
            .collect();
        names.sort_by(|a, b| b.cmp(a));
        for name in names {
            reqs.push(serde_json::json!({"request_id": name}));
        }
    }
    Json(reqs)
}

// ── Scan handler ────────────────────────────────────────────

async fn scan_handler(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let libs = state.api_client.list_libraries().await.map_err(AppError::from)?;
    let lib = libs
        .iter()
        .find(|l| l.id == req.library_id)
        .ok_or_else(|| AppError::NotFound("Library not found".into()))?;

    let root = lib.root.as_deref().unwrap_or("");
    let root = root.strip_prefix("file://").unwrap_or(root).to_string();

    let request_id = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let mut timer = ScanTimer::new(request_id.clone());

    tracing::info!(request_id, library = %lib.name, "Starting scan");

    // 1. DB series
    timer.begin("db_series");
    let db_series = db::read_series(&state.pg_pool, &req.library_id).await?;
    timer.end();

    // 2. DB extra queries
    timer.begin("db_extra");
    let db_unanalyzed = db::read_unanalyzed_books(&state.pg_pool, &req.library_id).await?;
    let db_no_thumbnail = db::read_books_missing_thumbnail(&state.pg_pool, &req.library_id).await?;
    timer.end();

    let hash_files = req.hash_files.unwrap_or(true);

    // 3. Load hash cache
    let hash_cache = if hash_files {
        let cache_file = Path::new(&state.config.hash_cache_dir)
            .join(format!("hashes-{}.json", req.library_id));
        let entries = hasher::load_cache(&cache_file);
        tracing::info!(request_id, count = entries.len(), "Hash cache loaded");
        Some(Arc::new(entries))
    } else {
        None
    };

    // 4. Parallel: DB books + FS walk
    let db_books_fut = db::read_books(&state.pg_pool, &req.library_id);
    let fs_data_fut = {
        let root = root.clone();
        let hash_cache = hash_cache.clone();
        let scan_threads = state.config.scan_threads;
        tokio::task::spawn_blocking(move || {
            let t0 = Instant::now();
            let cache_ref = hash_cache.as_ref().map(|arc| arc.as_ref().clone());
            let data = walker::walk_library(
                &root, &[], None, hash_files, &cache_ref,
                scan_threads, None::<fn(usize, usize, &str)>,
            );
            let elapsed = t0.elapsed().as_millis() as u64;
            (data, elapsed)
        })
    };

    timer.begin("db_books");
    let (db_books, fs_result) = tokio::join!(db_books_fut, fs_data_fut);
    let db_books = db_books?;
    let (fs_data, fs_elapsed) = fs_result.map_err(|e| AppError::Internal(format!("Join error: {}", e)))?;
    timer.record_phase("db_books", fs_elapsed);
    timer.record_phase("fs_walk", fs_elapsed);

    tracing::info!(request_id, db_books = db_books.len(), fs_series = fs_data.series.len(), "Parallel phase complete");

    // 5. Diff
    timer.begin("diff");
    let mut d = diff_engine::compute(&db_series, &db_books, &fs_data);
    d.to_be_analyzed = db_unanalyzed;
    d.no_metadata = db_no_thumbnail;
    timer.end();

    // 6. Export JSONs
    timer.begin("export_jsons");
    let export_dir = Path::new(&state.config.export_dir);
    let paths = export::export_snapshots(
        &req.library_id, &lib.name, &root,
        &db_series, &db_books, &fs_data, &request_id, export_dir,
    );
    timer.end();

    // 7. Save J3 diff
    let folder = export_dir.join(&request_id);
    fs::create_dir_all(&folder).map_err(|e| AppError::Internal(format!("Failed to create export dir: {}", e)))?;

    let db_series_by_url: HashMap<String, DbSeries> = db_series
        .iter().map(|s| (s.url.clone(), s.clone())).collect();

    let diff_data = build_diff_json(
        &request_id, &req.library_id, &lib.name, &root,
        hash_files, &db_series, &db_books, &fs_data, &d, &db_series_by_url,
    );

    let diff_path = folder.join(format!("{}_diff.json", request_id));
    fs::write(&diff_path, serde_json::to_string_pretty(&diff_data).unwrap())
        .map_err(|e| AppError::Internal(format!("Failed to write diff JSON: {}", e)))?;

    // 8. Performance
    let perf = timer.finish();
    let perf_path = folder.join(format!("{}_perf.json", request_id));
    fs::write(&perf_path, serde_json::to_string_pretty(&perf).unwrap())
        .map_err(|e| AppError::Internal(format!("Failed to write perf JSON: {}", e)))?;

    let totals = serde_json::json!({
        "new_series": d.new_series.len(),
        "deleted_series": d.deleted_series.len(),
        "new_books": d.new_books.len(),
        "deleted_books": d.deleted_books.len(),
        "changed_books": d.changed_books.len(),
        "pending_hash": d.pending_hash.len(),
        "to_be_analyzed": d.to_be_analyzed.len(),
        "no_metadata": d.no_metadata.len(),
    });

    let fs_file_count: usize = fs_data.series.values().map(|s| s.books.len()).sum::<usize>() + fs_data.oneshots.len();

    let result = serde_json::json!({
        "request_id": request_id,
        "library": {"id": req.library_id, "name": lib.name, "root": root},
        "db": {"series": db_series.len(), "books": db_books.len()},
        "fs": {"series": fs_data.series.len(), "files": fs_file_count},
        "diff": totals,
        "has_new_series": !d.new_series.is_empty(),
        "has_deleted_series": !d.deleted_series.is_empty(),
        "has_new_books": !d.new_books.is_empty(),
        "has_deleted_books": !d.deleted_books.is_empty(),
        "has_changed_books": !d.changed_books.is_empty(),
        "has_pending_hash": !d.pending_hash.is_empty(),
        "has_to_be_analyzed": !d.to_be_analyzed.is_empty(),
        "has_no_metadata": !d.no_metadata.is_empty(),
        "total_actions":
            d.new_series.len() + d.deleted_series.len() + d.new_books.len()
            + d.deleted_books.len() + d.changed_books.len() + d.pending_hash.len()
            + d.to_be_analyzed.len() + d.no_metadata.len(),
        "perf": {
            "total_ms": perf.total_ms,
            "phases": perf.phases,
        },
        "files": {
            "db": paths.db,
            "fs": paths.fs,
            "diff": diff_path.to_string_lossy().to_string(),
            "perf": perf_path.to_string_lossy().to_string(),
        },
    });

    Ok(Json(result))
}

fn build_diff_json(
    request_id: &str, library_id: &str, library_name: &str, root: &str,
    hash_files: bool, db_series: &[DbSeries], db_books: &[DbBook],
    fs_data: &FsData, d: &Diff, db_series_by_url: &HashMap<String, DbSeries>,
) -> serde_json::Value {
    serde_json::json!({
        "type": "J3 — Diff Result",
        "request_id": request_id,
        "library": {"id": library_id, "name": library_name, "root": root},
        "hash_files": hash_files,
        "exported_at": Utc::now().to_rfc3339(),
        "db": {"series": db_series.len(), "books": db_books.len()},
        "fs": {
            "series": fs_data.series.len(),
            "files": fs_data.series.values().map(|s| s.books.len()).sum::<usize>() + fs_data.oneshots.len(),
        },
        "diff": {
            "new_series": d.new_series.iter().map(|s| serde_json::json!({"name": s.name, "url": s.url, "books": s.books.len()})).collect::<Vec<_>>(),
            "deleted_series": d.deleted_series.iter().map(|s| serde_json::json!({"name": s.name, "url": s.url})).collect::<Vec<_>>(),
            "new_books": d.new_books.iter().map(|b| serde_json::json!({"name": b.name, "url": b.url})).collect::<Vec<_>>(),
            "deleted_books": d.deleted_books.iter().map(|b| serde_json::json!({"name": b.name, "url": b.url, "id": b.id})).collect::<Vec<_>>(),
            "changed_books": d.changed_books.iter().map(|b| serde_json::json!({"name": b.name, "url": b.url, "id": b.id})).collect::<Vec<_>>(),
            "pending_hash": d.pending_hash.iter().map(|b| serde_json::json!({"name": b.name, "url": b.url, "id": b.id})).collect::<Vec<_>>(),
            "to_be_analyzed": d.to_be_analyzed.iter().map(|b| serde_json::json!({"name": b.name, "id": b.id})).collect::<Vec<_>>(),
            "no_metadata": d.no_metadata.iter().map(|b| serde_json::json!({"name": b.name, "id": b.id})).collect::<Vec<_>>(),
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
        "_raw_db_series_by_url": db_series_by_url,
    })
}

// ── Curl handler ────────────────────────────────────────────

async fn curl_handler(
    State(state): State<AppState>,
    Json(req): Json<CurlRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let folder = Path::new(&state.config.export_dir).join(&req.request_id);
    let diff_path = folder.join(format!("{}_diff.json", req.request_id));

    if !diff_path.exists() {
        return Err(AppError::NotFound(format!("Scan result not found: {}", req.request_id)));
    }

    let diff_data: serde_json::Value = fs::read_to_string(&diff_path)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .parse::<serde_json::Value>().map_err(|e: serde_json::Error| AppError::Internal(e.to_string()))?;

    let raw = diff_data.get("_raw_diff").ok_or_else(|| AppError::Internal("Missing _raw_diff".into()))?.clone();
    let raw_fs = diff_data.get("_raw_fs_data").ok_or_else(|| AppError::Internal("Missing _raw_fs_data".into()))?.clone();
    let raw_db = diff_data.get("_raw_db_series_by_url").ok_or_else(|| AppError::Internal("Missing _raw_db_series_by_url".into()))?.clone();

    let fs_data: FsData = serde_json::from_value(raw_fs).map_err(|e| AppError::Internal(e.to_string()))?;
    let db_series_by_url: HashMap<String, DbSeries> = serde_json::from_value(raw_db).map_err(|e| AppError::Internal(e.to_string()))?;

    let lib_id = diff_data.get("library").and_then(|l| l.get("id")).and_then(|v| v.as_str()).unwrap_or_default().to_string();

    let d = Diff {
        new_series: serde_json::from_value(raw.get("new_series").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        deleted_series: serde_json::from_value(raw.get("deleted_series").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        new_books: serde_json::from_value(raw.get("new_books").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        deleted_books: serde_json::from_value(raw.get("deleted_books").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        changed_books: serde_json::from_value(raw.get("changed_books").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        pending_hash: serde_json::from_value(raw.get("pending_hash").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        to_be_analyzed: serde_json::from_value(raw.get("to_be_analyzed").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
        no_metadata: serde_json::from_value(raw.get("no_metadata").unwrap_or(&serde_json::Value::Null).clone()).unwrap_or_default(),
    };

    let scripts = crate::applier::generate_curl_scripts(
        &d, &lib_id, &req.categories,
        req.analyze.unwrap_or(true), req.refresh.unwrap_or(true),
        &folder, &req.request_id,
        Some(&db_series_by_url), Some(&fs_data),
    );

    Ok(Json(serde_json::json!({"scripts": scripts})))
}

// ── Execute handler (SSE) ───────────────────────────────────

async fn execute_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let export_dir = Path::new(&state.config.export_dir);
    let script_path = export_dir.join(&req.request_id).join(format!("{}_{}.sh", req.request_id, req.script_name));

    // Path traversal protection
    let abs_path = script_path.canonicalize().map_err(|_| AppError::NotFound(format!("Script not found: {}", req.script_name)))?;
    let allowed_dir = export_dir.canonicalize().map_err(|_| AppError::Internal("Bad export dir".into()))?;
    if !abs_path.starts_with(&allowed_dir) {
        return Err(AppError::Forbidden);
    }

    let script_content = fs::read_to_string(&abs_path)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let commands: Vec<String> = script_content
        .lines()
        .filter(|l| l.trim_start().starts_with("curl "))
        .map(|l| l.replace("\\\n", " ").lines().collect::<Vec<_>>().join(" "))
        .collect();

    let total = commands.len();
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

    send_sse(&tx, &serde_json::json!({"type": "start", "total": total})).await;

    tokio::spawn(async move {
        for (i, cmd) in commands.iter().enumerate() {
            let short = if cmd.len() > 120 { format!("{}...", &cmd[..120]) } else { cmd.clone() };

            let result = tokio::process::Command::new("sh").arg("-c").arg(cmd).output().await;
            let (success, output, error) = match result {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    (out.status.success(), stdout, stderr)
                }
                Err(e) => (false, String::new(), e.to_string()),
            };

            // Redact credentials from command output
            let short_redacted = redact_credentials(&short);

            let entry = serde_json::json!({
                "type": "progress", "index": i + 1, "total": total,
                "command": short_redacted, "success": success, "output": output,
                "error": if !success && !error.is_empty() { error } else { String::new() },
            });
            if tx.send(sse_safe(&entry.to_string())).await.is_err() { break; }
        }
        send_sse(&tx, &serde_json::json!({"type": "done", "total": total, "succeeded": 0, "failed": 0})).await;
    });

    let stream = ReceiverStream::new(rx).map(|msg| Ok::<_, Infallible>(Event::default().data(msg)));
    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)).text(": heartbeat\n\n")))
}

fn sse_safe(msg: &str) -> String {
    msg.replace('\n', " ").replace('\r', " ")
}

async fn send_sse(tx: &tokio::sync::mpsc::Sender<String>, msg: &serde_json::Value) {
    let sanitized = sse_safe(&msg.to_string());
    let _ = tx.send(sanitized).await;
}

fn redact_credentials(cmd: &str) -> String {
    // Redact -u 'user:password' pattern
    if let Some(start) = cmd.find(" -u '") {
        if let Some(end) = cmd[start + 5..].find('\'') {
            let before = &cmd[..start + 5];
            let after = &cmd[start + 5 + end..];
            return format!("{}[REDACTED]{}", before, after);
        }
    }
    cmd.to_string()
}

// ── Exports handler ─────────────────────────────────────────

async fn exports_handler(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Result<Response, AppError> {
    let file_path = Path::new(&state.config.export_dir).join(&path);
    let abs_path = file_path.canonicalize().map_err(|_| AppError::NotFound("File not found".into()))?;
    let allowed_dir = Path::new(&state.config.export_dir).canonicalize().map_err(|_| AppError::Internal("Bad export dir".into()))?;

    if !abs_path.starts_with(&allowed_dir) { return Err(AppError::Forbidden); }
    if !abs_path.is_file() { return Err(AppError::NotFound("File not found".into())); }

    Ok(([("content-type", "application/octet-stream")], fs::read(abs_path).map_err(|e| AppError::Internal(e.to_string()))?).into_response())
}

// ── Hash cache handlers ─────────────────────────────────────

fn get_cache_path(config: &Config, library_id: &str) -> PathBuf {
    Path::new(&config.hash_cache_dir).join(format!("hashes-{}.json", library_id))
}

async fn resolve_library_root(state: &AppState, library_id: &str) -> Result<String, AppError> {
    let mut cache = state.library_root_cache.lock().await;
    if let Some((root, time)) = cache.get(library_id) {
        if time.elapsed() < Duration::from_secs(300) {
            return Ok(root.clone());
        }
    }

    let libs = state.api_client.list_libraries().await.map_err(AppError::from)?;
    let lib = libs.iter().find(|l| l.id == library_id).ok_or_else(|| AppError::NotFound("Library not found".into()))?;

    let root = lib.root.as_deref().unwrap_or("").trim_start_matches("file://").to_string();
    if root.is_empty() { return Err(AppError::BadRequest("Library root is empty".into())); }

    cache.insert(library_id.to_string(), (root.clone(), Instant::now()));
    Ok(root)
}

async fn hash_cache_info_handler(
    State(state): State<AppState>,
    AxumPath(library_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let cache_path = get_cache_path(&state.config, &library_id);

    if !cache_path.is_file() {
        return Json(serde_json::json!({"exists": false}));
    }

    let meta = match fs::metadata(&cache_path) {
        Ok(m) => m,
        Err(_) => return Json(serde_json::json!({"exists": false})),
    };

    let mut info = serde_json::json!({
        "exists": true,
        "path": cache_path.to_string_lossy(),
        "file_size_bytes": meta.len(),
    });

    if let Ok(data) = fs::read_to_string(&cache_path) {
        if let Ok(cache) = serde_json::from_str::<HashCache>(&data) {
            info["total_files"] = serde_json::json!(cache.total_files);
            info["total_bytes"] = serde_json::json!(cache.total_bytes);
            info["generated_at"] = serde_json::json!(
                chrono::DateTime::from_timestamp(cache.generated_at_unix_secs as i64, 0)
                    .map(|dt| dt.to_rfc3339()).unwrap_or_default()
            );
            info["elapsed_secs"] = serde_json::json!(cache.elapsed_secs);
        }
    }

    Json(info)
}

async fn hash_cache_download_handler(
    State(state): State<AppState>,
    AxumPath(library_id): AxumPath<String>,
) -> Result<Response, AppError> {
    let cache_path = get_cache_path(&state.config, &library_id);
    let abs_path = cache_path.canonicalize().map_err(|_| AppError::NotFound("Cache file not found".into()))?;
    let allowed_dir = Path::new(&state.config.hash_cache_dir).canonicalize().map_err(|_| AppError::Internal("Bad cache dir".into()))?;

    if !abs_path.starts_with(&allowed_dir) { return Err(AppError::Forbidden); }

    Ok(([("content-type", "application/json")], fs::read(abs_path).map_err(|e| AppError::Internal(e.to_string()))?).into_response())
}

async fn hash_cache_generate_handler(
    State(state): State<AppState>,
    AxumPath(library_id): AxumPath<String>,
    Json(req): Json<Option<HashCacheRequest>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    // Resolve root BEFORE marking as in-progress — this can fail
    let root = resolve_library_root(&state, &library_id).await?;
    let threads = req.and_then(|r| r.threads).unwrap_or(state.config.hasher_threads);

    // Now mark as in-progress
    {
        let mut generating = state.generating_libs.lock().unwrap();
        // Check for stale entry (previous task may have crashed without cleanup)
        if let Some(started) = generating.get(&library_id) {
            if started.elapsed() < GENERATING_TIMEOUT {
                return Err(AppError::BadRequest("Hash generation already in progress for this library".into()));
            }
            tracing::warn!(library_id, elapsed_secs = started.elapsed().as_secs(), "Clearing stale generating_libs entry");
        }
        generating.insert(library_id.clone(), Instant::now());
    }

    let gen_lib_id = library_id.clone();
    let cache_path = get_cache_path(&state.config, &library_id);
    let generating = state.generating_libs.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

    tokio::spawn(async move {
        // Ensure cleanup on panic or normal exit
        let _guard = GeneratingGuard::new(generating, gen_lib_id);

        send_sse(&tx, &serde_json::json!({"event": "phase", "phase": "walking"})).await;

        let cache_path_clone = cache_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            let root_path = Path::new(&root);
            hasher::build_cache(root_path, &cache_path_clone, threads)
        }).await;

        match result {
            Ok(Ok(cache)) => {
                send_sse(&tx, &serde_json::json!({"event": "phase", "phase": "writing"})).await;
                match hasher::write_cache(&cache, &cache_path) {
                    Ok(_) => {
                        send_sse(&tx, &serde_json::json!({
                            "event": "done", "success": true,
                            "file_size_bytes": cache_path.metadata().map(|m| m.len()).unwrap_or(0),
                            "generated_at": Utc::now().to_rfc3339(),
                        })).await;
                    }
                    Err(e) => {
                        send_sse(&tx, &serde_json::json!({"event": "done", "success": false, "error": e.to_string()})).await;
                    }
                }
            }
            Ok(Err(e)) => {
                send_sse(&tx, &serde_json::json!({"event": "done", "success": false, "error": e.to_string()})).await;
            }
            Err(e) => {
                send_sse(&tx, &serde_json::json!({"event": "done", "success": false, "error": e.to_string()})).await;
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(|msg| Ok::<_, Infallible>(Event::default().data(msg)));
    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)).text(": heartbeat\n\n")))
}

// ── AppError ────────────────────────────────────────────────

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Forbidden,
    BadRequest(String),
    Internal(String),
    Reqwest(reqwest::Error),
    Sqlx(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".into()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            AppError::Reqwest(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            AppError::Sqlx(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self { AppError::Reqwest(e) }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self { AppError::Sqlx(e) }
}
