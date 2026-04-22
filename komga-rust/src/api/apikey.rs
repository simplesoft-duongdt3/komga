use axum::{
    extract::State,
    routing::{get, post, delete},
    Router, Json,
    response::IntoResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::ApiKeyDto;
use crate::domain::model::user::ApiKey;
use crate::domain::repository::{ApiKeyRepository, UserRepository};

async fn get_api_keys(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ApiKeyDto>>, axum::response::Response> {
    let user_repo = UserRepository::new(pool.clone());
    let api_key_repo = ApiKeyRepository::new(pool);
    
    let user = match user_repo.find_by_email("admin").await {
        Ok(Some(u)) => u,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "User not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    match api_key_repo.find_by_user(user.id).await {
        Ok(keys) => Ok(Json(keys.into_iter().map(|k| k.into()).collect())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn create_api_key(
    State(pool): State<PgPool>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyDto>, axum::response::Response> {
    let user_repo = UserRepository::new(pool.clone());
    let api_key_repo = ApiKeyRepository::new(pool);
    
    let user = match user_repo.find_by_email("admin").await {
        Ok(Some(u)) => u,
        Ok(None) => return Err((axum::http::StatusCode::NOT_FOUND, "User not found").into_response()),
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    };
    
    let api_key = ApiKey::new(user.id, req.name);
    match api_key_repo.create(&api_key).await {
        Ok(key) => Ok(Json(key.into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn delete_api_key(
    State(pool): State<PgPool>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let api_key_repo = ApiKeyRepository::new(pool);
    
    match api_key_repo.delete(&id).await {
        Ok(_) => Ok((axum::http::StatusCode::NO_CONTENT, "").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

#[derive(serde::Deserialize)]
struct CreateApiKeyRequest {
    name: String,
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/api-keys", get(get_api_keys))
        .route("/api/v1/api-keys", post(create_api_key))
        .route("/api/v1/api-keys/{id}", delete(delete_api_key))
}