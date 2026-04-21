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
}