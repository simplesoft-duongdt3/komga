#[cfg(test)]
mod tests {
    use komga_rust::api::dto::PageDto;
    use komga_rust::domain::model::book::Book;
    use komga_rust::domain::model::series::Series;
    use komga_rust::infrastructure::mediacontainer::image::ImageProcessor;
    use uuid::Uuid;

    mod book_model {
        use super::*;

        #[test]
        fn test_book_new() {
            let library_id = Uuid::new_v4();
            let series_id = Uuid::new_v4();
            let book = Book::new(
                "Test Book.cbz".to_string(),
                "/series/test-book".to_string(),
                series_id,
                library_id,
                1,
            );

            assert_eq!(book.name, "Test Book.cbz");
            assert_eq!(book.number, 1);
            assert_eq!(book.library_id, library_id);
            assert_eq!(book.series_id, series_id);
            assert!(book.id != Uuid::nil());
            assert_eq!(book.file_size, 0);
            assert!(book.file_hash.is_empty());
        }

        #[test]
        fn test_book_file_hash_update() {
            let library_id = Uuid::new_v4();
            let series_id = Uuid::new_v4();
            let mut book = Book::new(
                "Test.cbz".to_string(),
                "/test".to_string(),
                series_id,
                library_id,
                1,
            );

            book.file_hash = "abc123".to_string();
            assert_eq!(book.file_hash, "abc123");
        }

        #[test]
        fn test_book_with_metadata() {
            let library_id = Uuid::new_v4();
            let series_id = Uuid::new_v4();
            let mut book = Book::new(
                "Test.cbz".to_string(),
                "/test".to_string(),
                series_id,
                library_id,
                1,
            );

            book.file_size = 1000000;
            book.file_hash = "sha256hash".to_string();
            book.oneshot = true;

            assert_eq!(book.file_size, 1000000);
            assert_eq!(book.file_hash, "sha256hash");
            assert!(book.oneshot);
        }
    }

    mod series_model {
        use super::*;

        #[test]
        fn test_series_new() {
            let library_id = Uuid::new_v4();
            let series = Series::new(
                "Test Series".to_string(),
                "/library/test-series".to_string(),
                library_id,
            );

            assert_eq!(series.name, "Test Series");
            assert_eq!(series.url, "/library/test-series");
            assert_eq!(series.library_id, library_id);
            assert_eq!(series.book_count, 0);
            assert!(!series.oneshot);
            assert!(series.deleted_date.is_none());
        }

        #[test]
        fn test_series_with_book_count() {
            let library_id = Uuid::new_v4();
            let mut series =
                Series::new("Test Series".to_string(), "/test".to_string(), library_id);

            series.book_count = 10;
            assert_eq!(series.book_count, 10);
        }

        #[test]
        fn test_series_oneshot() {
            let library_id = Uuid::new_v4();
            let mut series = Series::new("Oneshot".to_string(), "/oneshot".to_string(), library_id);

            series.oneshot = true;
            assert!(series.oneshot);
        }

        #[test]
        fn test_series_deleted() {
            let library_id = Uuid::new_v4();
            let series = Series::new(
                "Deleted Series".to_string(),
                "/deleted".to_string(),
                library_id,
            );

            assert!(series.deleted_date.is_none());
        }
    }

    mod image_processor {
        use super::*;

        #[test]
        fn test_new_processor() {
            let _processor = ImageProcessor::new();
        }

        #[test]
        fn test_get_dimensions_invalid() {
            let processor = ImageProcessor::new();
            let result = processor.get_dimensions(b"not an image");
            assert!(result.is_none());
        }

        #[test]
        fn test_resize_invalid_input() {
            let processor = ImageProcessor::new();
            let result = processor.resize(b"invalid", 100, 100);
            assert!(result.is_err());
        }

        #[test]
        fn test_convert_format_invalid() {
            let processor = ImageProcessor::new();
            let result = processor.convert_format(b"invalid", "unknown");
            assert!(result.is_err());
        }
    }

    mod dto_tests {
        use super::*;

        #[test]
        fn test_page_dto() {
            let page = PageDto {
                number: 1,
                file_name: "page001.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                width: Some(800),
                height: Some(1200),
                size_bytes: Some(102400),
                size: None,
            };

            assert_eq!(page.number, 1);
            assert_eq!(page.file_name, "page001.jpg");
            assert!(page.width.is_some());
            assert!(page.size_bytes.is_some());
        }

