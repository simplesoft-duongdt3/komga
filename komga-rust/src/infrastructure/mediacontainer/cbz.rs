use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

use crate::infrastructure::mediacontainer::{BookExtractor, BookPage, MediaAnalysis};

pub struct CbzExtractor;

impl CbzExtractor {
    pub fn new() -> Self {
        Self
    }

    fn get_image_extensions() -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "webp", "bmp"]
    }

    fn is_image_file(name: &str) -> bool {
        let lower = name.to_lowercase();
        Self::get_image_extensions()
            .iter()
            .any(|ext| lower.ends_with(ext))
    }

    fn get_media_type(file_name: &str) -> String {
        let lower = file_name.to_lowercase();
        if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            "image/jpeg".to_string()
        } else if lower.ends_with(".png") {
            "image/png".to_string()
        } else if lower.ends_with(".gif") {
            "image/gif".to_string()
        } else if lower.ends_with(".webp") {
            "image/webp".to_string()
        } else if lower.ends_with(".bmp") {
            "image/bmp".to_string()
        } else {
            "image/jpeg".to_string()
        }
    }
}

impl BookExtractor for CbzExtractor {
    fn get_pages(
        &self,
        path: &Path,
    ) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        let mut pages: Vec<BookPage> = Vec::new();
        let mut names: Vec<String> = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();

            if Self::is_image_file(&name) && !name.contains("__MACOSX") {
                names.push(name);
            }
        }

        names.sort();
        for (idx, name) in names.iter().enumerate() {
            let file = archive.by_name(name)?;
            let size = file.size() as i64;

            pages.push(BookPage {
                number: (idx + 1) as i32,
                file_name: name.clone(),
                media_type: Self::get_media_type(name),
                width: None,
                height: None,
                size_bytes: Some(size),
            });
        }

        let page_count = pages.len() as i32;

        Ok(MediaAnalysis {
            media_type: "application/zip".to_string(),
            page_count,
            pages,
        })
    }

    fn get_page_content(
        &self,
        path: &Path,
        page_number: i32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        let analysis = self.get_pages(path)?;

        if page_number < 1 || page_number > analysis.page_count {
            return Err(format!("Page {} not found", page_number).into());
        }

        let page = &analysis.pages[(page_number - 1) as usize];
        let mut zip_file = archive.by_name(&page.file_name)?;

        let mut buffer = Vec::new();
        zip_file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(CbzExtractor::is_image_file("page01.jpg"));
        assert!(CbzExtractor::is_image_file("page02.JPEG"));
        assert!(CbzExtractor::is_image_file("cover.png"));
        assert!(!CbzExtractor::is_image_file("ComicInfo.xml"));
    }

    #[test]
    fn test_get_media_type() {
        assert_eq!(CbzExtractor::get_media_type("page.jpg"), "image/jpeg");
        assert_eq!(CbzExtractor::get_media_type("cover.png"), "image/png");
    }
}
