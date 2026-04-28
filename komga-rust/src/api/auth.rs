use axum::{
    extract::{Path, State},
    routing::{get, post, patch, delete, put},
    Router, Json,
    response::IntoResponse,
};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::api::dto::{LoginRequest, LoginResponse, RegisterRequest};
use crate::domain::repository::{UserRepository, ApiKeyRepository, ServerSettingsRepository, ClientSettingsRepository, HistoricalEventRepository};
use crate::domain::model::user::ApiKey;
use crate::infrastructure::auth::JwtAuth;

fn get_jwt_auth() -> JwtAuth {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "komga-rust-dev-secret".to_string());
    JwtAuth::new(secret)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimStatus {
    is_claimed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientSetting {
    key: String,
    value: String,
}

async fn register(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<()>, axum::response::Response> {
    let repo = UserRepository::new(pool);
    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    repo.create(&req.email, &password_hash).await
        .map_err(|e| (axum::http::StatusCode::CONFLICT, format!("Registration failed: {}", e)).into_response())?;
    
    Ok(Json(()))
}

async fn login(
    State(pool): State<PgPool>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, axum::response::Response> {
    let repo = UserRepository::new(pool);
    let user = repo.find_by_email(&req.email).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
        .ok_or_else(|| (axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials").into_response())?;
    
    if !bcrypt::verify(&req.password, &user.password)
        .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials").into_response())? 
    {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials").into_response());
    }
    
    let jwt = get_jwt_auth();
    let token = jwt.generate_token(user.id, user.email.clone())
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok(Json(LoginResponse { token }))
}

async fn me(State(pool): State<PgPool>) -> Json<UserDto> {
    let repo = UserRepository::new(pool);
    let users = repo.find_all().await.unwrap_or_default();
    match users.into_iter().next() {
        Some(u) => Json(UserDto {
            id: u.id.to_string(),
            email: u.email,
            roles: u.roles.iter().map(|r| format!("{:?}", r)).collect(),
            shared_all_libraries: u.shared_all_libraries,
        }),
        None => Json(UserDto {
            id: "admin".to_string(),
            email: "admin@localhost".to_string(),
            roles: vec!["ADMIN".to_string()],
            shared_all_libraries: true,
        }),
    }
}

#[derive(Serialize)]
struct UserDto {
    id: String,
    email: String,
    roles: Vec<String>,
    shared_all_libraries: bool,
}

async fn list_users(State(pool): State<PgPool>) -> Json<Vec<UserDto>> {
    let repo = UserRepository::new(pool);
    let users = repo.find_all().await.unwrap_or_default();
    Json(users.into_iter().map(|u| UserDto {
        id: u.id.to_string(),
        email: u.email,
        roles: u.roles.iter().map(|r| format!("{:?}", r)).collect(),
        shared_all_libraries: u.shared_all_libraries,
    }).collect())
}

async fn get_user(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<UserDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = UserRepository::new(pool);
    let user = repo.find_by_id(uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
        .ok_or_else(|| (axum::http::StatusCode::NOT_FOUND, "User not found").into_response())?;
    
    Ok(Json(UserDto {
        id: user.id.to_string(),
        email: user.email,
        roles: user.roles.iter().map(|r| format!("{:?}", r)).collect(),
        shared_all_libraries: user.shared_all_libraries,
    }))
}

async fn delete_user(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = UserRepository::new(pool);
    repo.delete(uuid).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_user(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = UserRepository::new(pool);
    
    let email = req.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
    let shared_all_libraries = req.get("sharedAllLibraries").and_then(|v| v.as_bool());
    let age_restriction = req.get("ageRestriction").and_then(|v| v.as_i64()).map(|v| v as i32);
    
    repo.update(uuid, email, shared_all_libraries, age_restriction).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_own_password(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    let repo = UserRepository::new(pool);
    let user = repo.find_all().await.unwrap_or_default().into_iter().next()
        .ok_or_else(|| (axum::http::StatusCode::NOT_FOUND, "User not found").into_response())?;
    
    repo.update_password(user.id, &password_hash).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn update_user_password(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    let repo = UserRepository::new(pool);
    repo.update_password(uuid, &password_hash).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_auth_activity(State(pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        r#"SELECT * FROM "AUTHENTICATION_ACTIVITY" ORDER BY "DATE_TIME" DESC LIMIT 50"#
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    
    Json(rows.into_iter().map(|r: sqlx::postgres::PgRow| {
        serde_json::json!({
            "userId": r.get::<Option<String>, _>("USER_ID"),
            "email": r.get::<Option<String>, _>("EMAIL"),
            "ip": r.get::<Option<String>, _>("IP"),
            "userAgent": r.get::<Option<String>, _>("USER_AGENT"),
            "success": r.get::<bool, _>("SUCCESS"),
            "error": r.get::<Option<String>, _>("ERROR"),
            "dateTime": r.get::<chrono::DateTime<chrono::Utc>, _>("DATE_TIME").to_rfc3339(),
            "source": r.get::<Option<String>, _>("SOURCE"),
        })
    }).collect())
}

async fn get_own_auth_activity(State(pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    let user = UserRepository::new(pool.clone()).find_all().await.unwrap_or_default().into_iter().next()
        .unwrap_or_else(|| {
            crate::domain::model::user::User::new_admin("admin@localhost".to_string(), String::new())
        });
    
    let rows = sqlx::query(
        r#"SELECT * FROM "AUTHENTICATION_ACTIVITY" WHERE "USER_ID" = $1 ORDER BY "DATE_TIME" DESC LIMIT 20"#
    )
    .bind(user.id.to_string())
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    
    Json(rows.into_iter().map(|r: sqlx::postgres::PgRow| {
        serde_json::json!({
            "userId": r.get::<Option<String>, _>("USER_ID"),
            "email": r.get::<Option<String>, _>("EMAIL"),
            "ip": r.get::<Option<String>, _>("IP"),
            "userAgent": r.get::<Option<String>, _>("USER_AGENT"),
            "success": r.get::<bool, _>("SUCCESS"),
            "error": r.get::<Option<String>, _>("ERROR"),
            "dateTime": r.get::<chrono::DateTime<chrono::Utc>, _>("DATE_TIME").to_rfc3339(),
            "source": r.get::<Option<String>, _>("SOURCE"),
        })
    }).collect())
}

async fn get_latest_auth_activity(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let result = sqlx::query(
        r#"SELECT * FROM "AUTHENTICATION_ACTIVITY" WHERE "USER_ID" = $1 ORDER BY "DATE_TIME" DESC LIMIT 1"#
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .ok().flatten();
    
    match result {
        Some(r) => {
            let r: sqlx::postgres::PgRow = r;
            Json(serde_json::json!({
                "userId": r.get::<Option<String>, _>("USER_ID"),
                "email": r.get::<Option<String>, _>("EMAIL"),
                "ip": r.get::<Option<String>, _>("IP"),
                "userAgent": r.get::<Option<String>, _>("USER_AGENT"),
                "success": r.get::<bool, _>("SUCCESS"),
                "dateTime": r.get::<chrono::DateTime<chrono::Utc>, _>("DATE_TIME").to_rfc3339(),
            }))
        }
        None => Json(serde_json::json!({})),
    }
}

async fn get_api_keys(State(pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    let user = UserRepository::new(pool.clone()).find_all().await
        .unwrap_or_default()
        .into_iter().next();
    
    let user_id = user.map(|u| u.id).unwrap_or_else(Uuid::nil);
    let key_repo = ApiKeyRepository::new(pool);
    
    let keys = key_repo.find_by_user(user_id).await.unwrap_or_default();
    Json(keys.into_iter().map(|k| {
        serde_json::json!({
            "id": k.id,
            "name": k.name,
            "key": k.key,
            "createdDate": k.created_date.to_rfc3339(),
            "lastUsedDate": k.last_used_date.map(|d| d.to_rfc3339()),
        })
    }).collect())
}

async fn create_api_key(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    let users_result = UserRepository::new(pool.clone()).find_all().await;
    let user = users_result
        .map_err(|e| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        })?
        .into_iter().next()
        .ok_or_else(|| {
            (axum::http::StatusCode::NOT_FOUND, "User not found").into_response()
        })?;
    
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("API Key").to_string();
    let api_key = ApiKey::new(user.id, name);
    let key_repo = ApiKeyRepository::new(pool);
    
    key_repo.create(&api_key).await
        .map_err(|e| {
            eprintln!("[DEBUG] API key create error: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        })?;
    
    Ok(Json(serde_json::json!({
        "id": api_key.id,
        "name": api_key.name,
        "key": api_key.key,
        "createdDate": api_key.created_date.to_rfc3339(),
    })))
}

async fn delete_api_key(
    State(pool): State<PgPool>,
    Path(key_id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let key_repo = ApiKeyRepository::new(pool);
    key_repo.delete(&key_id).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn claim_status(State(pool): State<PgPool>) -> Json<ClaimStatus> {
    let repo = UserRepository::new(pool);
    let users = repo.find_all().await.unwrap_or_default();
    Json(ClaimStatus {
        is_claimed: !users.is_empty(),
    })
}

async fn claim_account(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<LoginResponse>, axum::response::Response> {
    let email = req.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    
    let repo = UserRepository::new(pool.clone());
    let existing = repo.find_all().await.unwrap_or_default();
    if !existing.is_empty() {
        return Err((axum::http::StatusCode::CONFLICT, "Already claimed").into_response());
    }
    
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    let user = repo.create(email, &password_hash).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
    
    let jwt = get_jwt_auth();
    let token = jwt.generate_token(user.id, user.email).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    })?;
    
    Ok(Json(LoginResponse { token }))
}

async fn get_client_settings(State(pool): State<PgPool>) -> Json<Vec<ClientSetting>> {
    let repo = ClientSettingsRepository::new(pool);
    let settings = repo.get_global_all().await.unwrap_or_default();
    Json(settings.into_iter().map(|(key, value, allow)| ClientSetting {
        key,
        value,
    }).collect())
}

async fn update_client_settings(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let repo = ClientSettingsRepository::new(pool);
    if let Some(obj) = req.as_object() {
        for (key, value) in obj {
            let val_str = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            repo.set_global(key, &val_str, false).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
        }
    }
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_client_settings(
    State(pool): State<PgPool>,
) -> Result<axum::response::Response, axum::response::Response> {
    let repo = ClientSettingsRepository::new(pool);
    let settings = repo.get_global_all().await.unwrap_or_default();
    for (key, _, _) in settings {
        let _ = repo.delete_global(&key).await;
    }
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_user_client_settings(State(pool): State<PgPool>) -> Json<Vec<ClientSetting>> {
    let user = UserRepository::new(pool.clone()).find_all().await
        .unwrap_or_default()
        .into_iter().next();
    if let Some(u) = user {
        let repo = ClientSettingsRepository::new(pool);
        let settings = repo.get_user_all(u.id).await.unwrap_or_default();
        Json(settings.into_iter().map(|(key, value)| ClientSetting { key, value }).collect())
    } else {
        Json(vec![])
    }
}

async fn update_user_client_settings(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let user = UserRepository::new(pool.clone()).find_all().await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
        .into_iter().next()
        .ok_or_else(|| (axum::http::StatusCode::NOT_FOUND, "User not found").into_response())?;
    
    let repo = ClientSettingsRepository::new(pool);
    if let Some(obj) = req.as_object() {
        for (key, value) in obj {
            let val_str = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            repo.set_user(user.id, key, &val_str).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
        }
    }
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn delete_user_client_settings(
    State(pool): State<PgPool>,
) -> Result<axum::response::Response, axum::response::Response> {
    let user = UserRepository::new(pool.clone()).find_all().await
        .unwrap_or_default()
        .into_iter().next();
    if let Some(u) = user {
        let repo = ClientSettingsRepository::new(pool);
        let settings = repo.get_user_all(u.id).await.unwrap_or_default();
        for (key, _) in settings {
            let _ = repo.delete_user_setting(u.id, &key).await;
        }
    }
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_settings(State(pool): State<PgPool>) -> Json<serde_json::Value> {
    let repo = ServerSettingsRepository::new(pool);
    let settings = repo.get_all().await.unwrap_or_default();
    let mut map = serde_json::Map::new();
    for (key, value) in settings {
        map.insert(key, serde_json::Value::String(value));
    }
    if map.is_empty() {
        Json(serde_json::json!({
            "scanStartup": false,
            "scanCron": "",
            "taskPoolSize": 4,
        }))
    } else {
        Json(serde_json::Value::Object(map))
    }
}

async fn update_settings(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let repo = ServerSettingsRepository::new(pool);
    if let Some(obj) = req.as_object() {
        for (key, value) in obj {
            let val_str = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            repo.set(key, &val_str).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
        }
    }
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_releases(State(_pool): State<PgPool>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "buildDate": "2025-01-01",
        "name": "komga-rust",
        "description": "Komga media server written in Rust",
    }))
}

async fn get_announcements(State(pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    let user = UserRepository::new(pool.clone()).find_all().await
        .unwrap_or_default()
        .into_iter().next();
    if let Some(u) = user {
        let rows = sqlx::query(
            r#"SELECT "ANNOUNCEMENT_ID" FROM "ANNOUNCEMENTS_READ" WHERE "USER_ID" = $1"#
        )
        .bind(u.id.to_string())
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
        
        Json(rows.into_iter().map(|r: sqlx::postgres::PgRow| {
            serde_json::json!({
                "id": r.get::<String, _>("ANNOUNCEMENT_ID"),
                "read": true,
            })
        }).collect())
    } else {
        Json(vec![])
    }
}

async fn update_announcements(
    State(pool): State<PgPool>,
    Json(req): Json<serde_json::Value>,
) -> Result<axum::response::Response, axum::response::Response> {
    let user = UserRepository::new(pool.clone()).find_all().await
        .unwrap_or_default()
        .into_iter().next();
    if let Some(u) = user {
        if let Some(ids) = req.get("ids").and_then(|v| v.as_array()) {
            for id_val in ids {
                if let Some(id) = id_val.as_str() {
                    let _ = sqlx::query(
                        r#"INSERT INTO "ANNOUNCEMENTS_READ" ("USER_ID", "ANNOUNCEMENT_ID") VALUES ($1, $2)
                        ON CONFLICT DO NOTHING"#
                    )
                    .bind(u.id.to_string())
                    .bind(id)
                    .execute(&pool)
                    .await;
                }
            }
        }
    }
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_history(State(pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    let repo = HistoricalEventRepository::new(pool);
    let events = repo.find_all(50).await.unwrap_or_default();
    Json(events.into_iter().map(|e| {
        let props: serde_json::Map<String, serde_json::Value> = e.properties.into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect();
        serde_json::json!({
            "id": e.id,
            "type": e.event_type,
            "bookId": e.book_id,
            "seriesId": e.series_id,
            "timestamp": e.timestamp.to_rfc3339(),
            "properties": props,
        })
    }).collect())
}

async fn delete_syncpoints(State(_pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
}

async fn get_oauth2_providers(State(_pool): State<PgPool>) -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

async fn get_login_set_cookie(State(_pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    let cookie_value = format!("SESSION=komga-rust-session; Path=/; HttpOnly; SameSite=Lax");
    Ok((
        axum::http::StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie_value)],
        "",
    ).into_response())
}

async fn logout(State(_pool): State<PgPool>) -> Result<axum::response::Response, axum::response::Response> {
    let cookie_value = "SESSION=; Path=/; HttpOnly; Max-Age=0";
    Ok((
        axum::http::StatusCode::OK,
        [(axum::http::header::SET_COOKIE, cookie_value)],
        "",
    ).into_response())
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/users", post(register))
        .route("/api/v2/users", get(list_users))
        .route("/api/v2/users/:id", get(get_user))
        .route("/api/v2/users/:id", delete(delete_user))
        .route("/api/v2/users/:id", patch(update_user))
        .route("/api/v1/users/login", post(login))
        .route("/api/v2/users/me", get(me))
        .route("/api/v1/users/me", get(me))
        .route("/api/v2/users/me/password", patch(update_own_password))
        .route("/api/v2/users/:id/password", patch(update_user_password))
        .route("/api/v2/users/me/authentication-activity", get(get_own_auth_activity))
        .route("/api/v2/users/:id/authentication-activity/latest", get(get_latest_auth_activity))
        .route("/api/v2/users/me/api-keys", get(get_api_keys))
        .route("/api/v2/users/me/api-keys", post(create_api_key))
        .route("/api/v2/users/me/api-keys/:keyId", delete(delete_api_key))
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
