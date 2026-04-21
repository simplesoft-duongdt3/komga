pub mod cbz;
pub mod epub;
pub mod pdf;
pub mod image;

use std::path::Path;

#[derive(Debug, Clone)]
pub struct BookPage {
    pub number: i32,
    pub file_name: String,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MediaAnalysis {
    pub media_type: String,
    pub page_count: i32,
    pub pages: Vec<BookPage>,
}

pub trait BookExtractor {
    fn get_pages(&self, path: &Path) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>>;
    fn get_page_content(&self, path: &Path, page_number: i32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}