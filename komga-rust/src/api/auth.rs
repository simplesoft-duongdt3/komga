use axum::{
    extract::State,
    routing::{get, post},
    Router, Json,
};
use sqlx::PgPool;

use crate::api::dto::{LoginRequest, LoginResponse, RegisterRequest};

async fn register(
    State(_pool): State<PgPool>,
    Json(_req): Json<RegisterRequest>,
) -> Result<Json<()>, axum::response::Response> {
    Ok(Json(()))
}

async fn login(
    State(_pool): State<PgPool>,
    Json(_req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, axum::response::Response> {
    Ok(Json(LoginResponse {
        token: "placeholder".to_string(),
    }))
}

async fn me(State(_pool): State<PgPool>) -> &'static str {
    "me"
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/users", post(register))
        .route("/api/v1/users/login", post(login))
        .route("/api/v1/users/me", get(me))
}