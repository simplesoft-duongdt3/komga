use axum::{
    extract::{Path, Query, State, Multipart},
    routing::{get, put, delete, patch, post},
    Router, Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use std::path::PathBuf;
use std::fs;

use crate::api::dto::{SeriesDto, SeriesPageDto, SeriesMetadataDto};
use crate::domain::repository::SeriesRepository;
use crate::domain::model::series::SeriesMetadata;

#[derive(Deserialize)]
struct PageParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_size")]
    size: usize,
}

fn default_page() -> usize { 0 }
fn default_size() -> usize { 20 }

async fn get_series_by_library(
    State(pool): State<PgPool>,
    Path(library_id): Path<String>,
    Query(params): Query<PageParams>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let library_uuid = Uuid::parse_str(&library_id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    match repo.find_by_library(library_uuid).await {
        Ok(series_list) => {
            let total = series_list.len();
            let series: Vec<SeriesDto> = series_list.into_iter().map(|s| s.into()).collect();
            Ok(Json(SeriesPageDto {
                content: series,
                total_elements: total,
                total_pages: 1,
                number: params.page,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<SeriesDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(series)) => Ok(Json(series.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series_cover(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    let series = match repo.find_by_id(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let cover_path = series.cover_file_name
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

async fn upload_series_cover(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool.clone());
    
    let series = match repo.find_by_id(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
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
    
    let cover_path = covers_dir.join(format!("{}.jpg", series.id));
    if let Err(e) = fs::write(&cover_path, &bytes) {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    let repo = SeriesRepository::new(pool.clone());
    if let Err(e) = repo.update_cover(&uuid, cover_path.to_string_lossy().to_string()).await {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
    }
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_series_cover(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    let series = match repo.find_by_id(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    if let Some(cover_path) = series.cover_file_name {
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

async fn update_series_metadata(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<SeriesMetadataDto>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool.clone());
    
    let series = match repo.find_by_id(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let now = chrono::Utc::now();
    let mut metadata = series.metadata.unwrap_or(SeriesMetadata {
        created_date: now,
        last_modified_date: now,
        status: "OK".to_string(),
        status_lock: false,
        title: series.name.clone(),
        title_lock: false,
        title_sort: String::new(),
        title_sort_lock: false,
        series_id: uuid,
        publisher: String::new(),
        publisher_lock: false,
        reading_direction: None,
        reading_direction_lock: false,
        age_rating: None,
        age_rating_lock: false,
        summary: String::new(),
        summary_lock: false,
        language: "en".to_string(),
        language_lock: false,
        genres: vec![],
        genres_lock: false,
        tags: vec![],
        tags_lock: false,
        total_book_count: None,
        total_book_count_lock: false,
        sharing_labels: vec![],
        sharing_labels_lock: false,
        links: vec![],
        links_lock: false,
        alternate_titles: vec![],
        alternate_titles_lock: false,
    });
    
    metadata.last_modified_date = now;
    
    if let Some(title) = req.title {
        metadata.title = title;
    }
    if let Some(title_sort) = req.title_sort {
        metadata.title_sort = title_sort;
    }
    if let Some(publisher) = req.publisher {
        metadata.publisher = publisher;
    }
    if let Some(reading_direction) = req.reading_direction {
        metadata.reading_direction = Some(reading_direction);
    }
    if let Some(age_rating) = req.age_rating {
        metadata.age_rating = Some(age_rating);
    }
    if let Some(summary) = req.summary {
        metadata.summary = summary;
    }
    if let Some(language) = req.language {
        metadata.language = language;
    }
    if let Some(genres) = req.genres {
        metadata.genres = genres;
    }
    if let Some(tags) = req.tags {
        metadata.tags = tags;
    }
    
    repo.update_metadata(&uuid, &metadata).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn list_series(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let repo = SeriesRepository::new(pool);
    match repo.find_all(params.size, params.page * params.size).await {
        Ok(series_list) => {
            let total = series_list.len();
            Ok(Json(SeriesPageDto {
                content: series_list.into_iter().map(|s| s.into()).collect(),
                total_elements: total,
                total_pages: 1,
                number: params.page,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

#[derive(Deserialize)]
struct SeriesSearchRequest {
    #[serde(rename = "fullTextSearch")]
    full_text_search: Option<String>,
}

async fn list_series_post(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
    Json(_search): Json<SeriesSearchRequest>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let repo = SeriesRepository::new(pool);
    match repo.find_all(params.size, params.page * params.size).await {
        Ok(series_list) => {
            let total = series_list.len();
            Ok(Json(SeriesPageDto {
                content: series_list.into_iter().map(|s| s.into()).collect(),
                total_elements: total,
                total_pages: 1,
                number: params.page,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series_latest(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let repo = SeriesRepository::new(pool);
    match repo.find_latest(params.size).await {
        Ok(series_list) => {
            let total = series_list.len();
            Ok(Json(SeriesPageDto {
                content: series_list.into_iter().map(|s| s.into()).collect(),
                total_elements: total,
                total_pages: 1,
                number: 0,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series_new(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let repo = SeriesRepository::new(pool);
    match repo.find_new(params.size).await {
        Ok(series_list) => {
            let total = series_list.len();
            Ok(Json(SeriesPageDto {
                content: series_list.into_iter().map(|s| s.into()).collect(),
                total_elements: total,
                total_pages: 1,
                number: 0,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series_updated(
    State(pool): State<PgPool>,
    Query(params): Query<PageParams>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let repo = SeriesRepository::new(pool);
    match repo.find_updated(params.size).await {
        Ok(series_list) => {
            let total = series_list.len();
            Ok(Json(SeriesPageDto {
                content: series_list.into_iter().map(|s| s.into()).collect(),
                total_elements: total,
                total_pages: 1,
                number: 0,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series_alphabetical_groups(
    State(_pool): State<PgPool>,
    Query(_params): Query<PageParams>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({ "groups": [] })))
}

async fn list_series_alphabetical_groups(
    State(_pool): State<PgPool>,
    Query(_params): Query<PageParams>,
    Json(_search): Json<SeriesSearchRequest>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({ "groups": [] })))
}

async fn get_series_collections(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, axum::response::Response> {
    Ok(Json(vec![]))
}

async fn analyze_series(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn refresh_series_metadata(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

async fn mark_series_read_progress(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_series_read_progress(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_series_read_progress_tachiyomi(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({})))
}

async fn update_series_read_progress_tachiyomi(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_series_thumbnail(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    let series = match repo.find_by_id(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let cover_path = series.cover_file_name
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

async fn get_series_file(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_IMPLEMENTED, "Series file download not implemented").into_response())
}

async fn delete_series_file(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/series", get(list_series))
        .route("/api/v1/series/list", post(list_series_post))
        .route("/api/v1/series/latest", get(get_series_latest))
        .route("/api/v1/series/new", get(get_series_new))
        .route("/api/v1/series/updated", get(get_series_updated))
        .route("/api/v1/series/alphabetical-groups", get(get_series_alphabetical_groups))
        .route("/api/v1/series/list/alphabetical-groups", post(list_series_alphabetical_groups))
        .route("/api/v1/libraries/{libraryId}/series", get(get_series_by_library))
        .route("/api/v1/series/{id}", get(get_series))
        .route("/api/v1/series/{id}/collections", get(get_series_collections))
        .route("/api/v1/series/{id}/analyze", post(analyze_series))
        .route("/api/v1/series/{id}/metadata/refresh", post(refresh_series_metadata))
        .route("/api/v1/series/{id}/metadata", patch(update_series_metadata))
        .route("/api/v1/series/{id}/read-progress", post(mark_series_read_progress))
        .route("/api/v1/series/{id}/read-progress", delete(delete_series_read_progress))
        .route("/api/v2/series/{id}/read-progress/tachiyomi", get(get_series_read_progress_tachiyomi))
        .route("/api/v2/series/{id}/read-progress/tachiyomi", put(update_series_read_progress_tachiyomi))
        .route("/api/v1/series/{id}/thumbnail", get(get_series_thumbnail))
        .route("/api/v1/series/{id}/file", get(get_series_file))
        .route("/api/v1/series/{id}/file", delete(delete_series_file))
        .route("/api/v1/series/{id}/cover", get(get_series_cover))
        .route("/api/v1/series/{id}/cover", put(upload_series_cover))
        .route("/api/v1/series/{id}/cover", delete(delete_series_cover))
}