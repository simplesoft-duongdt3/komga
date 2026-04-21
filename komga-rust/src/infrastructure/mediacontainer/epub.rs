use std::path::Path;
use crate::super::BookExtractor;

pub struct EpubExtractor;

impl EpubExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl BookExtractor for EpubExtractor {
    fn get_pages(&self, path: &Path) -> Result<crate::super::MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        Ok(crate::super::MediaAnalysis {
            media_type: "application/epub+zip".to_string(),
            page_count: 0,
            pages: vec![],
        })
    }

    fn get_page_content(&self, path: &Path, page_number: i32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::read(path).map_err(|e| e.into())
    }
}