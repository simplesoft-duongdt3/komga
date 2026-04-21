use axum::{
    extract::{Path, State},
    routing::{get, post, delete},
    Router, Json,
    response::IntoResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::LibraryDto;
use crate::domain::model::library::Library;

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

async fn create_library(
    State(pool): State<PgPool>,
    Json(req): Json<LibraryDto>,
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

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/libraries", get(get_all_libraries))
        .route("/api/v1/libraries", post(create_library))
        .route("/api/v1/libraries/{id}", get(get_library))
        .route("/api/v1/libraries/{id}", delete(delete_library))
}