        #[test]
        fn test_page_dto_serialization() {
            let page = PageDto {
                number: 2,
                file_name: "page002.jpg".to_string(),
                media_type: "image/png".to_string(),
                width: None,
                height: None,
                size_bytes: Some(204800),
                size: None,
            };

            let json = serde_json::to_string(&page).unwrap();
            assert!(json.contains("\"number\":2"));
        }

        #[test]
        fn test_page_dto_deserialization() {
            let json = r#"{"number":5,"fileName":"test.png","mediaType":"image/png"}"#;
            let page: PageDto = serde_json::from_str(json).unwrap();

            assert_eq!(page.number, 5);
            assert_eq!(page.file_name, "test.png");
            assert_eq!(page.media_type, "image/png");
        }
    }

    mod uuid_tests {
        use uuid::Uuid;

        #[test]
        fn test_uuid_generation() {
            let id = Uuid::new_v4();
            assert!(id != Uuid::nil());
        }

        #[test]
        fn test_uuid_to_string() {
            let id = Uuid::new_v4();
            let s = id.to_string();
            assert_eq!(s.len(), 36);
        }

        #[test]
        fn test_library_id_format() {
            let library_id = Uuid::new_v4();
            let library_path = format!("/library/{}", library_id);

            assert!(library_path.starts_with("/library/"));
            assert!(library_path.len() > 9);
        }

        #[test]
        fn test_series_url_format() {
            let library_id = Uuid::new_v4();
            let series_name = "My Series";
            let url = format!(
                "/library/{}/series/{}",
                library_id,
                series_name.replace(' ', "%20")
            );

            assert!(url.contains("My%20Series"));
        }
    }

    mod media_analysis {
        use komga_rust::infrastructure::mediacontainer::{BookPage, MediaAnalysis};

        #[test]
        fn test_book_page_new() {
            let page = BookPage {
                number: 1,
                file_name: "page001.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                width: Some(1920),
                height: Some(1080),
                size_bytes: Some(50000),
            };

            assert_eq!(page.number, 1);
            assert!(page.width.is_some());
        }

        #[test]
        fn test_media_analysis_new() {
            let analysis = MediaAnalysis {
                media_type: "application/zip".to_string(),
                page_count: 100,
                pages: vec![],
            };

            assert_eq!(analysis.media_type, "application/zip");
            assert_eq!(analysis.page_count, 100);
            assert!(analysis.pages.is_empty());
        }

        #[test]
        fn test_media_analysis_with_pages() {
            let pages = vec![
                BookPage {
                    number: 1,
                    file_name: "page1.jpg".to_string(),
                    media_type: "image/jpeg".to_string(),
                    width: None,
                    height: None,
                    size_bytes: None,
                },
                BookPage {
                    number: 2,
                    file_name: "page2.jpg".to_string(),
                    media_type: "image/jpeg".to_string(),
                    width: None,
                    height: None,
                    size_bytes: None,
                },
            ];

            let analysis = MediaAnalysis {
                media_type: "application/zip".to_string(),
                page_count: 2,
                pages,
            };

            assert_eq!(analysis.page_count, 2);
            assert_eq!(analysis.pages.len(), 2);
        }
    }

    mod path_utils {
        #[test]
        fn test_file_extension_detection() {
            let test_cases = vec![
                ("book.cbz", Some("cbz")),
                ("book.zip", Some("zip")),
                ("book.pdf", Some("pdf")),
                ("book.epub", Some("epub")),
                ("book.CBZ", Some("cbz")),
                ("book", None),
                ("", None),
            ];

            for (input, expected) in test_cases {
                let path = std::path::PathBuf::from(input);
                let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());

                if let Some(exp) = expected {
                    assert_eq!(ext.as_deref(), Some(exp), "Failed for input: {}", input);
                } else {
                    assert!(ext.is_none(), "Failed for input: {}", input);
                }
            }
        }

        #[test]
        fn test_supported_formats() {
            let supported = ["cbz", "cbr", "zip", "rar", "pdf", "epub"];

            assert!(supported.contains(&"cbz"));
            assert!(supported.contains(&"pdf"));
            assert!(supported.contains(&"epub"));
            assert!(!supported.contains(&"txt"));
        }
    }
}
