use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use std::io::Cursor;

pub struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_thumbnail(&self, image_data: &[u8], max_size: u32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let img = image::load_from_memory(image_data)?;
        
        let (width, height) = img.dimensions();
        
        let ratio = max_size as f32 / width.max(height) as f32;
        let new_width = (width as f32 * ratio) as u32;
        let new_height = (height as f32 * ratio) as u32;
        
        let thumbnail = img.thumbnail(new_width, new_height);
        
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        
        thumbnail.write_to(&mut cursor, image::ImageFormat::Jpeg)?;
        
        Ok(buffer)
    }

    pub fn get_dimensions(&self, image_data: &[u32, 1]) -> Option<(u32, u32)> {
        match image::load_from_memory(image_data) {
            Ok(img) => Some(img.dimensions()),
            Err(_) => None,
        }
    }

    pub fn resize(&self, image_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let img = image::load_from_memory(image_data)?;
        let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        
        resized.write_to(&mut cursor, image::ImageFormat::Png)?;
        
        Ok(buffer)
    }

    pub fn convert_format(&self, image_data: &[u8], format: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let img = image::load_from_memory(image_data)?;
        
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        
        match format.to_lowercase().as_str() {
            "jpeg" | "jpg" => img.write_to(&mut cursor, image::ImageFormat::Jpeg)?,
            "png" => img.write_to(&mut cursor, image::ImageFormat::Png)?,
            "webp" => img.write_to(&mut cursor, image::ImageFormat::WebP)?,
            "gif" => img.write_to(&mut cursor, image::ImageFormat::Gif)?,
            _ => return Err("Unsupported format".into()),
        }
        
        Ok(buffer)
    }
}