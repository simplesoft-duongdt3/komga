pub struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_thumbnail(&self, image_data: &[u8], max_size: u32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(image_data.to_vec())
    }

    pub fn get_dimensions(&self, image_data: &[u8]) -> Option<(u32, u32)> {
        None
    }

    pub fn resize(&self, image_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(image_data.to_vec())
    }

    pub fn convert_format(&self, image_data: &[u8], format: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(image_data.to_vec())
    }
}