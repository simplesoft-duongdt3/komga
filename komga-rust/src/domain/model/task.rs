use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub priority: i32,
    pub created_date: DateTime<Utc>,
    pub scheduled_date: Option<DateTime<Utc>>,
    pub execution_start_date: Option<DateTime<Utc>>,
    pub execution_end_date: Option<DateTime<Utc>>,
    pub result: Option<TaskResult>,
    pub data: TaskData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskType {
    ScanLibrary,
    FindBooksToConvert,
    FindBooksWithMissingPageHash,
    FindDuplicatePagesToDelete,
    EmptyTrash,
    AnalyzeBook,
    VerifyBookHash,
    GenerateBookThumbnail,
    RefreshBookMetadata,
    HashBook,
    HashBookPages,
    HashBookKoreader,
    RefreshSeriesMetadata,
    AggregateSeriesMetadata,
    RefreshBookLocalArtwork,
    RefreshSeriesLocalArtwork,
    ImportBook,
    ConvertBook,
    RepairExtension,
    RemoveHashedPages,
    RebuildIndex,
    UpgradeIndex,
    DeleteBook,
    DeleteSeries,
    FindBookThumbnailsToRegenerate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TaskData {
    ScanLibrary { library_id: String, scan_deep: bool },
    FindBooksToConvert { library_id: String },
    FindBooksWithMissingPageHash { library_id: String },
    FindDuplicatePagesToDelete { library_id: String },
    EmptyTrash { library_id: String },
    AnalyzeBook { book_id: String },
    VerifyBookHash { book_id: String },
    GenerateBookThumbnail { book_id: String },
    RefreshBookMetadata { book_id: String, capabilities: Vec<String> },
    HashBook { book_id: String },
    HashBookPages { book_id: String },
    HashBookKoreader { book_id: String },
    RefreshSeriesMetadata { series_id: String },
    AggregateSeriesMetadata { series_id: String },
    RefreshBookLocalArtwork { book_id: String },
    RefreshSeriesLocalArtwork { series_id: String },
    ImportBook { source_file: String, series_id: String, copy_mode: String },
    ConvertBook { book_id: String },
    RepairExtension { book_id: String },
    RemoveHashedPages { book_id: String, pages: Vec<i32> },
    RebuildIndex { entities: Vec<String> },
    UpgradeIndex,
    DeleteBook { book_id: String },
    DeleteSeries { series_id: String },
    FindBookThumbnailsToRegenerate { for_bigger_result_only: bool },
}

impl Task {
    pub fn new(task_type: TaskType, data: TaskData, priority: i32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_type,
            status: TaskStatus::Queued,
            priority,
            created_date: Utc::now(),
            scheduled_date: None,
            execution_start_date: None,
            execution_end_date: None,
            result: None,
            data,
        }
    }
}

pub const HIGHEST_PRIORITY: i32 = 8;
pub const HIGH_PRIORITY: i32 = 6;
pub const DEFAULT_PRIORITY: i32 = 4;
pub const LOW_PRIORITY: i32 = 2;
pub const LOWEST_PRIORITY: i32 = 0;