use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::domain::model::{Library, Series, Book};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
    pub root: String,
    #[serde(rename = "type")]
    pub library_type: String,
    pub import_comicinfo_book: bool,
    pub import_comicinfo_series: bool,
    pub import_comicinfo_collection: bool,
    pub import_epub_book: bool,
    pub import_epub_series: bool,
    pub scan_force_modified_time: bool,
    pub scan_startup: bool,
    pub import_local_artwork: bool,
    pub import_comicinfo_readlist: bool,
    pub import_barcode_isbn: bool,
    pub convert_to_cbz: bool,
    pub repair_extensions: bool,
    pub empty_trash_after_scan: bool,
    pub import_mylar_series: bool,
    pub series_cover: String,
    pub unavailable_date: Option<String>,
    pub hash_files: bool,
    pub hash_pages: bool,
    pub analyze_dimensions: bool,
    pub import_comicinfo_series_append_volume: bool,
    pub oneshots_directory: Option<String>,
    pub scan_cbx: bool,
    pub scan_pdf: bool,
    pub scan_epub: bool,
    pub scan_interval: String,
    pub hash_koreader: bool,
}

impl From<Library> for LibraryDto {
    fn from(lib: Library) -> Self {
        Self {
            id: lib.id.to_string(),
            name: lib.name,
            root: lib.root,
            library_type: "COMIC".to_string(),
            import_comicinfo_book: lib.import_comicinfo_book,
            import_comicinfo_series: lib.import_comicinfo_series,
            import_comicinfo_collection: lib.import_comicinfo_collection,
            import_epub_book: lib.import_epub_book,
            import_epub_series: lib.import_epub_series,
            scan_force_modified_time: lib.scan_force_modified_time,
            scan_startup: lib.scan_startup,
            import_local_artwork: lib.import_local_artwork,
            import_comicinfo_readlist: lib.import_comicinfo_readlist,
            import_barcode_isbn: lib.import_barcode_isbn,
            convert_to_cbz: lib.convert_to_cbz,
            repair_extensions: lib.repair_extensions,
            empty_trash_after_scan: lib.empty_trash_after_scan,
            import_mylar_series: lib.import_mylar_series,
            series_cover: format!("{:?}", lib.series_cover),
            unavailable_date: lib.unavailable_date.map(|d| d.to_rfc3339()),
            hash_files: lib.hash_files,
            hash_pages: lib.hash_pages,
            analyze_dimensions: lib.analyze_dimensions,
            import_comicinfo_series_append_volume: lib.import_comicinfo_series_append_volume,
            oneshots_directory: lib.oneshots_directory,
            scan_cbx: lib.scan_cbx,
            scan_pdf: lib.scan_pdf,
            scan_epub: lib.scan_epub,
            scan_interval: format!("{:?}", lib.scan_interval),
            hash_koreader: lib.hash_koreader,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesDto {
    pub id: String,
    pub name: String,
    pub url: String,
    pub library_id: String,
    pub book_count: i32,
    pub oneshot: bool,
}

impl From<Series> for SeriesDto {
    fn from(s: Series) -> Self {
        Self {
            id: s.id.to_string(),
            name: s.name,
            url: s.url,
            library_id: s.library_id.to_string(),
            book_count: s.book_count,
            oneshot: s.oneshot,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesPageDto {
    pub content: Vec<SeriesDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDto {
    pub id: String,
    pub name: String,
    pub url: String,
    pub series_id: String,
    pub file_size: i64,
    pub number: i32,
    pub library_id: String,
    pub file_hash: String,
    pub oneshot: bool,
    pub file_hash_koreader: String,
}

impl From<Book> for BookDto {
    fn from(b: Book) -> Self {
        Self {
            id: b.id.to_string(),
            name: b.name,
            url: b.url,
            series_id: b.series_id.to_string(),
            file_size: b.file_size,
            number: b.number,
            library_id: b.library_id.to_string(),
            file_hash: b.file_hash,
            oneshot: b.oneshot,
            file_hash_koreader: b.file_hash_koreader,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookPageDto {
    pub content: Vec<BookDto>,
    pub total_elements: usize,
    pub total_pages: usize,
    pub number: usize,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDto {
    pub number: i32,
    pub file_name: String,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadProgressDto {
    pub book_id: String,
    pub user_id: String,
    pub page: i32,
    pub completed: bool,
    pub read_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadProgressUpdateRequest {
    pub page: Option<i32>,
    pub completed: Option<bool>,
}