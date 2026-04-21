use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, delete},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use std::path::PathBuf;
use std::fs;

use crate::api::dto::{BookDto, BookPageDto, PageDto, ReadProgressDto, ReadProgressUpdateRequest};
use crate::domain::repository::{BookRepository, ReadProgressRepository};
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

async fn get_books_by_series(
    State(pool): State<PgPool>,
    Path(series_id): Path<String>,
    Query(params): Query<PageParams>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let series_uuid = Uuid::parse_str(&series_id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    
    match repo.find_by_series(series_uuid).await {
        Ok(books_list) => {
            let total = books_list.len();
            let books: Vec<BookDto> = books_list.into_iter().map(|b| b.into()).collect();
            Ok(Json(BookPageDto {
                content: books,
                total_elements: total,
                total_pages: 1,
                number: params.page,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<BookDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(book)) => Ok(Json(book.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_pages(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<Vec<PageDto>>, axum::response::Response> {
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(_book)) => {
            Ok(Json(vec![]))
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_page(
    State(_pool): State<PgPool>,
    Path((book_id, page_number)): Path<(String, i32)>,
    Query(params): Query<PageStreamParams>,
) -> impl axum::response::IntoResponse {
    let book_repo = BookRepository::new(_pool.clone());
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    
    let book = match book_repo.find_by_id(uuid).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Book not found")),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };
    
    let book_path = PathBuf::from(&book.name);
    if !book_path.exists() {
        return Err((axum::http::StatusCode::NOT_FOUND, "Book file not found"));
    }
    
    let ext = book_path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    
    if !["cbz", "zip", "pdf", "epub"].contains(&ext.as_str()) {
        return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported format"));
    }
    
    if let Ok(bytes) = fs::read(&book_path) {
        let media_type = if ext == "pdf" { "image/png" } else { "image/jpeg" };
        return Ok((axum::http::StatusCode::OK, bytes).into_response());
    }
    
    Err((axum::http::StatusCode::NOT_FOUND, "Could not read book"))
}

#[derive(Deserialize, Default)]
struct PageStreamParams {
    convert: Option<String>,
}

async fn get_book_thumbnail(
    State(pool): State<PgPool>,
    Path((book_id, page_number)): Path<(String, i32)>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    
    let book = match repo.find_by_id(uuid).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let book_path = PathBuf::from(&book.name);
    if !book_path.exists() {
        return Err((axum::http::StatusCode::NOT_FOUND, "Book file not found").into_response());
    }
    
    let ext = book_path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    
    if ext != "cbz" && ext != "zip" {
        return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Thumbnail only supported for CBZ").into_response());
    }
    
    if let Ok(bytes) = fs::read(&book_path) {
        return Ok((axum::http::StatusCode::OK, bytes).into_response());
    }
    
    Err((axum::http::StatusCode::NOT_FOUND, "Could not read book").into_response())
}

async fn get_book_read_progress(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<Option<ReadProgressDto>>, axum::response::Response> {
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let user_id = Uuid::nil();
    
    let repo = ReadProgressRepository::new(pool);
    
    match repo.find_by_book_and_user(book_uuid, user_id).await {
        Ok(Some(progress)) => Ok(Json(Some(ReadProgressDto {
            book_id: progress.book_id.to_string(),
            user_id: progress.user_id.to_string(),
            page: progress.page,
            completed: progress.completed,
            read_date: progress.read_date.map(|d| d.to_rfc3339()),
        }))),
        Ok(None) => Ok(Json(None)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn update_book_read_progress(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
    Json(req): Json<ReadProgressUpdateRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let user_id = Uuid::nil();
    
    let page = req.page.unwrap_or(1);
    let completed = req.completed.unwrap_or(false);
    
    let progress = ReadProgress::new(book_uuid, user_id, page, completed);
    let repo = ReadProgressRepository::new(pool);
    
    match repo.upsert(&progress).await {
        Ok(_) => Ok((axum::http::StatusCode::NO_CONTENT, "").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn delete_book_read_progress(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let user_id = Uuid::nil();
    
    let repo = ReadProgressRepository::new(pool);
    
    match repo.delete(book_uuid, user_id).await {
        Ok(_) => Ok((axum::http::StatusCode::NO_CONTENT, "").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/series/{seriesId}/books", get(get_books_by_series))
        .route("/api/v1/books/{id}", get(get_book))
        .route("/api/v1/books/{bookId}/pages", get(get_book_pages))
        .route("/api/v1/books/{bookId}/pages/{pageNumber}", get(get_book_page))
        .route("/api/v1/books/{bookId}/pages/{pageNumber}/thumbnail", get(get_book_thumbnail))
        .route("/api/v1/books/{bookId}/read-progress", get(get_book_read_progress))
        .route("/api/v1/books/{bookId}/read-progress", patch(update_book_read_progress))
        .route("/api/v1/books/{bookId}/read-progress", delete(delete_book_read_progress))
}