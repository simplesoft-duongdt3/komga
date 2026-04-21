use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: Uuid,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub file_last_modified: Option<DateTime<Utc>>,
    pub name: String,
    pub url: String,
    pub library_id: Uuid,
    pub book_count: i32,
    pub deleted_date: Option<DateTime<Utc>>,
    pub oneshot: bool,
    pub metadata: Option<SeriesMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesMetadata {
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub status: String,
    pub status_lock: bool,
    pub title: String,
    pub title_lock: bool,
    pub title_sort: String,
    pub title_sort_lock: bool,
    pub series_id: Uuid,
    pub publisher: String,
    pub publisher_lock: bool,
    pub reading_direction: Option<String>,
    pub reading_direction_lock: bool,
    pub age_rating: Option<i32>,
    pub age_rating_lock: bool,
    pub summary: String,
    pub summary_lock: bool,
    pub language: String,
    pub language_lock: bool,
    pub genres: Vec<String>,
    pub genres_lock: bool,
    pub tags: Vec<String>,
    pub tags_lock: bool,
    pub total_book_count: Option<i32>,
    pub total_book_count_lock: bool,
    pub sharing_labels: Vec<String>,
    pub sharing_labels_lock: bool,
    pub links: Vec<MetadataLink>,
    pub links_lock: bool,
    pub alternate_titles: Vec<AlternateTitle>,
    pub alternate_titles_lock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternateTitle {
    pub label: String,
    pub title: String,
}

impl Series {
    pub fn new(name: String, url: String, library_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_date: now,
            last_modified_date: now,
            file_last_modified: None,
            name,
            url,
            library_id,
            book_count: 0,
            deleted_date: None,
            oneshot: false,
            metadata: None,
        }
    }
}