use std::path::Path;

use super::{BookExtractor, BookPage, MediaAnalysis};

pub struct EpubExtractor;

impl EpubExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl BookExtractor for EpubExtractor {
    fn get_pages(&self, path: &Path) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let epub = epub::Doc::new(path)?;
        
        let mut pages = Vec::new();
        let mut resources = epub.resources();
        
        let mut image_resources: Vec<_> = resources
            .filter(|(name, _)| {
                let name_lower = name.to_lowercase();
                name_lower.ends_with(".jpg") 
                    || name_lower.ends_with(".jpeg") 
                    || name_lower.ends_with(".png") 
                    || name_lower.ends_with(".gif")
                    || name_lower.ends_with(".webp")
            })
            .collect();
        
        image_resources.sort_by(|a, b| a.0.cmp(b.0));

        for (i, (name, _)) in image_resources.iter().enumerate() {
            pages.push(BookPage {
                number: (i + 1) as i32,
                file_name: name.clone(),
                media_type: Self::get_media_type(name),
                width: None,
                height: None,
                size_bytes: None,
            });
        }

        Ok(MediaAnalysis {
            media_type: "application/epub+zip".to_string(),
            page_count: pages.len() as i32,
            pages,
        })
    }

    fn get_page_content(&self, path: &Path, page_number: i32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut epub = epub::Doc::new(path)?;
        let mut resources = epub.resources();
        
        let image_resources: Vec<_> = resources
            .filter(|(name, _)| {
                let name_lower = name.to_lowercase();
                name_lower.ends_with(".jpg") 
                    || name_lower.ends_with(".jpeg") 
                    || name_lower.ends_with(".png") 
                    || name_lower.ends_with(".gif")
                    || name_lower.ends_with(".webp")
            })
            .collect();
        
        let mut sorted: Vec<_> = image_resources.collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        if page_number as usize >= sorted.len() {
            return Err("Page not found".into());
        }

        let resource_name = sorted[page_number as usize - 1].0.clone();
        
        if let Some(content) = epub.get_resource(&resource_name) {
            Ok(content)
        } else {
            Err("Resource not found".into())
        }
    }
}

impl EpubExtractor {
    fn get_media_type(name: &str) -> String {
        let lower = name.to_lowercase();
        if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            "image/jpeg".to_string()
        } else if lower.ends_with(".png") {
            "image/png".to_string()
        } else if lower.ends_with(".gif") {
            "image/gif".to_string()
        } else if lower.ends_with(".webp") {
            "image/webp".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }
}