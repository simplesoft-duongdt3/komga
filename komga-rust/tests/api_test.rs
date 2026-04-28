#[cfg(test)]
mod tests {
    use komga_rust::api::dto::{LibraryDto, LoginRequest};
    
    #[test]
    fn test_library_dto_serialization() {
        let library = LibraryDto {
            id: "test-id".to_string(),
            name: "Test Library".to_string(),
            root: "/books".to_string(),
            import_comic_info_book: true,
            import_comic_info_series: true,
            import_comic_info_collection: true,
            import_epub_book: true,
            import_epub_series: true,
            scan_force_modified_time: false,
            scan_on_startup: false,
            import_local_artwork: true,
            import_comic_info_read_list: true,
            import_barcode_isbn: true,
            convert_to_cbz: false,
            repair_extensions: false,
            empty_trash_after_scan: false,
            import_mylar_series: true,
            series_cover: "FIRST".to_string(),
            scan_directory_exclusions: vec![],
            scan_cbx: true,
            scan_pdf: true,
            scan_epub: true,
            scan_interval: "EVERY_6H".to_string(),
            hash_files: true,
            hash_pages: false,
            analyze_dimensions: true,
            import_comic_info_series_append_volume: true,
            hash_koreader: false,
            oneshots_directory: None,
            unavailable: None,
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
}
