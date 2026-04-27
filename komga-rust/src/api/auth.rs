use axum::{
    extract::{Path, State},
    routing::{get, post, patch, delete},
    Router, Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api::dto::{LoginRequest, LoginResponse, RegisterRequest};

#[derive(Serialize)]
struct ClaimStatus {
    is_claimed: bool,
}

#[derive(Serialize)]
struct ClientSetting {
    key: String,
    value: String,
}

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

async fn me(State(_pool): State<PgPool>) -> Json<UserDto> {
    Json(UserDto {
        id: "admin".to_string(),
        email: "admin@localhost".to_string(),
        roles: vec!["ADMIN".to_string()],
        shared_all_libraries: true,
    })
}

#[derive(Serialize)]
struct UserDto {
    id: String,
    email: String,
    roles: Vec<String>,
    shared_all_libraries: bool,
}

async fn list_users(State(_pool): State<PgPool>) -> Json<Vec<UserDto>> {
    Json(vec![UserDto {
        id: "admin".to_string(),
        email: "admin@localhost".to_string(),
        roles: vec!["ADMIN".to_string()],
        shared_all_libraries: true,
    }])
}

async fn get_user(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<UserDto>, axum::response::Response> {
    Ok(Json(UserDto {
        id: "admin".to_string(),
        email: "admin@localhost".to_string(),
        roles: vec!["ADMIN".to_string()],
        shared_all_libraries: true,
    }))
}

async fn delete_user(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_user(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_own_password(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_user_password(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_auth_activity(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn get_own_auth_activity(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn get_latest_auth_activity(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

async fn get_api_keys(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn create_api_key(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    Ok(Json(serde_json::json!({
        "id": "key-1",
        "name": "test-key",
        "key": "abc123",
    })))
}

async fn delete_api_key(
    State(_pool): State<PgPool>,
    Path(_key_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn claim_status() -> Json<ClaimStatus> {
    Json(ClaimStatus { is_claimed: true })
}

async fn claim_account(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<LoginResponse>, axum::response::Response> {
    Ok(Json(LoginResponse {
        token: "placeholder".to_string(),
    }))
}

async fn get_client_settings() -> Json<Vec<ClientSetting>> {
    Json(vec![])
}

async fn update_client_settings(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_client_settings(
    State(_pool): State<PgPool>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_user_client_settings() -> Json<Vec<ClientSetting>> {
    Json(vec![])
}

async fn update_user_client_settings(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_user_client_settings(
    State(_pool): State<PgPool>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_settings(State(_pool): State<PgPool>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "scanStartup": false,
        "scanCron": "0 0 * * * ?",
        "taskPoolSize": 4,
    }))
}

async fn update_settings(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_releases(State(_pool): State<PgPool>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": "0.1.0",
        "buildDate": "2024-01-01",
    }))
}

async fn get_announcements(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn update_announcements(
    State(_pool): State<PgPool>,
    Json(_req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_history(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn delete_syncpoints(State(_pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_oauth2_providers(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn get_login_set_cookie(State(_pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::OK, "").into_response())
}

async fn logout(State(_pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::OK, "").into_response())
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/users", post(register))
        .route("/api/v2/users", get(list_users))
        .route("/api/v2/users/{id}", get(get_user))
        .route("/api/v2/users/{id}", delete(delete_user))
        .route("/api/v2/users/{id}", patch(update_user))
        .route("/api/v1/users/login", post(login))
        .route("/api/v1/users/me", get(me))
        .route("/api/v2/users/me", get(me))
        .route("/api/v2/users/me/password", patch(update_own_password))
        .route("/api/v2/users/{id}/password", patch(update_user_password))
        .route("/api/v2/users/me/authentication-activity", get(get_own_auth_activity))
        .route("/api/v2/users/authentication-activity", get(get_auth_activity))
        .route("/api/v2/users/{id}/authentication-activity/latest", get(get_latest_auth_activity))
        .route("/api/v2/users/me/api-keys", get(get_api_keys))
        .route("/api/v2/users/me/api-keys", post(create_api_key))
        .route("/api/v2/users/me/api-keys/{keyId}", delete(delete_api_key))
        .route("/api/v1/claim", get(claim_status))
        .route("/api/v1/claim", post(claim_account))
        .route("/api/v1/client-settings/global/list", get(get_client_settings))
        .route("/api/v1/client-settings/global", patch(update_client_settings))
        .route("/api/v1/client-settings/global", delete(delete_client_settings))
        .route("/api/v1/client-settings/user/list", get(get_user_client_settings))
        .route("/api/v1/client-settings/user", patch(update_user_client_settings))
        .route("/api/v1/client-settings/user", delete(delete_user_client_settings))
        .route("/api/v1/settings", get(get_settings))
        .route("/api/v1/settings", patch(update_settings))
        .route("/api/v1/releases", get(get_releases))
        .route("/api/v1/announcements", get(get_announcements))
        .route("/api/v1/announcements", put(update_announcements))
        .route("/api/v1/history", get(get_history))
        .route("/api/v1/syncpoints/me", delete(delete_syncpoints))
        .route("/api/v1/oauth2/providers", get(get_oauth2_providers))
        .route("/api/v1/login/set-cookie", get(get_login_set_cookie))
        .route("/api/logout", get(logout))
}
