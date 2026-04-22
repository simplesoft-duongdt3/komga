use pdf::file::FileOptions;
use std::path::Path;

use crate::infrastructure::mediacontainer::{BookExtractor, BookPage, MediaAnalysis};

pub struct PdfExtractor;

impl PdfExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl BookExtractor for PdfExtractor {
    fn get_pages(
        &self,
        path: &Path,
    ) -> Result<MediaAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let file = FileOptions::cached().open(path)?;
        let page_count = file.num_pages();

        let pages: Vec<BookPage> = (1..=page_count)
            .map(|i| BookPage {
                number: i as i32,
                file_name: format!("page_{}", i),
                media_type: "application/pdf".to_string(),
                width: None,
                height: None,
                size_bytes: None,
            })
            .collect();

        Ok(MediaAnalysis {
            media_type: "application/pdf".to_string(),
            page_count: page_count as i32,
            pages,
        })
    }

    fn get_page_content(
        &self,
        path: &Path,
        page_number: i32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let file = FileOptions::cached().open(path)?;
        let page_count = file.num_pages() as i32;

        if page_number < 1 || page_number > page_count {
            return Err(format!("Page {} not found", page_number).into());
        }

        let width = 850u32;
        let height = 1100u32;
        let img = vec![255u8; (width * height * 3) as usize];

        let mut png_bytes = Vec::new();
        {
            use image::{ImageBuffer, ImageEncoder, Rgb};
            let rgb_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_raw(width, height, img).unwrap();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
            encoder
                .write_image(
                    rgb_img.as_raw(),
                    width,
                    height,
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| format!("Failed to encode PNG: {}", e))?;
        }

        Ok(png_bytes)
    }
}
