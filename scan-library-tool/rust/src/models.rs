use serde::{Deserialize, Serialize};

// ── Libraries ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub root: Option<String>,
}

// ── Database types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbSeries {
    #[sqlx(rename = "ID")]
    pub id: String,
    #[sqlx(rename = "NAME")]
    pub name: String,
    #[sqlx(rename = "URL")]
    pub url: String,
    #[sqlx(rename = "FILE_LAST_MODIFIED")]
    pub file_last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbBook {
    #[sqlx(rename = "ID")]
    pub id: String,
    #[sqlx(rename = "NAME")]
    pub name: String,
    #[sqlx(rename = "URL")]
    pub url: String,
    #[sqlx(rename = "FILE_LAST_MODIFIED")]
    pub file_last_modified: Option<String>,
    #[sqlx(rename = "FILE_SIZE")]
    pub file_size: Option<i64>,
    #[sqlx(rename = "FILE_HASH")]
    pub file_hash: Option<String>,
    #[sqlx(rename = "SERIES_ID")]
    pub series_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbUnanalyzedBook {
    #[sqlx(rename = "ID")]
    pub id: String,
    #[sqlx(rename = "NAME")]
    pub name: String,
    #[sqlx(rename = "SERIES_ID")]
    pub series_id: Option<String>,
    #[sqlx(rename = "STATUS")]
    pub media_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbBookNoThumbnail {
    #[sqlx(rename = "ID")]
    pub id: String,
    #[sqlx(rename = "NAME")]
    pub name: String,
    #[sqlx(rename = "SERIES_ID")]
    pub series_id: Option<String>,
    #[sqlx(rename = "STATUS")]
    pub media_status: Option<String>,
}

// ── Filesystem types ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsBook {
    pub url: String,
    pub name: String,
    pub file_size: u64,
    pub file_last_modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsSeries {
    pub url: String,
    pub name: String,
    pub file_last_modified: String,
    pub books: Vec<FsBook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsData {
    pub series: std::collections::HashMap<String, FsSeries>,
    pub oneshots: Vec<FsBook>,
}

// ── Diff types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Diff {
    pub new_series: Vec<FsSeries>,
    pub deleted_series: Vec<DbSeries>,
    pub new_books: Vec<FsBook>,
    pub deleted_books: Vec<DbBook>,
    pub changed_books: Vec<ChangedBook>,
    pub pending_hash: Vec<PendingHashBook>,
    pub to_be_analyzed: Vec<DbUnanalyzedBook>,
    pub no_metadata: Vec<DbBookNoThumbnail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedBook {
    pub id: String,
    pub name: String,
    pub series_id: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingHashBook {
    pub id: String,
    pub name: String,
    pub series_id: Option<String>,
    pub url: String,
    pub file_hash: String,
}

// ── Hash cache ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashCacheEntry {
    pub hash: String,
    pub size: u64,
    pub mtime_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashCache {
    #[serde(rename = "type")]
    pub cache_type: String,
    pub version: u8,
    pub root: String,
    pub generated_at_unix_secs: u64,
    pub elapsed_secs: f64,
    pub total_files: usize,
    pub total_bytes: u64,
    pub entries: std::collections::HashMap<String, HashCacheEntry>,
}

// ── API DTOs ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSeriesRequest {
    pub library_id: String,
    pub name: String,
    pub url: String,
    pub file_last_modified: String,
    pub books: Vec<BookDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDto {
    pub name: String,
    pub url: String,
    pub file_size: u64,
    pub file_last_modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesDto {
    pub id: String,
    pub name: String,
    // other fields ignored
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookResponseDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BooksPage {
    pub content: Vec<BookResponseDto>,
    pub total_elements: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHashRequest {
    pub file_hash: String,
}
