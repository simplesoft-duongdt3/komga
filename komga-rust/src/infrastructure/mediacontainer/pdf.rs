use std::path::Path;

use pdf::file::FileOptions;
use pdf::object::Page;

use super::{BookExtractor, BookPage, MediaAnalysis};

pub struct PdfExtractor;

impl PdfExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl BookExtractor for PdfExtractor {
    fn get_pages(&self, path: &Path) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let file = FileOptions::memory().open(path)?;
        let page_count = file.num_pages() as i32;
        
        let mut pages = Vec::new();
        
        for i in 1..=page_count {
            let (width, height) = if let Ok(page) = file.get_page(i - 1) {
                let media_box = page.media_box();
                (
                    Some(media_box.width as i32),
                    Some(media_box.height as i32),
                )
            } else {
                (None, None)
            };

            pages.push(BookPage {
                number: i,
                file_name: format!("page_{}.png", i),
                media_type: "image/png".to_string(),
                width,
                height,
                size_bytes: None,
            });
        }

        Ok(MediaAnalysis {
            media_type: "application/pdf".to_string(),
            page_count,
            pages,
        })
    }

    fn get_page_content(&self, path: &Path, page_number: i32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let file = FileOptions::memory().open(path)?;
        
        let page = file.get_page(page_number as usize - 1)?;
        
        let width = 1200u32;
        let height = (width as f32 * page.media_box().height as f32 / page.media_box().width as f32) as u32;
        
        let mut canvas = image::RgbaImage::new(width, height);
        
        let renderer = pdf::render::Renderer::new();
        let _ = renderer.render_page(&mut canvas, &page, width, height);
        
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        
        image::DynamicImage::ImageRgba8(canvas)
            .write_to(&mut cursor, image::ImageFormat::Png)?;
        
        Ok(buffer)
    }
}