use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::model::{Book, Library, Series};
use crate::domain::model::{Collection, ReadList};
use crate::domain::model::Task;
use crate::domain::model::user::ApiKey;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
    pub root: String,
    pub import_comic_info_book: bool,
    pub import_comic_info_series: bool,
    pub import_comic_info_collection: bool,
    pub import_epub_book: bool,
    pub import_epub_series: bool,
    pub scan_force_modified_time: bool,
    pub scan_on_startup: bool,
    pub import_local_artwork: bool,
    pub import_comic_info_read_list: bool,
    pub import_barcode_isbn: bool,
    pub convert_to_cbz: bool,
    pub repair_extensions: bool,
    pub empty_trash_after_scan: bool,
    pub import_mylar_series: bool,
    pub series_cover: String,
    pub scan_directory_exclusions: Vec<String>,
    pub scan_cbx: bool,
    pub scan_pdf: bool,
    pub scan_epub: bool,
    pub scan_interval: String,
    pub hash_files: bool,
    pub hash_pages: bool,
    pub analyze_dimensions: bool,
    pub import_comic_info_series_append_volume: bool,
    pub hash_koreader: bool,
    pub oneshots_directory: Option<String>,
    pub unavailable: Option<bool>,
}

impl From<Library> for LibraryDto {
    fn from(lib: Library) -> Self {
        Self {
            id: lib.id.to_string(),
            name: lib.name,
            root: lib.root,
            import_comic_info_book: lib.import_comicinfo_book,
            import_comic_info_series: lib.import_comicinfo_series,
            import_comic_info_collection: lib.import_comicinfo_collection,
            import_epub_book: lib.import_epub_book,
            import_epub_series: lib.import_epub_series,
            scan_force_modified_time: lib.scan_force_modified_time,
            scan_on_startup: lib.scan_startup,
            import_local_artwork: lib.import_local_artwork,
            import_comic_info_read_list: lib.import_comicinfo_readlist,
            import_barcode_isbn: lib.import_barcode_isbn,
            convert_to_cbz: lib.convert_to_cbz,
            repair_extensions: lib.repair_extensions,
            empty_trash_after_scan: lib.empty_trash_after_scan,
            import_mylar_series: lib.import_mylar_series,
            series_cover: format!("{:?}", lib.series_cover),
            scan_directory_exclusions: vec![],
            scan_cbx: lib.scan_cbx,
            scan_pdf: lib.scan_pdf,
            scan_epub: lib.scan_epub,
            scan_interval: format!("{:?}", lib.scan_interval),
            hash_files: lib.hash_files,
            hash_pages: lib.hash_pages,
            analyze_dimensions: lib.analyze_dimensions,
            import_comic_info_series_append_volume: lib.import_comicinfo_series_append_volume,
            hash_koreader: lib.hash_koreader,
            oneshots_directory: lib.oneshots_directory,
            unavailable: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDto {
    pub id: String,
    pub name: String,
    pub url: String,
    pub library_id: String,
    pub books_count: i32,
    pub books_in_progress_count: i32,
    pub books_read_count: i32,
    pub books_unread_count: i32,
    pub oneshot: bool,
    pub deleted: bool,
    pub created: Option<String>,
    pub last_modified: Option<String>,
    pub file_last_modified: Option<String>,
    pub books_metadata: Option<BookMetadataAggregationDto>,
    pub metadata: Option<SeriesMetadataDto>,
}

impl From<Series> for SeriesDto {
    fn from(s: Series) -> Self {
        Self {
            id: s.id.to_string(),
            name: s.name,
            url: s.url,
            library_id: s.library_id.to_string(),
            books_count: s.book_count,
            books_in_progress_count: 0,
            books_read_count: 0,
            books_unread_count: s.book_count,
            oneshot: s.oneshot,
            deleted: s.deleted_date.is_some(),
            created: Some(s.created_date.to_rfc3339()),
            last_modified: Some(s.last_modified_date.to_rfc3339()),
            file_last_modified: None,
            books_metadata: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetadataAggregationDto {
    pub books_count: i32,
    pub books_in_progress_count: i32,
    pub books_read_count: i32,
    pub books_unread_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesMetadataDto {
    pub status: Option<String>,
    pub status_lock: bool,
    pub title: Option<String>,
    pub title_lock: bool,
    pub title_sort: Option<String>,
    pub title_sort_lock: bool,
    pub publisher: Option<String>,
    pub publisher_lock: bool,
    pub reading_direction: Option<String>,
    pub reading_direction_lock: bool,
    pub age_rating: Option<i32>,
    pub age_rating_lock: bool,
    pub summary: Option<String>,
    pub summary_lock: bool,
    pub language: Option<String>,
    pub language_lock: bool,
    pub genres: Option<Vec<String>>,
    pub genres_lock: bool,
    pub tags: Option<Vec<String>>,
    pub tags_lock: bool,
    pub total_book_count: Option<i32>,
    pub total_book_count_lock: bool,
    pub sharing_labels: Option<Vec<String>>,
    pub sharing_labels_lock: bool,
    pub links: Option<Vec<MetadataLinkDto>>,
    pub links_lock: bool,
    pub alternate_titles: Option<Vec<AlternateTitleDto>>,
    pub alternate_titles_lock: bool,
    pub created: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataLinkDto {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternateTitleDto {
    pub label: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetadataDto {
    pub number: Option<String>,
    pub number_lock: bool,
    pub number_sort: Option<f32>,
    pub number_sort_lock: bool,
    pub release_date: Option<String>,
    pub release_date_lock: bool,
    pub summary: Option<String>,
    pub summary_lock: bool,
    pub title: Option<String>,
    pub title_lock: bool,
    pub authors: Option<Vec<AuthorDto>>,
    pub authors_lock: bool,
    pub tags: Option<Vec<String>>,
    pub tags_lock: bool,
    pub isbn: Option<String>,
    pub isbn_lock: bool,
    pub links: Option<Vec<MetadataLinkDto>>,
    pub links_lock: bool,
    pub created: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorDto {
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDto {
    pub id: String,
    pub name: String,
    pub url: String,
    pub series_id: String,
    pub series_title: String,
    pub number: i32,
    pub oneshot: bool,
    pub file_hash: String,
    pub size_bytes: i64,
    pub size: String,
    pub library_id: String,
    pub created: Option<String>,
    pub last_modified: Option<String>,
    pub file_last_modified: Option<String>,
    pub deleted: bool,
    pub media: Option<MediaDto>,
    pub metadata: Option<BookMetadataDto>,
    pub read_progress: Option<ReadProgressDto>,
}

impl From<Book> for BookDto {
    fn from(b: Book) -> Self {
        Self {
            id: b.id.to_string(),
            name: b.name,
            url: b.url,
            series_id: b.series_id.to_string(),
            series_title: String::new(),
            number: b.number,
            oneshot: b.oneshot,
            file_hash: b.file_hash,
            size_bytes: b.file_size,
            size: String::new(),
            library_id: b.library_id.to_string(),
            created: Some(b.created_date.to_rfc3339()),
            last_modified: Some(b.last_modified_date.to_rfc3339()),
            file_last_modified: None,
            deleted: b.deleted_date.is_some(),
            media: None,
            metadata: None,
            read_progress: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDto {
    pub file_size: i64,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub pages_count: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub number: i32,
    pub file_name: String,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size_bytes: Option<i64>,
    pub size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReadProgressDto {
    pub page: i32,
    pub completed: bool,
    pub read_date: Option<String>,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub created: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadProgressUpdateRequest {
    pub page: Option<i32>,
    pub completed: Option<bool>,
}

// Paginated DTOs

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPageDto {
    pub content: Vec<SeriesDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
    pub empty: bool,
    pub first: bool,
    pub last: bool,
    pub number_of_elements: usize,
}

impl SeriesPageDto {
    pub fn new(content: Vec<SeriesDto>, total_elements: usize, page: usize, size: usize) -> Self {
        let total_pages = if size > 0 { (total_elements + size - 1) / size } else { 1 };
        Self {
            number_of_elements: content.len(),
            empty: content.is_empty(),
            first: page == 0,
            last: page + 1 >= total_pages,
            content,
            total_elements,
            total_pages,
            number: page,
            size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BookPageDto {
    pub content: Vec<BookDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
    pub empty: bool,
    pub first: bool,
    pub last: bool,
    pub number_of_elements: usize,
}

impl BookPageDto {
    pub fn new(content: Vec<BookDto>, total_elements: usize, page: usize, size: usize) -> Self {
        let total_pages = if size > 0 { (total_elements + size - 1) / size } else { 1 };
        Self {
            number_of_elements: content.len(),
            empty: content.is_empty(),
            first: page == 0,
            last: page + 1 >= total_pages,
            content,
            total_elements,
            total_pages,
            number: page,
            size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadListDto {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub ordered: bool,
    pub filtered: bool,
    pub book_ids: Vec<String>,
    pub created_date: Option<String>,
    pub last_modified_date: Option<String>,
}

impl From<ReadList> for ReadListDto {
    fn from(r: ReadList) -> Self {
        Self {
            id: r.id.to_string(),
            name: r.name,
            summary: r.summary,
            ordered: r.ordered,
            filtered: false,
            book_ids: vec![],
            created_date: Some(r.created_date.to_rfc3339()),
            last_modified_date: Some(r.last_modified_date.to_rfc3339()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReadListPageDto {
    pub content: Vec<ReadListDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
    pub empty: bool,
    pub first: bool,
    pub last: bool,
    pub number_of_elements: usize,
}

impl ReadListPageDto {
    pub fn new(content: Vec<ReadListDto>, total_elements: usize, page: usize, size: usize) -> Self {
        let total_pages = if size > 0 { (total_elements + size - 1) / size } else { 1 };
        Self {
            number_of_elements: content.len(),
            empty: content.is_empty(),
            first: page == 0,
            last: page + 1 >= total_pages,
            content,
            total_elements,
            total_pages,
            number: page,
            size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReadListRequest {
    pub name: String,
    pub summary: Option<String>,
    pub ordered: Option<bool>,
    pub book_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReadListRequest {
    pub name: Option<String>,
    pub summary: Option<String>,
    pub ordered: Option<bool>,
    pub book_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDto {
    pub id: String,
    pub name: String,
    pub ordered: bool,
    pub filtered: bool,
    pub series_ids: Vec<String>,
    pub created_date: Option<String>,
    pub last_modified_date: Option<String>,
}

impl From<Collection> for CollectionDto {
    fn from(c: Collection) -> Self {
        Self {
            id: c.id.to_string(),
            name: c.name,
            ordered: c.ordered,
            filtered: false,
            series_ids: vec![],
            created_date: Some(c.created_date.to_rfc3339()),
            last_modified_date: Some(c.last_modified_date.to_rfc3339()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CollectionPageDto {
    pub content: Vec<CollectionDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
    pub empty: bool,
    pub first: bool,
    pub last: bool,
    pub number_of_elements: usize,
}

impl CollectionPageDto {
    pub fn new(content: Vec<CollectionDto>, total_elements: usize, page: usize, size: usize) -> Self {
        let total_pages = if size > 0 { (total_elements + size - 1) / size } else { 1 };
        Self {
            number_of_elements: content.len(),
            empty: content.is_empty(),
            first: page == 0,
            last: page + 1 >= total_pages,
            content,
            total_elements,
            total_pages,
            number: page,
            size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectionRequest {
    pub name: String,
    pub ordered: Option<bool>,
    pub series_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub ordered: Option<bool>,
    pub series_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub task_type: String,
    pub simple_type: String,
    pub status: String,
    pub priority: i32,
    pub created_date: String,
    pub last_modified_date: String,
    pub scheduled_date: Option<String>,
    pub execution_start_date: Option<String>,
    pub execution_end_date: Option<String>,
    pub duration_millis: Option<i64>,
}

impl From<Task> for TaskDto {
    fn from(t: Task) -> Self {
        let duration = match (t.execution_start_date, t.execution_end_date) {
            (Some(start), Some(end)) => Some((end - start).num_milliseconds()),
            _ => None,
        };
        Self {
            id: t.id,
            task_type: t.task_type.as_str().to_string(),
            simple_type: t.task_type.simple_type().to_string(),
            status: t.status.as_str().to_string(),
            priority: t.priority,
            created_date: t.created_date.to_rfc3339(),
            last_modified_date: t.created_date.to_rfc3339(),
            scheduled_date: t.scheduled_date.map(|d| d.to_rfc3339()),
            execution_start_date: t.execution_start_date.map(|d| d.to_rfc3339()),
            execution_end_date: t.execution_end_date.map(|d| d.to_rfc3339()),
            duration_millis: duration,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskPageDto {
    pub content: Vec<TaskDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
    pub empty: bool,
    pub first: bool,
    pub last: bool,
    pub number_of_elements: usize,
}

impl TaskPageDto {
    pub fn new(content: Vec<TaskDto>, total_elements: usize, page: usize, size: usize) -> Self {
        let total_pages = if size > 0 { (total_elements + size - 1) / size } else { 1 };
        Self {
            number_of_elements: content.len(),
            empty: content.is_empty(),
            first: page == 0,
            last: page + 1 >= total_pages,
            content,
            total_elements,
            total_pages,
            number: page,
            size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyDto {
    pub id: String,
    pub comment: String,
    pub key: String,
    pub created_date: String,
    pub last_modified_date: Option<String>,
    pub user_id: Option<String>,
}

impl From<ApiKey> for ApiKeyDto {
    fn from(k: ApiKey) -> Self {
        Self {
            id: k.id,
            comment: k.name,
            key: k.key,
            created_date: k.created_date.to_rfc3339(),
            last_modified_date: k.last_used_date.map(|d| d.to_rfc3339()),
            user_id: Some(k.user_id.to_string()),
        }
    }
}
