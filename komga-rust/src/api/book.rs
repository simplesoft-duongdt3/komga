use axum::{
    extract::{Path, Query, State, Multipart},
    routing::{get, patch, delete, put, post},
    Router, Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use std::path::PathBuf;
use std::fs;

use crate::api::dto::{BookDto, BookPageDto, PageDto, ReadProgressDto, ReadProgressUpdateRequest, BookMetadataDto, ReadListDto};
use crate::domain::repository::{BookRepository, ReadProgressRepository, TaskRepository, ReadListRepository};
use crate::domain::model::read_progress::ReadProgress;
use crate::domain::model::book::BookMetadata;
use crate::domain::model::task::{Task, TaskData, TaskType};
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
            Ok(Json(BookPageDto::new(books, total, params.page, params.size,)))
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
                        size: None,
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
            page: progress.page,
            completed: progress.completed,
            read_date: progress.read_date.map(|d| d.to_rfc3339()),
            device_id: None,
            device_name: None,
            created: None,
            last_modified: None,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BookSearchRequest {
    condition: Option<SearchCondition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchCondition {
    all_of: Option<Vec<SearchClause>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchClause {
    library_id: Option<LibraryIdClause>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryIdClause {
    value: String,
}

async fn list_books(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let repo = BookRepository::new(pool);
    match repo.find_all(params.size, params.page * params.size).await {
        Ok(books) => {
            let total = books.len();
            Ok(Json(BookPageDto::new(books.into_iter().map(|b| b.into()).collect(), total, params.page, params.size,)))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn list_books_post(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
    Json(search): Json<BookSearchRequest>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let repo = BookRepository::new(pool.clone());
    
    let library_id = search.condition
        .and_then(|c| c.all_of)
        .and_then(|clauses| clauses.into_iter().find_map(|c| c.library_id))
        .map(|lid| Uuid::parse_str(&lid.value).unwrap_or_default());
    
    match library_id {
        Some(lid) => {
            let total = repo.count_by_library(lid).await.unwrap_or(0) as usize;
            let books = repo.find_by_library_paginated(lid, params.size, params.page * params.size).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
            Ok(Json(BookPageDto::new(books.into_iter().map(|b| b.into()).collect(), total, params.page, params.size)))
        }
        None => {
            match repo.find_all(params.size, params.page * params.size).await {
                Ok(books) => {
                    let total = books.len();
                    Ok(Json(BookPageDto::new(books.into_iter().map(|b| b.into()).collect(), total, params.page, params.size)))
                }
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            }
        }
    }
}

async fn get_books_latest(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let repo = BookRepository::new(pool);
    match repo.find_latest(params.size).await {
        Ok(books) => {
            let total = books.len();
            Ok(Json(BookPageDto::new(books.into_iter().map(|b| b.into()).collect(), total, 0, params.size,)))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_books_ondeck(
    State(pool): State<PgPool>,
    Query(_params): Query<PageParams>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let repo = BookRepository::new(pool);
    match repo.find_ondeck(_params.size).await {
        Ok(books) => {
            let total = books.len();
            Ok(Json(BookPageDto::new(books.into_iter().map(|b| b.into()).collect(), total, 0, _params.size,)))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_books_duplicates(
    State(pool): State<PgPool>,
    Query(_params): Query<PageParams>,
) -> Result<Json<BookPageDto>, axum::response::Response> {
    let repo = BookRepository::new(pool);
    match repo.find_duplicates().await {
        Ok(books) => {
            let total = books.len();
            Ok(Json(BookPageDto::new(books.into_iter().map(|b| b.into()).collect(), total, 0, _params.size,)))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_previous(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<Option<BookDto>>, axum::response::Response> {
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    match repo.find_by_id(uuid).await {
        Ok(Some(book)) => {
            match repo.find_previous_in_series(&book.series_id, book.number).await {
                Ok(Some(prev)) => Ok(Json(Some(prev.into()))),
                Ok(None) => Ok(Json(None)),
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            }
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_next(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<Option<BookDto>>, axum::response::Response> {
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    match repo.find_by_id(uuid).await {
        Ok(Some(book)) => {
            match repo.find_next_in_series(&book.series_id, book.number).await {
                Ok(Some(next)) => Ok(Json(Some(next.into()))),
                Ok(None) => Ok(Json(None)),
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            }
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_readlists(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<Vec<ReadListDto>>, axum::response::Response> {
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let readlist_repo = ReadListRepository::new(pool);
    
    match readlist_repo.find_by_book(book_uuid).await {
        Ok(readlists) => Ok(Json(readlists.into_iter().map(|r| r.into()).collect())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_manifest(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let repo = BookRepository::new(pool);
    match repo.find_by_id(uuid).await {
        Ok(Some(book)) => {
            Ok(Json(serde_json::json!({
                "@context": "https://readium.org/webpub-manifest/context.jsonld",
                "metadata": {
                    "@type": "http://schema.org/Book",
                    "title": book.name,
                    "identifier": book.id.to_string(),
                },
                "readingOrder": [],
                "resources": [],
            })))
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Book not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_book_positions(
    State(_pool): State<PgPool>,
    Path(_book_id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({
        "positionList": []
    })))
}

async fn analyze_book(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::AnalyzeBook,
        TaskData::AnalyzeBook { book_id: book_id.clone() },
        4,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn refresh_book_metadata(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::RefreshBookMetadata,
        TaskData::RefreshBookMetadata { book_id: book_id.clone(), capabilities: vec![] },
        4,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

#[derive(Deserialize)]
struct BookImportDto {
    #[serde(rename = "sourceFile")]
    source_file: String,
    #[serde(rename = "seriesId")]
    series_id: String,
    #[serde(rename = "destinationName")]
    destination_name: Option<String>,
    #[serde(rename = "upgradeBookId")]
    upgrade_book_id: Option<String>,
}

#[derive(Deserialize)]
struct BookImportBatchDto {
    books: Vec<BookImportDto>,
    #[serde(rename = "copyMode")]
    copy_mode: Option<String>,
}

async fn import_books(
    State(pool): State<PgPool>,
    Json(batch): Json<BookImportBatchDto>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    for book in &batch.books {
        let task = Task::new(
            TaskType::ImportBook,
            TaskData::ImportBook {
                source_file: book.source_file.clone(),
                series_id: book.series_id.clone(),
                copy_mode: batch.copy_mode.clone().unwrap_or_else(|| "HARDLINK".to_string()),
            },
            4,
        );
        task_repo.create(&task).await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    }
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn delete_book_file(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::DeleteBook,
        TaskData::DeleteBook { book_id: book_id.clone() },
        4,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn regenerate_book_thumbnails(
    State(pool): State<PgPool>,
    Query(_params): Query<RegenerateParams>,
) -> Result<axum::response::Response, axum::response::Response> {
    let task_repo = TaskRepository::new(pool.clone());
    let task = Task::new(
        TaskType::FindBookThumbnailsToRegenerate,
        TaskData::FindBookThumbnailsToRegenerate {
            for_bigger_result_only: _params.for_bigger_result_only,
        },
        2,
    );
    task_repo.create(&task).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

#[derive(Deserialize)]
struct RegenerateParams {
    #[serde(rename = "for_bigger_result_only", default)]
    for_bigger_result_only: bool,
}

async fn get_book_file(
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
    
    let book_path = PathBuf::from(&book.url);
    if !book_path.exists() {
        return Err((axum::http::StatusCode::NOT_FOUND, "Book file not found").into_response());
    }
    
    let data = match fs::read(&book_path) {
        Ok(d) => d,
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let media_type = book_path.extension()
        .map(|e| match e.to_string_lossy().to_lowercase().as_str() {
            "cbz" => "application/zip",
            "zip" => "application/zip",
            "pdf" => "application/pdf",
            "epub" => "application/epub+zip",
            _ => "application/octet-stream",
        })
        .unwrap_or("application/octet-stream");
    
    let filename = book_path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "book".to_string());
    
    let disposition = format!("attachment; filename=\"{}\"", filename);
    
    Ok((
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, media_type),
            (axum::http::header::CONTENT_DISPOSITION, disposition.as_str()),
        ],
        data,
    ).into_response())
}

async fn get_book_thumbnail_single(
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
            Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "image/jpeg")], data).into_response())
        }
        None => Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response()),
    }
}

async fn get_book_page_raw(
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
    
    let book_path = PathBuf::from(&book.url);
    if !book_path.exists() {
        return Err((axum::http::StatusCode::NOT_FOUND, "Book file not found").into_response());
    }
    
    let ext = book_path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    
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
    
    Ok((axum::http::StatusCode::OK, [(axum::http::header::CONTENT_TYPE, media_type)], bytes).into_response())
}

async fn get_book_progression(
    State(pool): State<PgPool>,
    Path(book_id): Path<String>,
) -> Result<Json<Option<ReadProgressDto>>, axum::response::Response> {
    let book_uuid = Uuid::parse_str(&book_id).unwrap_or_default();
    let user_id = Uuid::nil();
    let repo = ReadProgressRepository::new(pool);
    
    match repo.find_by_book_and_user(book_uuid, user_id).await {
        Ok(Some(progress)) => Ok(Json(Some(ReadProgressDto {
            page: progress.page,
            completed: progress.completed,
            read_date: progress.read_date.map(|d| d.to_rfc3339()),
            device_id: None,
            device_name: None,
            created: None,
            last_modified: None,
        }))),
        Ok(None) => Ok(Json(None)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn update_book_progression(
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

async fn get_book_resource(
    State(_pool): State<PgPool>,
    Path((_book_id, _resource)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Resource not found").into_response())
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/books", get(list_books))
        .route("/api/v1/books/list", post(list_books_post))
        .route("/api/v1/books/latest", get(get_books_latest))
        .route("/api/v1/books/ondeck", get(get_books_ondeck))
        .route("/api/v1/books/duplicates", get(get_books_duplicates))
        .route("/api/v1/books/import", post(import_books))
        .route("/api/v1/books/thumbnails", put(regenerate_book_thumbnails))
        .route("/api/v1/books/:id", get(get_book))
        .route("/api/v1/books/:id/file", get(get_book_file))
        .route("/api/v1/books/:id/file", delete(delete_book_file))
        .route("/api/v1/books/:id/thumbnail", get(get_book_thumbnail_single))
        .route("/api/v1/books/:id/metadata", patch(update_book_metadata))
        .route("/api/v1/books/:id/metadata/refresh", post(refresh_book_metadata))
        .route("/api/v1/books/:id/analyze", post(analyze_book))
        .route("/api/v1/books/:id/positions", get(get_book_positions))
        .route("/api/v1/books/:id/manifest", get(get_book_manifest))
        .route("/api/v1/books/:id/manifest/divina", get(get_book_manifest))
        .route("/api/v1/books/:id/manifest/epub", get(get_book_manifest))
        .route("/api/v1/books/:id/manifest/pdf", get(get_book_manifest))
        .route("/api/v1/books/:id/resource/:resource", get(get_book_resource))
        .route("/api/v1/books/:id/next", get(get_book_next))
        .route("/api/v1/books/:id/previous", get(get_book_previous))
        .route("/api/v1/books/:id/progression", get(get_book_progression))
        .route("/api/v1/books/:id/progression", put(update_book_progression))
        .route("/api/v1/books/:id/readlists", get(get_book_readlists))
        .route("/api/v1/books/:id/cover", get(get_book_cover))
        .route("/api/v1/books/:id/cover", put(upload_book_cover))
        .route("/api/v1/books/:id/cover", delete(delete_book_cover))
        .route("/api/v1/books/:id/pages", get(get_book_pages))
        .route("/api/v1/books/:id/pages/:pageNumber", get(get_book_page))
        .route("/api/v1/books/:id/pages/:pageNumber/raw", get(get_book_page_raw))
        .route("/api/v1/books/:id/pages/:pageNumber/thumbnail", get(get_book_thumbnail))
        .route("/api/v1/books/:id/read-progress", get(get_book_read_progress))
        .route("/api/v1/books/:id/read-progress", patch(update_book_read_progress))
        .route("/api/v1/books/:id/read-progress", delete(delete_book_read_progress))
        .route("/api/v1/series/:seriesId/books", get(get_books_by_series))
}// rebuild Thu Apr 23 22:17:41 +07 2026
