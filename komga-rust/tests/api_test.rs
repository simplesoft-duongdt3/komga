#[cfg(test)]
mod tests {
    use komga_rust::api::dto::{LibraryDto, LoginRequest};
    
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
}