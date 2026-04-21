use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadList {
    pub id: Uuid,
    pub name: String,
    pub book_count: i32,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub summary: String,
    pub ordered: bool,
}

impl ReadList {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            book_count: 0,
            created_date: now,
            last_modified_date: now,
            summary: String::new(),
            ordered: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadListBook {
    pub readlist_id: Uuid,
    pub book_id: Uuid,
    pub number: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateReadListRequest {
    pub name: String,
    pub summary: Option<String>,
    pub ordered: Option<bool>,
    pub book_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateReadListRequest {
    pub name: Option<String>,
    pub summary: Option<String>,
    pub ordered: Option<bool>,
}