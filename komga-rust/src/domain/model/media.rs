use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub media_type: Option<String>,
    pub status: MediaStatus,
    pub created_date: chrono::DateTime<chrono::Utc>,
    pub last_modified_date: chrono::DateTime<chrono::Utc>,
    pub comment: Option<String>,
    pub book_id: uuid::Uuid,
    pub page_count: i32,
    pub extension_class: Option<String>,
    pub epub_divina_compatible: bool,
    pub epub_is_kepub: bool,
    pub pages: Vec<MediaPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaStatus {
    Ready,
    Error,
    Unsupported,
    Waiting,
    Unknown,
    Outdated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPage {
    pub number: i32,
    pub file_name: String,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size_bytes: Option<i64>,
}

impl Media {
    pub fn new(book_id: uuid::Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            media_type: None,
            status: MediaStatus::Waiting,
            created_date: now,
            last_modified_date: now,
            comment: None,
            book_id,
            page_count: 0,
            extension_class: None,
            epub_divina_compatible: false,
            epub_is_kepub: false,
            pages: Vec::new(),
        }
    }
}