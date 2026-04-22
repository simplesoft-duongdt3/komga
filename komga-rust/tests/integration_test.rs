#[cfg(test)]
mod tests {
    use komga_rust::api::dto::{BookDto, LibraryDto, LoginRequest, SeriesDto, TaskDto};

    #[test]
    fn test_library_dto_serialization() {
        let library = LibraryDto {
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

        let json = serde_json::to_string(&library).unwrap();
        assert!(json.contains("Test Library"));
    }

    #[test]
    fn test_login_request_serialization() {
        let login = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&login).unwrap();
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_series_dto_serialization() {
        let series = SeriesDto {
            id: "test-id".to_string(),
            name: "Test Series".to_string(),
            url: "/library/1/series/test".to_string(),
            library_id: "lib-1".to_string(),
            book_count: 10,
            oneshot: false,
        };

        let json = serde_json::to_string(&series).unwrap();
        assert!(json.contains("Test Series"));
    }

    #[test]
    fn test_book_dto_serialization() {
        let book = BookDto {
            id: "test-id".to_string(),
            name: "Test Book.cbz".to_string(),
            url: "/library/1/series/test/Book.cbz".to_string(),
            series_id: "series-1".to_string(),
            file_size: 1024,
            number: 1,
            library_id: "lib-1".to_string(),
            file_hash: "abc123".to_string(),
            oneshot: false,
            file_hash_koreader: "".to_string(),
        };

        let json = serde_json::to_string(&book).unwrap();
        assert!(json.contains("Test Book"));
    }

    #[test]
    fn test_task_dto_serialization() {
        let task = TaskDto {
            id: "task-1".to_string(),
            task_type: "ScanLibrary".to_string(),
            status: "QUEUED".to_string(),
            priority: 4,
            created_date: "2024-01-01T00:00:00Z".to_string(),
            scheduled_date: None,
            execution_start_date: None,
            execution_end_date: None,
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("ScanLibrary"));
    }
}
