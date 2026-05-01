use axum::{
    extract::{Path, Query, State},
    routing::{get, post, patch, delete, put},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use std::io::{Write, Cursor};
use std::path::PathBuf;
use std::fs;
use zip::write::FileOptions;
use zip::CompressionMethod;

use crate::api::dto::{ReadListDto, ReadListPageDto, CreateReadListRequest, UpdateReadListRequest, BookDto, BookPageDto};
use crate::domain::model::readlist::ReadList;
use crate::domain::repository::{ReadListRepository, BookRepository, ReadProgressRepository, ThumbnailRepository};
use crate::domain::model::read_progress::ReadProgress;

#[derive(Deserialize)]
struct PageParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_size")]
    size: usize,
}

fn default_page() -> usize { 0 }
fn default_size() -> usize { 20 }

async fn get_all_readlists(
    State(pool): State<PgPool>,
    Query(_params): Query<PageParams>,
) -> Result<Json<ReadListPageDto>, axum::response::Response> {
    let repo = ReadListRepository::new(pool);
    
    match repo.find_all().await {
        Ok(readlists) => {
            let total = readlists.len();
            let readlists: Vec<ReadListDto> = readlists.into_iter().map(|r| r.into()).collect();
            Ok(Json(ReadListPageDto::new(readlists, total, 0, total,)))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_readlist(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<ReadListDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = ReadListRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(readlist)) => Ok(Json(readlist.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "ReadList not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn create_readlist(
    State(pool): State<PgPool>,
    Json(req): Json<CreateReadListRequest>,
) -> Result<Json<ReadListDto>, axum::response::Response> {
    let mut readlist = ReadList::new(req.name);
    if let Some(summary) = req.summary {
        readlist.summary = summary;
    }
    if let Some(ordered) = req.ordered {
        readlist.ordered = ordered;
    }
    
    let repo = ReadListRepository::new(pool);
    
    match repo.create(&readlist).await {
        Ok(created) => Ok(Json(created.into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn update_readlist(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateReadListRequest>,
) -> Result<Json<ReadListDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = ReadListRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(mut readlist)) => {
            if let Some(name) = req.name {
                readlist.name = name;
            }
            if let Some(summary) = req.summary {
                readlist.summary = summary;
            }
            if let Some(ordered) = req.ordered {
                readlist.ordered = ordered;
            }
            
            match repo.update(&readlist).await {
                Ok(updated) => Ok(Json(updated.into())),
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            }
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "ReadList not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn delete_readlist(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<(), axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = ReadListRepository::new(pool);
    
    match repo.delete(uuid).await {
        Ok(_) => Ok(()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/readlists", get(get_all_readlists))
        .route("/api/v1/readlists", post(create_readlist))
        .route("/api/v1/readlists/:id", get(get_readlist))
        .route("/api/v1/readlists/:id", patch(update_readlist))
        .route("/api/v1/readlists/:id", delete(delete_readlist))
        .route("/api/v1/readlists/match/comicrack", post(match_comicrack))
        .route("/api/v1/readlists/:id/books", get(get_readlist_books))
        .route("/api/v1/readlists/:id/books/:bookId/next", get(get_next_book))
        .route("/api/v1/readlists/:id/books/:bookId/previous", get(get_previous_book))
        .route("/api/v1/readlists/:id/file", get(get_readlist_file))
        .route("/api/v1/readlists/:id/read-progress/tachiyomi", get(get_tachiyomi_progress))
        .route("/api/v1/readlists/:id/read-progress/tachiyomi", put(update_tachiyomi_progress))
        .route("/api/v1/readlists/:id/thumbnail", get(get_readlist_thumbnail))
        .route("/api/v1/readlists/:id/thumbnails", get(get_readlist_thumbnails))
        .route("/api/v1/readlists/:id/thumbnails", post(create_readlist_thumbnail))
        .route("/api/v1/readlists/:id/thumbnails/:thumbnailId", get(get_readlist_thumbnail_by_id))
        .route("/api/v1/readlists/:id/thumbnails/:thumbnailId", delete(delete_readlist_thumbnail))
        .route("/api/v1/readlists/:id/thumbnails/:thumbnailId/selected", put(mark_readlist_thumbnail_selected))
}

async fn match_comicrack(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ReadListDto>, axum::response::Response> {
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("ComicRack List");
    let readlist = ReadList::new(name.to_string());
    
    let repo = ReadListRepository::new(pool);
    match repo.create(&readlist).await {
        Ok(created) => Ok(Json(created.into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_readlist_books(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Query(_params): Query<PageParams>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool.clone());
    let book_repo = BookRepository::new(pool.clone());
    
    match readlist_repo.get_books(uuid).await {
        Ok(book_ids) => {
            let mut books = Vec::new();
            for bid_str in book_ids {
                if let Ok(bid) = Uuid::parse_str(&bid_str) {
                    if let Ok(Some(book)) = book_repo.find_by_id(bid).await {
                        books.push(BookDto::from(book));
                    }
                }
            }
            let total = books.len();
            Ok(Json(BookPageDto::new(books, total, 0, total.max(1),)))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_next_book(
    State(pool): State<PgPool>,
    Path((id, book_id)): Path<(String, String)>,
) -> Result<Json<Option<BookDto>>, axum::response::Response> {
    let readlist_uuid = Uuid::parse_str(&id).unwrap_or_default();
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool.clone());
    let book_repo = BookRepository::new(pool);
    
    let book_ids = readlist_repo.get_books(readlist_uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    let pos = book_ids.iter().position(|bid| bid == &book_id);
    if let Some(idx) = pos {
        if idx + 1 < book_ids.len() {
            let next_id = &book_ids[idx + 1];
            if let Ok(next_uuid) = Uuid::parse_str(next_id) {
                if let Ok(Some(book)) = book_repo.find_by_id(next_uuid).await {
                    return Ok(Json(Some(BookDto::from(book))));
                }
            }
        }
    }
    Ok(Json(None))
}

async fn get_previous_book(
    State(pool): State<PgPool>,
    Path((id, book_id)): Path<(String, String)>,
) -> Result<Json<Option<BookDto>>, axum::response::Response> {
    let readlist_uuid = Uuid::parse_str(&id).unwrap_or_default();
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool.clone());
    let book_repo = BookRepository::new(pool);
    
    let book_ids = readlist_repo.get_books(readlist_uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    let pos = book_ids.iter().position(|bid| bid == &book_id);
    if let Some(idx) = pos {
        if idx > 0 {
            let prev_id = &book_ids[idx - 1];
            if let Ok(prev_uuid) = Uuid::parse_str(prev_id) {
                if let Ok(Some(book)) = book_repo.find_by_id(prev_uuid).await {
                    return Ok(Json(Some(BookDto::from(book))));
                }
            }
        }
    }
    Ok(Json(None))
}

async fn get_readlist_file(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool.clone());
    let book_repo = BookRepository::new(pool);
    
    let readlist = match readlist_repo.find_by_id(uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "ReadList not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let book_ids = match readlist_repo.get_books(uuid).await {
        Ok(ids) => ids,
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    if book_ids.is_empty() {
        return Err((axum::http::StatusCode::NOT_FOUND, "No books in readlist").into_response());
    }
    
    let mut zip_buffer = Cursor::new(Vec::new());
    let mut zip_writer = zip::ZipWriter::new(&mut zip_buffer);
    let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
    
    for (i, bid_str) in book_ids.iter().enumerate() {
        if let Ok(bid) = Uuid::parse_str(bid_str) {
            if let Ok(Some(book)) = book_repo.find_by_id(bid).await {
                let book_path = PathBuf::from(&book.url);
                if let Ok(data) = fs::read(&book_path) {
                    let name = book_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| format!("{}.cbz", book.id));
                    let numbered_name = format!("{:03}_{}", i + 1, name);
                    if let Err(e) = zip_writer.start_file(&numbered_name, options) {
                        tracing::warn!("Failed to add {} to ZIP: {}", numbered_name, e);
                        continue;
                    }
                    if let Err(e) = zip_writer.write_all(&data) {
                        tracing::warn!("Failed to write {} to ZIP: {}", numbered_name, e);
                        continue;
                    }
                }
            }
        }
    }
    
    if let Err(e) = zip_writer.finish() {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    let data = zip_buffer.into_inner();
    
    let filename = format!("{}.zip", readlist.name.replace(' ', "_"));
    let disposition = format!("attachment; filename=\"{}\"", filename);
    
    Ok((
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "application/zip"),
            (axum::http::header::CONTENT_DISPOSITION, disposition.as_str()),
        ],
        data,
    ).into_response())
}

async fn get_tachiyomi_progress(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let readlist_uuid = Uuid::parse_str(&id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool.clone());
    let progress_repo = ReadProgressRepository::new(pool);
    let user_id = Uuid::nil();

    let book_ids = readlist_repo.get_books(readlist_uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut read_chapters = Vec::new();
    for bid_str in &book_ids {
        if let Ok(bid) = Uuid::parse_str(bid_str) {
            if let Ok(Some(progress)) = progress_repo.find_by_book_and_user(bid, user_id).await {
                if progress.completed || progress.page > 0 {
                    read_chapters.push(bid_str.clone());
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "categoryId": id,
        "readChapters": read_chapters,
    })))
}

async fn update_tachiyomi_progress(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let readlist_uuid = Uuid::parse_str(&id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool.clone());
    let progress_repo = ReadProgressRepository::new(pool);
    let user_id = Uuid::nil();

    let read_chapters: Vec<String> = body.get("readChapters")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let book_ids = readlist_repo.get_books(readlist_uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    for bid_str in &book_ids {
        if let Ok(bid) = Uuid::parse_str(bid_str) {
            let completed = read_chapters.contains(bid_str);
            let page = if completed { 1 } else { 0 };
            let progress = ReadProgress::new(bid, user_id, page, completed);
            let _ = progress_repo.upsert(&progress).await;
        }
    }

    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_readlist_thumbnail(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = ThumbnailRepository::new(pool);
    
    let thumbnails = repo.find_readlist_thumbnails(uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    if let Some(thumbnail) = thumbnails.iter().find(|t| t.selected).or_else(|| thumbnails.first()) {
        if let Some(ref data) = thumbnail.data {
            return Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, thumbnail.media_type.as_str())], data.clone()).into_response());
        }
    }
    
    Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response())
}

async fn get_readlist_thumbnails(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = ThumbnailRepository::new(pool);
    
    let thumbnails = repo.find_readlist_thumbnails(uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok(Json(thumbnails.into_iter().map(|t| {
        serde_json::json!({
            "id": t.id,
            "url": t.url,
            "selected": t.selected,
            "type": t.thumbnail_type,
            "width": t.width,
            "height": t.height,
            "mediaType": t.media_type,
            "fileSize": t.file_size,
        })
    }).collect()))
}

async fn create_readlist_thumbnail(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    Ok(Json(serde_json::json!({"id": uuid.to_string(), "selected": false})))
}

async fn get_readlist_thumbnail_by_id(
    State(pool): State<PgPool>,
    Path((id, thumbnail_id)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    let thumb_uuid = Uuid::parse_str(&thumbnail_id).unwrap_or_default();
    let repo = ThumbnailRepository::new(pool);
    
    let thumbnails = repo.find_readlist_thumbnails(Uuid::parse_str(&id).unwrap_or_default()).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    if let Some(thumbnail) = thumbnails.into_iter().find(|t| t.id == thumb_uuid.to_string()) {
        if let Some(ref data) = thumbnail.data {
            return Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, thumbnail.media_type.as_str())], data.clone()).into_response());
        }
    }
    
    Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response())
}

async fn delete_readlist_thumbnail(
    State(pool): State<PgPool>,
    Path((_id, thumbnail_id)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    let thumb_uuid = Uuid::parse_str(&thumbnail_id).unwrap_or_default();
    let repo = ThumbnailRepository::new(pool);
    repo.delete_readlist_thumbnail(thumb_uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn mark_readlist_thumbnail_selected(
    State(pool): State<PgPool>,
    Path((id, thumbnail_id)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    let readlist_uuid = Uuid::parse_str(&id).unwrap_or_default();
    let thumb_uuid = Uuid::parse_str(&thumbnail_id).unwrap_or_default();
    let repo = ThumbnailRepository::new(pool);
    repo.mark_readlist_thumbnail_selected(readlist_uuid, thumb_uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}