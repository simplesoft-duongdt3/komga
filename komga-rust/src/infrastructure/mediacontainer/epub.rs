use epub::doc::EpubDoc;
use std::path::Path;

use crate::infrastructure::mediacontainer::{BookExtractor, BookPage, MediaAnalysis};

pub struct EpubExtractor;

impl EpubExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl BookExtractor for EpubExtractor {
    fn get_pages(
        &self,
        path: &Path,
    ) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let mut doc = EpubDoc::new(path)?;
        let page_count = doc.get_num_pages();

        let pages: Vec<BookPage> = (1..=page_count)
            .map(|i| BookPage {
                number: i as i32,
                file_name: format!("page_{}", i),
                media_type: "application/xhtml+xml".to_string(),
                width: None,
                height: None,
                size_bytes: None,
            })
            .collect();

        Ok(MediaAnalysis {
            media_type: "application/epub+zip".to_string(),
            page_count: page_count as i32,
            pages,
        })
    }

    fn get_page_content(
        &self,
        path: &Path,
        page_number: i32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut doc = EpubDoc::new(path)?;

        let total_pages = doc.get_num_pages() as i32;
        if page_number < 1 || page_number > total_pages {
            return Err(format!("Page {} not found", page_number).into());
        }

        doc.set_current_page((page_number - 1) as usize);

        if let Some((content, _)) = doc.get_current_str() {
            Ok(content.into_bytes())
        } else {
            Err("Failed to get page content".into())
        }
    }
}
