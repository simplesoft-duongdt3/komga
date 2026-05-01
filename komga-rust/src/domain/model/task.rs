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

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::ScanLibrary => "SCAN_LIBRARY",
            TaskType::FindBooksToConvert => "FIND_BOOKS_TO_CONVERT",
            TaskType::FindBooksWithMissingPageHash => "FIND_BOOKS_WITH_MISSING_PAGE_HASH",
            TaskType::FindDuplicatePagesToDelete => "FIND_DUPLICATE_PAGES_TO_DELETE",
            TaskType::EmptyTrash => "EMPTY_TRASH",
            TaskType::AnalyzeBook => "ANALYZE_BOOK",
            TaskType::VerifyBookHash => "VERIFY_BOOK_HASH",
            TaskType::GenerateBookThumbnail => "GENERATE_BOOK_THUMBNAIL",
            TaskType::RefreshBookMetadata => "REFRESH_BOOK_METADATA",
            TaskType::HashBook => "HASH_BOOK",
            TaskType::HashBookPages => "HASH_BOOK_PAGES",
            TaskType::HashBookKoreader => "HASH_BOOK_KOREADER",
            TaskType::RefreshSeriesMetadata => "REFRESH_SERIES_METADATA",
            TaskType::AggregateSeriesMetadata => "AGGREGATE_SERIES_METADATA",
            TaskType::RefreshBookLocalArtwork => "REFRESH_BOOK_LOCAL_ARTWORK",
            TaskType::RefreshSeriesLocalArtwork => "REFRESH_SERIES_LOCAL_ARTWORK",
            TaskType::ImportBook => "IMPORT_BOOK",
            TaskType::ConvertBook => "CONVERT_BOOK",
            TaskType::RepairExtension => "REPAIR_EXTENSION",
            TaskType::RemoveHashedPages => "REMOVE_HASHED_PAGES",
            TaskType::RebuildIndex => "REBUILD_INDEX",
            TaskType::UpgradeIndex => "UPGRADE_INDEX",
            TaskType::DeleteBook => "DELETE_BOOK",
            TaskType::DeleteSeries => "DELETE_SERIES",
            TaskType::FindBookThumbnailsToRegenerate => "FIND_BOOK_THUMBNAILS_TO_REGENERATE",
        }
    }

    pub fn simple_type(&self) -> &'static str {
        match self {
            TaskType::ScanLibrary => "ScanLibrary",
            TaskType::FindBooksToConvert => "FindBooksToConvert",
            TaskType::FindBooksWithMissingPageHash => "FindBooksWithMissingPageHash",
            TaskType::FindDuplicatePagesToDelete => "FindDuplicatePagesToDelete",
            TaskType::EmptyTrash => "EmptyTrash",
            TaskType::AnalyzeBook => "AnalyzeBook",
            TaskType::VerifyBookHash => "VerifyBookHash",
            TaskType::GenerateBookThumbnail => "GenerateBookThumbnail",
            TaskType::RefreshBookMetadata => "RefreshBookMetadata",
            TaskType::HashBook => "HashBook",
            TaskType::HashBookPages => "HashBookPages",
            TaskType::HashBookKoreader => "HashBookKoreader",
            TaskType::RefreshSeriesMetadata => "RefreshSeriesMetadata",
            TaskType::AggregateSeriesMetadata => "AggregateSeriesMetadata",
            TaskType::RefreshBookLocalArtwork => "RefreshBookLocalArtwork",
            TaskType::RefreshSeriesLocalArtwork => "RefreshSeriesLocalArtwork",
            TaskType::ImportBook => "ImportBook",
            TaskType::ConvertBook => "ConvertBook",
            TaskType::RepairExtension => "RepairExtension",
            TaskType::RemoveHashedPages => "RemoveHashedPages",
            TaskType::RebuildIndex => "RebuildIndex",
            TaskType::UpgradeIndex => "UpgradeIndex",
            TaskType::DeleteBook => "DeleteBook",
            TaskType::DeleteSeries => "DeleteSeries",
            TaskType::FindBookThumbnailsToRegenerate => "FindBookThumbnailsToRegenerate",
        }
    }
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

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Queued => "QUEUED",
            TaskStatus::Running => "RUNNING",
            TaskStatus::Completed => "COMPLETED",
            TaskStatus::Failed => "FAILED",
            TaskStatus::Cancelled => "CANCELLED",
        }
    }
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