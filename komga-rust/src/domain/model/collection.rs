use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub ordered: bool,
    pub series_count: i32,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
}

impl Collection {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            ordered: false,
            series_count: 0,
            created_date: now,
            last_modified_date: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSeries {
    pub collection_id: Uuid,
    pub series_id: Uuid,
    pub number: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub ordered: Option<bool>,
    pub series_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub ordered: Option<bool>,
    pub series_ids: Option<Vec<String>>,
}