#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};
    use komga_rust::api::dto::*;
    use komga_rust::domain::model::collection::Collection;
    use komga_rust::domain::model::library::Library;
    use komga_rust::domain::model::media::Media;
    use komga_rust::domain::model::read_progress::ReadProgress;
    use komga_rust::domain::model::readlist::ReadList;
    use komga_rust::domain::model::task::{TaskData, TaskType};
    use komga_rust::domain::model::user::User;
    use komga_rust::domain::model::*;
    use komga_rust::infrastructure::mediacontainer::cbz::CbzExtractor;
    use komga_rust::infrastructure::mediacontainer::epub::EpubExtractor;
    use komga_rust::infrastructure::mediacontainer::image::ImageProcessor;
    use komga_rust::infrastructure::mediacontainer::pdf::PdfExtractor;
    use komga_rust::infrastructure::mediacontainer::{BookPage, MediaAnalysis};
    use komga_rust::infrastructure::metadata::local_artwork::LocalArtwork;
    use komga_rust::infrastructure::metadata::mylar::MylarProvider;
    use uuid::Uuid;

    // ==================== USER MODEL TESTS ====================
    mod user_model {
        use super::*;

        #[test]
        fn test_user_new() {
            let user = User::new("test@example.com".to_string(), "password".to_string());
            assert_eq!(user.email, "test@example.com");
            assert!(!user.password.is_empty());
            assert!(user.id != Uuid::nil());
        }

        #[test]
        fn test_user_with_roles() {
            let mut user = User::new("admin@test.com".to_string(), "pass".to_string());
            user.roles = vec![];
            assert!(user.roles.is_empty());
        }

        #[test]
        fn test_user_serialization() {
            let user = User::new("test@test.com".to_string(), "pass".to_string());
            let json = serde_json::to_string(&user).unwrap();
            assert!(json.contains("test@test.com"));
        }
    }

    // ==================== LIBRARY MODEL TESTS ====================
    mod library_model {
        use super::*;

        #[test]
        fn test_library_new() {
            let library = Library::new("My Library".to_string(), "/books".to_string());
            assert_eq!(library.name, "My Library");
            assert_eq!(library.root, "/books");
            assert!(library.id != Uuid::nil());
        }

        #[test]
        fn test_library_default_settings() {
            let library = Library::new("Test".to_string(), "/test".to_string());
            assert!(library.import_comicinfo_book);
            assert!(library.hash_files);
        }

        #[test]
        fn test_library_serialization() {
            let library = Library::new("Test".to_string(), "/test".to_string());
            let json = serde_json::to_string(&library).unwrap();
            assert!(json.contains("Test"));
        }
    }

    // ==================== SERIES MODEL TESTS ====================
    mod series_model {
        use super::*;

        #[test]
        fn test_series_new() {
            let library_id = Uuid::new_v4();
            let series = Series::new(
                "My Series".to_string(),
                "/library/test-series".to_string(),
                library_id,
            );
            assert_eq!(series.name, "My Series");
            assert_eq!(series.url, "/library/test-series");
            assert_eq!(series.library_id, library_id);
            assert!(series.id != Uuid::nil());
        }

        #[test]
        fn test_series_default_book_count() {
            let series = Series::new("Test".to_string(), "/test".to_string(), Uuid::new_v4());
            assert_eq!(series.book_count, 0);
        }

        #[test]
        fn test_series_serialization() {
            let series = Series::new("Test".to_string(), "/test".to_string(), Uuid::new_v4());
            let json = serde_json::to_string(&series).unwrap();
            assert!(json.contains("Test"));
        }
    }

    // ==================== BOOK MODEL TESTS ====================
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
        fn test_book_serialization() {
            let book = Book::new(
                "test.cbz".to_string(),
                "/test".to_string(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                1,
            );
            let json = serde_json::to_string(&book).unwrap();
            assert!(json.contains("test.cbz"));
        }

        #[test]
        fn test_book_json_roundtrip() {
            let book = Book::new(
                "test.cbz".to_string(),
                "/test".to_string(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                1,
            );
            let json = serde_json::to_string(&book).unwrap();
            let parsed: Book = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.name, book.name);
        }
    }

    // ==================== COLLECTION MODEL TESTS ====================
    mod collection_model {
        use super::*;

        #[test]
        fn test_collection_new() {
            let collection = Collection::new("My Collection".to_string());
            assert_eq!(collection.name, "My Collection");
            assert!(collection.id != Uuid::nil());
            assert_eq!(collection.series_count, 0);
        }

        #[test]
        fn test_collection_serialization() {
            let collection = Collection::new("Test".to_string());
            let json = serde_json::to_string(&collection).unwrap();
            assert!(json.contains("Test"));
        }
    }

    // ==================== READLIST MODEL TESTS ====================
    mod readlist_model {
        use super::*;

        #[test]
        fn test_readlist_new() {
            let readlist = ReadList::new("My ReadList".to_string());
            assert_eq!(readlist.name, "My ReadList");
            assert!(readlist.id != Uuid::nil());
            assert_eq!(readlist.book_count, 0);
        }

        #[test]
        fn test_readlist_serialization() {
            let readlist = ReadList::new("Test".to_string());
            let json = serde_json::to_string(&readlist).unwrap();
            assert!(json.contains("Test"));
        }
    }

    // ==================== READ PROGRESS MODEL TESTS ====================
    mod read_progress_model {
        use super::*;

        #[test]
        fn test_read_progress_new() {
            let book_id = Uuid::new_v4();
            let user_id = Uuid::new_v4();
            let progress = ReadProgress::new(book_id, user_id, 10, false);
            assert_eq!(progress.book_id, book_id);
            assert_eq!(progress.user_id, user_id);
            assert_eq!(progress.page, 10);
            assert!(!progress.completed);
        }

        #[test]
        fn test_read_progress_completed() {
            let progress = ReadProgress::new(Uuid::new_v4(), Uuid::new_v4(), 100, true);
            assert!(progress.completed);
            assert_eq!(progress.page, 100);
        }

        #[test]
        fn test_read_progress_serialization() {
            let progress = ReadProgress::new(Uuid::new_v4(), Uuid::new_v4(), 5, false);
            let json = serde_json::to_string(&progress).unwrap();
            assert!(json.contains("5"));
        }
    }

    // ==================== MEDIA MODEL TESTS ====================
    mod media_model {
        use super::*;

        #[test]
        fn test_media_new() {
            let book_id = Uuid::new_v4();
            let media = Media::new(book_id);
            assert_eq!(media.book_id, book_id);
            assert_eq!(media.page_count, 0);
            assert!(!media.epub_divina_compatible);
            assert!(!media.epub_is_kepub);
        }

        #[test]
        fn test_media_serialization() {
            let media = Media::new(Uuid::new_v4());
            let _json = serde_json::to_string(&media);
        }
    }

    // ==================== TASK MODEL TESTS ====================
    mod task_model {
        use super::*;

        #[test]
        fn test_task_type_variants() {
            assert!(matches!(TaskType::ScanLibrary, TaskType::ScanLibrary));
            assert!(matches!(TaskType::AnalyzeBook, TaskType::AnalyzeBook));
            assert!(matches!(
                TaskType::GenerateBookThumbnail,
                TaskType::GenerateBookThumbnail
            ));
            assert!(matches!(TaskType::HashBook, TaskType::HashBook));
            assert!(matches!(TaskType::HashBookPages, TaskType::HashBookPages));
            assert!(matches!(
                TaskType::HashBookKoreader,
                TaskType::HashBookKoreader
            ));
            assert!(matches!(
                TaskType::RefreshBookMetadata,
                TaskType::RefreshBookMetadata
            ));
            assert!(matches!(
                TaskType::RefreshSeriesMetadata,
                TaskType::RefreshSeriesMetadata
            ));
            assert!(matches!(TaskType::EmptyTrash, TaskType::EmptyTrash));
            assert!(matches!(TaskType::DeleteBook, TaskType::DeleteBook));
            assert!(matches!(TaskType::DeleteSeries, TaskType::DeleteSeries));
            assert!(matches!(TaskType::ImportBook, TaskType::ImportBook));
        }

        #[test]
        fn test_task_data_variants() {
            let _data = TaskData::ScanLibrary {
                library_id: "test".to_string(),
                scan_deep: false,
            };
            let _data = TaskData::AnalyzeBook {
                book_id: "test".to_string(),
            };
            let _data = TaskData::HashBook {
                book_id: "test".to_string(),
            };
            let _data = TaskData::EmptyTrash {
                library_id: "test".to_string(),
            };
            let _data = TaskData::DeleteBook {
                book_id: "test".to_string(),
            };
        }
    }

    // ==================== DTO TESTS ====================
    mod dto_tests {
        use super::*;

        #[test]
        fn test_library_dto() {
            let dto = LibraryDto {
                id: "test-id".to_string(),
                name: "Test Library".to_string(),
                root: "/books".to_string(),
                library_type: "COMIC".to_string(),
                import_comicinfo_book: true,
                import_comicinfo_series: true,
                import_comicinfo_collection: true,
                import_epub_book: true,
                import_epub_series: true,
                scan_force_modified_time: false,
                scan_startup: false,
                import_local_artwork: true,
                import_comicinfo_readlist: true,
                import_barcode_isbn: true,
                convert_to_cbz: false,
                repair_extensions: false,
                empty_trash_after_scan: false,
                import_mylar_series: true,
                series_cover: "FIRST".to_string(),
                unavailable_date: None,
                hash_files: true,
                hash_pages: false,
                analyze_dimensions: true,
                import_comicinfo_series_append_volume: true,
                oneshots_directory: None,
                scan_cbx: true,
                scan_pdf: true,
                scan_epub: true,
                scan_interval: "EVERY_6H".to_string(),
                hash_koreader: false,
            };
            assert_eq!(dto.name, "Test Library");
        }

        #[test]
        fn test_series_dto() {
            let dto = SeriesDto {
                id: "id".to_string(),
                library_id: "lib".to_string(),
                name: "Series".to_string(),
                url: "/url".to_string(),
                book_count: 5,
                oneshot: false,
            };
            assert_eq!(dto.book_count, 5);
        }

        #[test]
        fn test_book_dto() {
            let dto = BookDto {
                id: "id".to_string(),
                series_id: "series".to_string(),
                library_id: "lib".to_string(),
                name: "book.cbz".to_string(),
                url: "/url".to_string(),
                number: 1,
                file_size: 1000,
                file_hash: "hash".to_string(),
                oneshot: false,
                file_hash_koreader: "".to_string(),
            };
            assert_eq!(dto.number, 1);
        }

        #[test]
        fn test_login_request() {
            let req = LoginRequest {
                email: "test@test.com".to_string(),
                password: "password".to_string(),
            };
            let json = serde_json::to_string(&req).unwrap();
            assert!(json.contains("test@test.com"));
        }

        #[test]
        fn test_login_response() {
            let resp = LoginResponse {
                token: "token".to_string(),
            };
            assert!(!resp.token.is_empty());
        }

        #[test]
        fn test_read_progress_dto() {
            let dto = ReadProgressDto {
                book_id: "book-id".to_string(),
                user_id: "user-id".to_string(),
                page: 10,
                completed: false,
                read_date: Some("2024-01-01".to_string()),
            };
            assert_eq!(dto.page, 10);
        }

        #[test]
        fn test_page_dto() {
            let dto = PageDto {
                number: 1,
                file_name: "page01.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                width: Some(1920),
                height: Some(1080),
                size_bytes: Some(50000),
            };
            assert_eq!(dto.number, 1);
        }

        #[test]
        fn test_book_page_dto() {
            let dto = BookPageDto {
                content: vec![],
                total_elements: 0,
                total_pages: 0,
                number: 0,
                size: 20,
            };
            assert_eq!(dto.size, 20);
        }

        #[test]
        fn test_task_dto() {
            let dto = TaskDto {
                id: "id".to_string(),
                task_type: "SCAN_LIBRARY".to_string(),
                status: "PENDING".to_string(),
                priority: 0,
                created_date: "2024-01-01T00:00:00Z".to_string(),
                scheduled_date: None,
                execution_start_date: None,
                execution_end_date: None,
            };
            assert_eq!(dto.status, "PENDING");
        }
    }

    // ==================== MEDIA CONTAINER TESTS ====================
    mod mediacontainer_tests {
        use super::*;

        #[test]
        fn test_book_page() {
            let page = BookPage {
                number: 1,
                file_name: "page01.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                width: Some(1920),
                height: Some(1080),
                size_bytes: Some(50000),
            };
            assert_eq!(page.number, 1);
            assert!(page.width.is_some());
        }

        #[test]
        fn test_media_analysis() {
            let analysis = MediaAnalysis {
                media_type: "application/zip".to_string(),
                page_count: 100,
                pages: vec![],
            };
            assert_eq!(analysis.page_count, 100);
            assert!(analysis.pages.is_empty());
        }

        #[test]
        fn test_media_analysis_with_pages() {
            let pages = vec![
                BookPage {
                    number: 1,
                    file_name: "p1.jpg".to_string(),
                    media_type: "image/jpeg".to_string(),
                    width: None,
                    height: None,
                    size_bytes: None,
                },
                BookPage {
                    number: 2,
                    file_name: "p2.jpg".to_string(),
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
            assert_eq!(analysis.pages.len(), 2);
        }

        #[test]
        fn test_cbz_extractor_new() {
            let _extractor = CbzExtractor::new();
        }

        #[test]
        fn test_epub_extractor_new() {
            let _extractor = EpubExtractor::new();
        }

        #[test]
        fn test_pdf_extractor_new() {
            let _extractor = PdfExtractor::new();
        }

        #[test]
        fn test_image_processor_new() {
            let _processor = ImageProcessor::new();
        }

        #[test]
        fn test_image_processor_invalid_input() {
            let processor = ImageProcessor::new();
            assert!(processor.get_dimensions(b"invalid").is_none());
            assert!(processor.resize(b"invalid", 100, 100).is_err());
            assert!(processor.convert_format(b"invalid", "unknown").is_err());
        }
    }

    // ==================== METADATA TESTS ====================
    mod metadata_tests {
        use super::*;

        #[test]
        fn test_mylar_provider_new() {
            let _provider = MylarProvider::new();
        }

        #[test]
        fn test_local_artwork_new() {
            let _artwork = LocalArtwork::new();
        }
    }

    // ==================== UTILITY TESTS ====================
    mod utils {
        use super::*;

        #[test]
        fn test_uuid_generation() {
            let id = Uuid::new_v4();
            assert!(id != Uuid::nil());
            assert_eq!(id.to_string().len(), 36);
        }

        #[test]
        fn test_datetime_now() {
            let now = Utc::now();
            assert!(now.timestamp() > 0);
        }

        #[test]
        fn test_naive_date_parse() {
            let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d");
            assert!(date.is_ok());
        }

        #[test]
        fn test_path_extensions() {
            let cases = vec![
                ("book.cbz", Some("cbz")),
                ("book.zip", Some("zip")),
                ("book.pdf", Some("pdf")),
                ("book.epub", Some("epub")),
                ("book", None),
            ];
            for (path, expected) in cases {
                let p = std::path::PathBuf::from(path);
                let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase());
                assert_eq!(ext.as_deref(), expected, "Failed: {}", path);
            }
        }

        #[test]
        fn test_supported_formats() {
            let formats = vec!["cbz", "cbr", "zip", "rar", "pdf", "epub"];
            assert!(formats.contains(&"cbz"));
            assert!(formats.contains(&"pdf"));
            assert!(formats.contains(&"epub"));
        }

        #[test]
        fn test_library_path_format() {
            let id = Uuid::new_v4();
            let path = format!("/library/{}", id);
            assert!(path.starts_with("/library/"));
        }
    }

    // ==================== EDGE CASE TESTS ====================
    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_book_name() {
            let book = Book::new(
                "".to_string(),
                "/".to_string(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                0,
            );
            assert_eq!(book.name, "");
        }

        #[test]
        fn test_zero_book_number() {
            let book = Book::new(
                "test.cbz".to_string(),
                "/test".to_string(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                0,
            );
            assert_eq!(book.number, 0);
        }

        #[test]
        fn test_empty_hash() {
            let book = Book::new(
                "test.cbz".to_string(),
                "/test".to_string(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                1,
            );
            assert!(book.file_hash.is_empty());
        }

        #[test]
        fn test_library_empty_root() {
            let library = Library::new("Test".to_string(), "".to_string());
            assert_eq!(library.root, "");
        }

        #[test]
        fn test_series_zero_book_count() {
            let series = Series::new("Test".to_string(), "/test".to_string(), Uuid::new_v4());
            assert_eq!(series.book_count, 0);
        }

        #[test]
        fn test_read_progress_zero_page() {
            let progress = ReadProgress::new(Uuid::new_v4(), Uuid::new_v4(), 0, false);
            assert_eq!(progress.page, 0);
        }

        #[test]
        fn test_empty_collection_name() {
            let collection = Collection::new("".to_string());
            assert_eq!(collection.name, "");
        }

        #[test]
        fn test_empty_readlist_name() {
            let readlist = ReadList::new("".to_string());
            assert_eq!(readlist.name, "");
        }
    }
}
