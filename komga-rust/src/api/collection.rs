use axum::{
    extract::{Path, Query, State},
    routing::{get, post, patch, delete},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::{CollectionDto, CollectionPageDto, CreateCollectionRequest, UpdateCollectionRequest};
use crate::domain::model::collection::Collection;
use crate::domain::repository::CollectionRepository;

#[derive(Deserialize)]
struct PageParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_size")]
    size: usize,
}

fn default_page() -> usize { 0 }
fn default_size() -> usize { 20 }

async fn get_all_collections(
    State(pool): State<PgPool>,
    Query(_params): Query<PageParams>,
) -> Result<Json<CollectionPageDto>, axum::response::Response> {
    let repo = CollectionRepository::new(pool);
    
    match repo.find_all().await {
        Ok(collections) => {
            let total = collections.len();
            let collections: Vec<CollectionDto> = collections.into_iter().map(|c| c.into()).collect();
            Ok(Json(CollectionPageDto {
                content: collections,
                total_elements: total,
                total_pages: 1,
                number: 0,
                size: total,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_collection(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<CollectionDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = CollectionRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(collection)) => Ok(Json(collection.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Collection not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn create_collection(
    State(pool): State<PgPool>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionDto>, axum::response::Response> {
    let mut collection = Collection::new(req.name);
    if let Some(ordered) = req.ordered {
        collection.ordered = ordered;
    }
    
    let repo = CollectionRepository::new(pool);
    
    match repo.create(&collection).await {
        Ok(created) => Ok(Json(created.into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn update_collection(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCollectionRequest>,
) -> Result<Json<CollectionDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = CollectionRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(mut collection)) => {
            if let Some(name) = req.name {
                collection.name = name;
            }
            if let Some(ordered) = req.ordered {
                collection.ordered = ordered;
            }
            
            match repo.update(&collection).await {
                Ok(updated) => Ok(Json(updated.into())),
                Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
            }
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Collection not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn delete_collection(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<(), axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = CollectionRepository::new(pool);
    
    match repo.delete(uuid).await {
        Ok(_) => Ok(()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/collections", get(get_all_collections))
        .route("/api/v1/collections", post(create_collection))
        .route("/api/v1/collections/{id}", get(get_collection))
        .route("/api/v1/collections/{id}", patch(update_collection))
        .route("/api/v1/collections/{id}", delete(delete_collection))
}