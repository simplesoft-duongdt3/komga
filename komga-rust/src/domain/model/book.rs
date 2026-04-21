use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: Uuid,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub file_last_modified: Option<DateTime<Utc>>,
    pub name: String,
    pub url: String,
    pub series_id: Uuid,
    pub file_size: i64,
    pub number: i32,
    pub library_id: Uuid,
    pub file_hash: String,
    pub deleted_date: Option<DateTime<Utc>>,
    pub oneshot: bool,
    pub file_hash_koreader: String,
    pub metadata: Option<BookMetadata>,
    pub media: Option<Media>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMetadata {
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub number: String,
    pub number_lock: bool,
    pub number_sort: f32,
    pub number_sort_lock: bool,
    pub release_date: Option<chrono::NaiveDate>,
    pub release_date_lock: bool,
    pub summary: String,
    pub summary_lock: bool,
    pub title: String,
    pub title_lock: bool,
    pub authors: Vec<Author>,
    pub authors_lock: bool,
    pub tags: Vec<String>,
    pub tags_lock: bool,
    pub book_id: Uuid,
    pub isbn: String,
    pub isbn_lock: bool,
    pub links: Vec<BookMetadataLink>,
    pub links_lock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMetadataLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub media_type: Option<String>,
    pub status: MediaStatus,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub comment: Option<String>,
    pub book_id: Uuid,
    pub page_count: i32,
    pub extension_class: Option<String>,
    pub epub_divina_compatible: bool,
    pub epub_is_kepub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaStatus {
    Ready,
    Error,
    Unsupported,
    Waiting,
}

impl Media {
    pub fn new(book_id: Uuid) -> Self {
        let now = Utc::now();
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
        }
    }
}

impl Book {
    pub fn new(name: String, url: String, series_id: Uuid, library_id: Uuid, number: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_date: now,
            last_modified_date: now,
            file_last_modified: None,
            name,
            url,
            series_id,
            file_size: 0,
            number,
            library_id,
            file_hash: String::new(),
            deleted_date: None,
            oneshot: false,
            file_hash_koreader: String::new(),
            metadata: None,
            media: None,
        }
    }
}