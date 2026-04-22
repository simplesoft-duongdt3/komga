use axum::{
    extract::{Path, Query, State, Multipart},
    routing::{get, patch, delete, put},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use std::path::PathBuf;
use std::fs;

use crate::api::dto::{BookDto, BookPageDto, PageDto, ReadProgressDto, ReadProgressUpdateRequest, BookMetadataDto};
use crate::domain::repository::{BookRepository, ReadProgressRepository};
use crate::domain::model::read_progress::ReadProgress;
use crate::domain::model::book::BookMetadata;
use crate::infrastructure::mediacontainer::{cbz::CbzExtractor, epub::EpubExtractor, pdf::PdfExtractor, BookExtractor};
use crate::infrastructure::mediacontainer::image::ImageProcessor;

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
        Ok(Some(book)) => {
            let book_path = std::path::PathBuf::from(&book.name);
            if !book_path.exists() {
                return Err((axum::http::StatusCode::NOT_FOUND, "Book file not found").into_response());
            }
            
            let ext = book_path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            
            let analysis = if ext == "cbz" || ext == "zip" {
                CbzExtractor::new().get_pages(&book_path)
            } else if ext == "epub" {
                EpubExtractor::new().get_pages(&book_path)
            } else if ext == "pdf" {
                PdfExtractor::new().get_pages(&book_path)
            } else {
                return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported format").into_response());
            };
            
            match analysis {
                Ok(analysis) => {
                    let pages: Vec<PageDto> = analysis.pages.iter().map(|p| PageDto {
                        number: p.number,
                        file_name: p.file_name.clone(),
                        media_type: p.media_type.clone(),
                        width: p.width,
                        height: p.height,
                        size_bytes: p.size_bytes,
                    }).collect();
                    Ok(Json(pages))
                }
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            }
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_page(
    State(_pool): State<PgPool>,
    Path((book_id, page_number)): Path<(String, i32)>,
    Query(params): Query<PageStreamParams>,
) -> Result<axum::response::Response, axum::response::Response> {
    let book_repo = BookRepository::new(_pool.clone());
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    
    let book = match book_repo.find_by_id(uuid).await {
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
    
    if !["cbz", "zip", "pdf", "epub"].contains(&ext.as_str()) {
        return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported format").into_response());
    }
    
    let result: Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> = if ext == "cbz" || ext == "zip" {
        CbzExtractor::new().get_page_content(&book_path, page_number)
    } else if ext == "epub" {
        EpubExtractor::new().get_page_content(&book_path, page_number)
    } else if ext == "pdf" {
        PdfExtractor::new().get_page_content(&book_path, page_number)
    } else {
        return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported format").into_response());
    };
    
    let bytes = match result {
        Ok(b) => b,
        Err(e) => return Err((axum::http::StatusCode::NOT_FOUND, format!("Failed to extract page: {}", e)).into_response()),
    };
    
    let media_type = if ext == "pdf" { "image/png" } else if ext == "epub" { "application/xhtml+xml" } else { "image/jpeg" };
    
    let response = (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, media_type)],
        bytes,
    );
    
    Ok(response.into_response())
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
    
    if !["cbz", "zip", "pdf", "epub"].contains(&ext.as_str()) {
        return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Thumbnail not supported for this format").into_response());
    }
    
    let page_result: Result<Vec<u8>, _> = if ext == "cbz" || ext == "zip" {
        CbzExtractor::new().get_page_content(&book_path, page_number)
    } else if ext == "pdf" {
        PdfExtractor::new().get_page_content(&book_path, page_number)
    } else {
        return Err((axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE, "Thumbnail not supported for EPUB").into_response());
    };
    
    let page_data = match page_result {
        Ok(data) => data,
        Err(e) => return Err((axum::http::StatusCode::NOT_FOUND, format!("Failed to extract page: {}", e)).into_response()),
    };
    
    let processor = ImageProcessor::new();
    let thumbnail = match processor.generate_thumbnail(&page_data, 300) {
        Ok(thumb) => thumb,
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to generate thumbnail: {}", e)).into_response()),
    };
    
    let response = (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        thumbnail,
    );
    
    Ok(response.into_response())
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

async fn get_book_cover(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    
    let book = match repo.find_by_id(uuid).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let cover_path = book.cover_file_name
        .map(PathBuf::from)
        .filter(|p| p.exists());
    
    match cover_path {
        Some(path) => {
            let data = match fs::read(&path) {
                Ok(d) => d,
                Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            };
            let media_type = path.extension()
                .map(|e| match e.to_string_lossy().to_lowercase().as_str() {
                    "png" => "image/png",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "image/jpeg",
                })
                .unwrap_or("image/jpeg");
            Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, media_type)], data).into_response())
        }
        None => Err((axum::http::StatusCode::NOT_FOUND, "Cover not found").into_response()),
    }
}

async fn upload_book_cover(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = BookRepository::new(pool.clone());
    
    let book = match repo.find_by_id(uuid).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => return Err((axum::http::StatusCode::BAD_REQUEST, "No file provided").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let bytes = match field.bytes().await {
        Ok(b) => b,
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let covers_dir = std::env::current_dir().unwrap_or_default().join("covers");
    if let Err(e) = fs::create_dir_all(&covers_dir) {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    let cover_path = covers_dir.join(format!("{}.jpg", book.id));
    if let Err(e) = fs::write(&cover_path, &bytes) {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    let repo = BookRepository::new(pool.clone());
    if let Err(e) = repo.update_cover(&uuid, cover_path.to_string_lossy().to_string()).await {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_book_cover(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    
    let book = match repo.find_by_id(uuid).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    if let Some(cover_path) = book.cover_file_name {
        let path = PathBuf::from(cover_path);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
    
    if let Err(e) = repo.update_cover(&uuid, String::new()).await {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_book_metadata(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<BookMetadataDto>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = BookRepository::new(pool.clone());
    
    let book = match repo.find_by_id(uuid).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let now = chrono::Utc::now();
    let mut metadata = book.metadata.unwrap_or(BookMetadata {
        created_date: now,
        last_modified_date: now,
        number: String::new(),
        number_lock: false,
        number_sort: 0.0,
        number_sort_lock: false,
        release_date: None,
        release_date_lock: false,
        summary: String::new(),
        summary_lock: false,
        title: book.name.clone(),
        title_lock: false,
        authors: vec![],
        authors_lock: false,
        tags: vec![],
        tags_lock: false,
        book_id: uuid,
        isbn: String::new(),
        isbn_lock: false,
        links: vec![],
        links_lock: false,
    });
    
    metadata.last_modified_date = now;
    
    if let Some(number) = req.number {
        metadata.number = number;
    }
    if let Some(number_sort) = req.number_sort {
        metadata.number_sort = number_sort;
    }
    if let Some(summary) = req.summary {
        metadata.summary = summary;
    }
    if let Some(title) = req.title {
        metadata.title = title;
    }
    if let Some(authors) = req.authors {
        metadata.authors = authors.iter().map(|a| crate::domain::model::book::Author {
            name: a.name.clone(),
            role: a.role.clone(),
        }).collect();
    }
    if let Some(tags) = req.tags {
        metadata.tags = tags;
    }
    if let Some(isbn) = req.isbn {
        metadata.isbn = isbn;
    }
    
    repo.update_metadata(&uuid, &metadata).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
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
        .route("/api/v1/books/{id}/cover", get(get_book_cover))
        .route("/api/v1/books/{id}/cover", put(upload_book_cover))
        .route("/api/v1/books/{id}/cover", delete(delete_book_cover))
        .route("/api/v1/books/{id}/metadata", patch(update_book_metadata))
}