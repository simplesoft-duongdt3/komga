use image::ImageReader;

pub struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_thumbnail(
        &self,
        image_data: &[u8],
        max_size: u32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let img = ImageReader::new(std::io::Cursor::new(image_data))
            .with_guessed_format()?
            .decode()?;

        let thumbnail = img.thumbnail(max_size, max_size);

        let mut bytes = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
            encoder
                .write_image(
                    thumbnail.as_bytes(),
                    thumbnail.width(),
                    thumbnail.height(),
                    thumbnail.color().into(),
                )
                .map_err(|e| format!("Failed to encode thumbnail: {}", e))?;
        }

        Ok(bytes)
    }

    pub fn get_dimensions(&self, image_data: &[u8]) -> Option<(u32, u32)> {
        let img = ImageReader::new(std::io::Cursor::new(image_data))
            .with_guessed_format()
            .ok()?
            .decode()
            .ok()?;

        Some((img.width(), img.height()))
    }

    pub fn resize(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let img = ImageReader::new(std::io::Cursor::new(image_data))
            .with_guessed_format()?
            .decode()?;

        let resized = img.resize(width, height, image::imageops::FilterType::Lanczos3);

        let mut bytes = Vec::new();
        {
            use image::ImageEncoder;
            let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
            encoder
                .write_image(
                    resized.as_bytes(),
                    resized.width(),
                    resized.height(),
                    resized.color().into(),
                )
                .map_err(|e| format!("Failed to encode image: {}", e))?;
        }

        Ok(bytes)
    }

    pub fn convert_format(
        &self,
        image_data: &[u8],
        format: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let img = ImageReader::new(std::io::Cursor::new(image_data))
            .with_guessed_format()?
            .decode()?;

        match format.to_lowercase().as_str() {
            "jpeg" | "jpg" => {
                let mut bytes = Vec::new();
                {
                    use image::ImageEncoder;
                    let encoder = image::codecs::jpeg::JpegEncoder::new(&mut bytes);
                    encoder
                        .write_image(
                            img.as_bytes(),
                            img.width(),
                            img.height(),
                            img.color().into(),
                        )
                        .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
                }
                Ok(bytes)
            }
            "png" => {
                let mut bytes = Vec::new();
                {
                    use image::ImageEncoder;
                    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
                    encoder
                        .write_image(
                            img.as_bytes(),
                            img.width(),
                            img.height(),
                            img.color().into(),
                        )
                        .map_err(|e| format!("Failed to encode PNG: {}", e))?;
                }
                Ok(bytes)
            }
            _ => Err(format!("Unsupported format: {}", format).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_processor_new() {
        let processor = ImageProcessor::new();
        assert!(std::mem::size_of_val(&processor) > 0);
    }
}
