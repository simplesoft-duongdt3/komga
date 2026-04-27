use axum::{
    extract::{Path, Query, State},
    routing::{get, post, patch, delete},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::{ReadListDto, ReadListPageDto, CreateReadListRequest, UpdateReadListRequest};
use crate::domain::model::readlist::ReadList;
use crate::domain::repository::ReadListRepository;

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
            Ok(Json(ReadListPageDto {
                content: readlists,
                total_elements: total,
                total_pages: 1,
                number: 0,
                size: total,
            }))
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
        .route("/api/v1/readlists/{id}", get(get_readlist))
        .route("/api/v1/readlists/{id}", patch(update_readlist))
        .route("/api/v1/readlists/{id}", delete(delete_readlist))
        .route("/api/v1/readlists/match/comicrack", post(match_comicrack))
        .route("/api/v1/readlists/{id}/books", get(get_readlist_books))
        .route("/api/v1/readlists/{id}/books/{bookId}/next", get(get_next_book))
        .route("/api/v1/readlists/{id}/books/{bookId}/previous", get(get_previous_book))
        .route("/api/v1/readlists/{id}/file", get(get_readlist_file))
        .route("/api/v1/readlists/{id}/read-progress/tachiyomi", get(get_tachiyomi_progress))
        .route("/api/v1/readlists/{id}/read-progress/tachiyomi", put(update_tachiyomi_progress))
        .route("/api/v1/readlists/{id}/thumbnail", get(get_readlist_thumbnail))
        .route("/api/v1/readlists/{id}/thumbnails", get(get_readlist_thumbnails))
        .route("/api/v1/readlists/{id}/thumbnails", post(create_readlist_thumbnail))
        .route("/api/v1/readlists/{id}/thumbnails/{thumbnailId}", get(get_readlist_thumbnail_by_id))
        .route("/api/v1/readlists/{id}/thumbnails/{thumbnailId}", delete(delete_readlist_thumbnail))
        .route("/api/v1/readlists/{id}/thumbnails/{thumbnailId}/selected", put(mark_readlist_thumbnail_selected))
}

async fn match_comicrack(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<ReadListDto>, axum::response::Response> {
    Ok(Json(ReadListDto {
        id: "temp".to_string(),
        name: "Matched ReadList".to_string(),
        book_count: 0,
        summary: String::new(),
        ordered: false,
    }))
}

async fn get_readlist_books(
    State(_pool): State<PgPool>,
    Path((_id,)): Path<(String,)>,
    Query(_params): Query<PageParams>,
) -> Result<Json<crate::api::dto::BookPageDto>, axum::response::Response> {
    Ok(Json(crate::api::dto::BookPageDto {
        content: vec![],
        total_elements: 0,
        total_pages: 0,
        number: 0,
        size: 20,
    }))
}

async fn get_next_book(
    State(_pool): State<PgPool>,
    Path((_id, _book_id)): Path<(String, String)>,
) -> Result<Json<Option<crate::api::dto::BookDto>>, axum::response::Response> {
    Ok(Json(None))
}

async fn get_previous_book(
    State(_pool): State<PgPool>,
    Path((_id, _book_id)): Path<(String, String)>,
) -> Result<Json<Option<crate::api::dto::BookDto>>, axum::response::Response> {
    Ok(Json(None))
}

async fn get_readlist_file(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_IMPLEMENTED, "ReadList file download not implemented").into_response())
}

async fn get_tachiyomi_progress(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({})))
}

async fn update_tachiyomi_progress(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_readlist_thumbnail(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response())
}

async fn get_readlist_thumbnails(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, axum::response::Response> {
    Ok(Json(vec![]))
}

async fn create_readlist_thumbnail(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({})))
}

async fn get_readlist_thumbnail_by_id(
    State(_pool): State<PgPool>,
    Path((_id, _thumbnail_id)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Thumbnail not found").into_response())
}

async fn delete_readlist_thumbnail(
    State(_pool): State<PgPool>,
    Path((_id, _thumbnail_id)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn mark_readlist_thumbnail_selected(
    State(_pool): State<PgPool>,
    Path((_id, _thumbnail_id)): Path<(String, String)>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::ACCEPTED, "").into_response())
}