use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub email: String,
    pub password: String,
    pub shared_all_libraries: bool,
    pub age_restriction: Option<i32>,
    pub age_restriction_allow_only: Option<bool>,
    pub roles: Vec<UserRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    Admin,
    PageViewer,
    BookDownload,
    BookUpload,
}

impl User {
    pub fn new(email: String, password: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_date: now,
            last_modified_date: now,
            email,
            password,
            shared_all_libraries: true,
            age_restriction: None,
            age_restriction_allow_only: None,
            roles: vec![UserRole::PageViewer],
        }
    }

    pub fn new_admin(email: String, password: String) -> Self {
        let mut user = Self::new(email, password);
        user.roles = vec![
            UserRole::Admin,
            UserRole::PageViewer,
            UserRole::BookDownload,
        ];
        user
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
}

impl JwtClaims {
    pub fn new(email: String, user_id: Uuid, expires_at: i64) -> Self {
        Self {
            sub: user_id.to_string(),
            email,
            exp: expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: Uuid,
    pub name: String,
    pub key: String,
    pub created_date: DateTime<Utc>,
    pub last_used_date: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn new(user_id: Uuid, name: String) -> Self {
        let now = Utc::now();
        let key = Uuid::new_v4().to_string().replace("-", "");
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            key,
            created_date: now,
            last_used_date: None,
        }
    }
}
