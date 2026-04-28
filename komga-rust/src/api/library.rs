use axum::{
    extract::{Path, State, Query},
    routing::{get, post, delete, patch, put},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use std::path::PathBuf;
use std::fs;

use crate::api::dto::LibraryDto;
use crate::domain::model::library::Library;
use crate::domain::model::task::{Task, TaskData, TaskType};
use crate::domain::repository::{TaskRepository, PageHashRepository};

async fn get_all_libraries(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<LibraryDto>>, axum::response::Response> {
    let repo = crate::domain::repository::LibraryRepository::new(pool);
    let libraries = repo.find_all().await.unwrap_or_default();
    Ok(Json(libraries.into_iter().map(|l| l.into()).collect()))
}

async fn get_library(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<LibraryDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = crate::domain::repository::LibraryRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(library)) => Ok(Json(library.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Library not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

#[derive(Deserialize)]
struct CreateLibraryRequest {
    name: String,
    root: String,
}

async fn create_library(
    State(pool): State<PgPool>,
    Json(req): Json<CreateLibraryRequest>,
) -> Result<Json<LibraryDto>, axum::response::Response> {
    let library = Library::new(req.name, req.root);
    let repo = crate::domain::repository::LibraryRepository::new(pool);
    
    match repo.create(&library).await {
        Ok(created) => Ok(Json(created.into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn delete_library(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<(), axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = crate::domain::repository::LibraryRepository::new(pool);
    
    match repo.delete(uuid).await {
        Ok(_) => Ok(()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn scan_library(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<String>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = crate::domain::repository::LibraryRepository::new(pool.clone());
    
    let _library = match repo.find_by_id(uuid).await {
        Ok(Some(l)) => l,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Library not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let task = Task::new(
        TaskType::ScanLibrary,
        TaskData::ScanLibrary { 
            library_id: id.clone(), 
            scan_deep: false 
        },
        4,
    );
    let task_repo = TaskRepository::new(pool.clone());
    if let Err(e) = task_repo.create(&task).await {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    Ok(Json(format!("Scan started for library: {}", id)))
}

async fn update_library(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(_req): Json<LibraryDto>,
) -> Result<Json<LibraryDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = crate::domain::repository::LibraryRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(library)) => Ok(Json(library.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Library not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn patch_library(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<LibraryDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = crate::domain::repository::LibraryRepository::new(pool.clone());
    
    let library = match repo.find_by_id(uuid).await {
        Ok(Some(l)) => l,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Library not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let mut updated = library.clone();
    
    if let Some(name) = req.get("name").and_then(|v| v.as_str()) {
        updated.name = name.to_string();
    }
    if let Some(root) = req.get("root").and_then(|v| v.as_str()) {
        updated.root = root.to_string();
    }
    if let Some(v) = req.get("importComicInfoBook").and_then(|v| v.as_bool()) {
        updated.import_comicinfo_book = v;
    }
    if let Some(v) = req.get("importComicInfoSeries").and_then(|v| v.as_bool()) {
        updated.import_comicinfo_series = v;
    }
    if let Some(v) = req.get("importEpubBook").and_then(|v| v.as_bool()) {
        updated.import_epub_book = v;
    }
    if let Some(v) = req.get("importEpubSeries").and_then(|v| v.as_bool()) {
        updated.import_epub_series = v;
    }
    if let Some(v) = req.get("importLocalArtwork").and_then(|v| v.as_bool()) {
        updated.import_local_artwork = v;
    }
    if let Some(v) = req.get("importBarcodeIsbn").and_then(|v| v.as_bool()) {
        updated.import_barcode_isbn = v;
    }
    if let Some(v) = req.get("convertToCbz").and_then(|v| v.as_bool()) {
        updated.convert_to_cbz = v;
    }
    if let Some(v) = req.get("hashFiles").and_then(|v| v.as_bool()) {
        updated.hash_files = v;
    }
    if let Some(v) = req.get("hashPages").and_then(|v| v.as_bool()) {
        updated.hash_pages = v;
    }
    if let Some(v) = req.get("analyzeDimensions").and_then(|v| v.as_bool()) {
        updated.analyze_dimensions = v;
    }
    
    match repo.update(&updated).await {
        Ok(lib) => Ok(Json(lib.into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn analyze_library(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::AnalyzeBook,
        TaskData::AnalyzeBook { book_id: id.clone() },
        4,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn refresh_library_metadata(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::RefreshSeriesMetadata,
        TaskData::RefreshSeriesMetadata { series_id: id.clone() },
        4,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn empty_trash(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::EmptyTrash,
        TaskData::EmptyTrash { library_id: id.clone() },
        4,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

#[derive(Deserialize)]
struct FilesystemRequest {
    path: String,
}

async fn filesystem(
    State(_pool): State<PgPool>,
    Json(req): Json<FilesystemRequest>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    use std::fs;
    let path = std::path::Path::new(&req.path);
    
    if !path.exists() {
        return Ok(Json(serde_json::json!({
            "path": req.path,
            "children": [],
        })));
    }
    
    let children: Vec<serde_json::Value> = fs::read_dir(path)
        .ok()
        .map(|entries| {
            entries.filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    let file_type = entry.file_type().ok()?;
                    Some(serde_json::json!({
                        "path": path.to_string_lossy().to_string(),
                        "name": path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                        "isDirectory": file_type.is_dir(),
                        "isFile": file_type.is_file(),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    
    Ok(Json(serde_json::json!({
        "path": req.path,
        "children": children,
    })))
}

async fn get_referential(
    State(pool): State<PgPool>,
    Query(_params): Query<ReferentialParams>,
) -> Result<Json<Vec<String>>, axum::response::Response> {
    #[allow(clippy::never_loop)]
    let results: Vec<String> = sqlx::query_as::<_, (String,)>(
        r#"SELECT DISTINCT "VALUE" FROM "REFERENTIAL" ORDER BY "VALUE" LIMIT 100"#
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(v,)| v)
    .collect();
    
    Ok(Json(results))
}

#[derive(Deserialize)]
struct ReferentialParams {
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    unpaged: Option<bool>,
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_size")]
    size: usize,
}

fn default_page() -> usize { 0 }
fn default_size() -> usize { 20 }

async fn get_sharing_labels(State(_pool): State<PgPool>) -> Json<Vec<String>> {
    Json(vec![])
}

async fn get_release_dates(State(_pool): State<PgPool>) -> Json<Vec<String>> {
    Json(vec![])
}

async fn delete_tasks(State(pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    task_repo.delete_completed().await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_page_hashes(
    State(pool): State<PgPool>,
    Query(params): Query<ReferentialParams>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let repo = PageHashRepository::new(pool);
    let limit = params.size as i64;
    let offset = (params.page * params.size) as i64;
    
    match repo.find_all(limit, offset).await {
        Ok((hashes, total)) => {
            Ok(Json(serde_json::json!({
                "content": hashes.into_iter().map(|h| serde_json::json!({
                    "hash": h.hash,
                    "size": h.size,
                    "action": h.action,
                    "deleteCount": h.delete_count,
                })).collect::<Vec<_>>(),
                "totalElements": total,
                "totalPages": 1,
                "number": params.page,
                "size": params.size,
            })))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_page_hash(
    State(pool): State<PgPool>,
    Path(hash): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let repo = PageHashRepository::new(pool);
    match repo.find_by_hash(&hash).await {
        Ok(Some(h)) => Ok(Json(serde_json::json!({
            "hash": h.hash,
            "size": h.size,
            "action": h.action,
        }))),
        Ok(None) => Ok(Json(serde_json::json!({}))),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_page_hash_thumbnail(
    State(_pool): State<PgPool>,
    Path(_hash): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response())
}

async fn delete_all_page_hash(
    State(_pool): State<PgPool>,
    Path(_hash): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn delete_match_page_hash(
    State(_pool): State<PgPool>,
    Path(_hash): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn get_unknown_page_hashes(
    State(_pool): State<PgPool>,
    Query(_params): Query<ReferentialParams>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({
        "content": [],
        "totalElements": 0,
        "totalPages": 0,
        "number": 0,
        "size": 20,
    })))
}

async fn get_unknown_page_hash_thumbnail(
    State(_pool): State<PgPool>,
    Path(_hash): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response())
}

async fn get_transient_books(
    State(_pool): State<PgPool>,
    Query(_params): Query<ReferentialParams>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let dir = std::env::current_dir().unwrap_or_default().join("transient");
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ["cbz", "cbr", "zip", "pdf", "epub"].contains(&ext.as_ref()) {
                    files.push(serde_json::json!({
                        "name": path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                        "path": path.to_string_lossy().to_string(),
                        "size": fs::metadata(&path).ok().map(|m| m.len()).unwrap_or(0),
                    }));
                }
            }
        }
    }
    Ok(Json(serde_json::json!({
        "content": files,
        "totalElements": files.len(),
        "totalPages": 1,
        "number": 0,
        "size": files.len(),
    })))
}

async fn create_transient_book(
    State(_pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
    Ok(Json(serde_json::json!({
        "id": Uuid::new_v4().to_string(),
        "name": name,
        "pages": 0,
    })))
}

async fn analyze_transient_book(
    State(_pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let _book_path = PathBuf::from(&id);
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn get_transient_book_page(
    State(_pool): State<PgPool>,
    Path((_id, _page)): Path<(String, i32)>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Page not found").into_response())
}

async fn get_fonts(
    State(_pool): State<PgPool>,
) -> Result<Json<Vec<String>>, axum::response::Response> {
    let fonts_dir = std::env::current_dir().unwrap_or_default().join("fonts");
    let mut families = Vec::new();
    if let Ok(entries) = fs::read_dir(&fonts_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    families.push(name.to_string());
                }
            }
        }
    }
    Ok(Json(families))
}

async fn get_font_css(
    State(_pool): State<PgPool>,
    Path(family): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let css = format!(
        "@font-face {{ font-family: '{}'; src: url('/api/v1/fonts/resource/{}/regular.ttf') format('truetype'); }}",
        family, family
    );
    Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/css")], css).into_response())
}

async fn get_font_file(
    State(_pool): State<PgPool>,
    Path((family, file)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    let path = std::env::current_dir().unwrap_or_default().join("fonts").join(&family).join(&file);
    if path.exists() {
        let data = fs::read(&path)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
        let content_type = if file.ends_with(".ttf") { "font/ttf" }
            else if file.ends_with(".woff") { "font/woff" }
            else if file.ends_with(".woff2") { "font/woff2" }
            else { "application/octet-stream" };
        Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, content_type)], data).into_response())
    } else {
        Err((axum::http::StatusCode::NOT_FOUND, "Font file not found").into_response())
    }
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/libraries", get(get_all_libraries))
        .route("/api/v1/libraries", post(create_library))
        .route("/api/v1/libraries/{id}", get(get_library))
        .route("/api/v1/libraries/{id}", delete(delete_library))
        .route("/api/v1/libraries/{id}", put(update_library))
        .route("/api/v1/libraries/{id}", patch(patch_library))
        .route("/api/v1/libraries/{id}/scan", post(scan_library))
        .route("/api/v1/libraries/{id}/analyze", post(analyze_library))
        .route("/api/v1/libraries/{id}/metadata/refresh", post(refresh_library_metadata))
        .route("/api/v1/libraries/{id}/empty-trash", post(empty_trash))
        .route("/api/v1/filesystem", post(filesystem))
        .route("/api/v1/authors", get(get_referential))
        .route("/api/v2/authors", get(get_referential))
        .route("/api/v1/authors/names", get(get_referential))
        .route("/api/v1/authors/roles", get(get_referential))
        .route("/api/v1/genres", get(get_referential))
        .route("/api/v1/tags", get(get_referential))
        .route("/api/v1/tags/book", get(get_referential))
        .route("/api/v1/tags/series", get(get_referential))
        .route("/api/v1/languages", get(get_referential))
        .route("/api/v1/publishers", get(get_referential))
        .route("/api/v1/age-ratings", get(get_referential))
        .route("/api/v1/sharing-labels", get(get_sharing_labels))
        .route("/api/v1/series/release-dates", get(get_release_dates))
        .route("/api/v1/tasks", delete(delete_tasks))
        .route("/api/v1/page-hashes", get(get_page_hashes))
        .route("/api/v1/page-hashes/{hash}", get(get_page_hash))
        .route("/api/v1/page-hashes/{hash}/thumbnail", get(get_page_hash_thumbnail))
        .route("/api/v1/page-hashes/{hash}/delete-all", post(delete_all_page_hash))
        .route("/api/v1/page-hashes/{hash}/delete-match", post(delete_match_page_hash))
        .route("/api/v1/page-hashes/unknown", get(get_unknown_page_hashes))
        .route("/api/v1/page-hashes/unknown/{hash}/thumbnail", get(get_unknown_page_hash_thumbnail))
        .route("/api/v1/transient-books", get(get_transient_books))
        .route("/api/v1/transient-books", post(create_transient_book))
        .route("/api/v1/transient-books/{id}/analyze", post(analyze_transient_book))
        .route("/api/v1/transient-books/{id}/pages/{page}", get(get_transient_book_page))
        .route("/api/v1/fonts/families", get(get_fonts))
        .route("/api/v1/fonts/resource/{family}/css", get(get_font_css))
        .route("/api/v1/fonts/resource/{family}/{file}", get(get_font_file))
}