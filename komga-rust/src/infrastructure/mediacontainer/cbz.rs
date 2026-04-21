use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use super::{BookExtractor, BookPage, MediaAnalysis};

pub struct CbzExtractor;

impl CbzExtractor {
    pub fn new() -> Self {
        Self
    }

    fn is_image_file(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.ends_with(".jpg") 
            || lower.ends_with(".jpeg") 
            || lower.ends_with(".png") 
            || lower.ends_with(".gif") 
            || lower.ends_with(".webp")
    }
}

impl BookExtractor for CbzExtractor {
    fn get_pages(&self, path: &Path) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let mut pages = Vec::new();
        let mut names: Vec<String> = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            if Self::is_image_file(&name) && !name.contains("__MACOSX") {
                names.push(name);
            }
        }

        names.sort();

        for (i, name) in names.iter().enumerate() {
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
            media_type: "application/zip".to_string(),
            page_count: pages.len() as i32,
            pages,
        })
    }

    fn get_page_content(&self, path: &Path, page_number: i32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let mut names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            if Self::is_image_file(&name) && !name.contains("__MACOSX") {
                names.push(name);
            }
        }

        names.sort();

        if page_number as usize >= names.len() {
            return Err("Page not found".into());
        }

        let mut file = archive.by_name(&names[page_number as usize - 1])?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}

impl CbzExtractor {
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