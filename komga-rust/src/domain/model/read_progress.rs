use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadProgress {
    pub book_id: Uuid,
    pub user_id: Uuid,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub page: i32,
    pub completed: bool,
    pub read_date: Option<DateTime<Utc>>,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub locator: Option<Vec<u8>>,
}

impl ReadProgress {
    pub fn new(book_id: Uuid, user_id: Uuid, page: i32, completed: bool) -> Self {
        let now = Utc::now();
        Self {
            book_id,
            user_id,
            created_date: now,
            last_modified_date: now,
            page,
            completed,
            read_date: Some(now),
            device_id: None,
            device_name: None,
            locator: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadProgressUpdateRequest {
    pub page: Option<i32>,
    pub completed: Option<bool>,
}