use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: Uuid,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub name: String,
    pub root: String,
    pub import_comicinfo_book: bool,
    pub import_comicinfo_series: bool,
    pub import_comicinfo_collection: bool,
    pub import_epub_book: bool,
    pub import_epub_series: bool,
    pub scan_force_modified_time: bool,
    pub scan_startup: bool,
    pub import_local_artwork: bool,
    pub import_comicinfo_readlist: bool,
    pub import_barcode_isbn: bool,
    pub convert_to_cbz: bool,
    pub repair_extensions: bool,
    pub empty_trash_after_scan: bool,
    pub import_mylar_series: bool,
    pub series_cover: SeriesCover,
    pub unavailable_date: Option<DateTime<Utc>>,
    pub hash_files: bool,
    pub hash_pages: bool,
    pub analyze_dimensions: bool,
    pub import_comicinfo_series_append_volume: bool,
    pub oneshots_directory: Option<String>,
    pub scan_cbx: bool,
    pub scan_pdf: bool,
    pub scan_epub: bool,
    pub scan_interval: ScanInterval,
    pub hash_koreader: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeriesCover {
    First,
    Front,
    Back,
    Spine,
    Alternative,
}

impl Default for SeriesCover {
    fn default() -> Self {
        SeriesCover::First
    }
}

impl TryFrom<&str> for SeriesCover {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_uppercase().as_str() {
            "FIRST" => Ok(SeriesCover::First),
            "FRONT" => Ok(SeriesCover::Front),
            "BACK" => Ok(SeriesCover::Back),
            "SPINE" => Ok(SeriesCover::Spine),
            "ALTERNATIVE" => Ok(SeriesCover::Alternative),
            _ => Err(format!("Unknown SeriesCover: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanInterval {
    Every1H,
    Every2H,
    Every6H,
    Every12H,
    Every24H,
    Disabled,
}

impl Default for ScanInterval {
    fn default() -> Self {
        ScanInterval::Every6H
    }
}

impl TryFrom<&str> for ScanInterval {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_uppercase().as_str() {
            "EVERY_1H" => Ok(ScanInterval::Every1H),
            "EVERY_2H" => Ok(ScanInterval::Every2H),
            "EVERY_6H" => Ok(ScanInterval::Every6H),
            "EVERY_12H" => Ok(ScanInterval::Every12H),
            "EVERY_24H" => Ok(ScanInterval::Every24H),
            "DISABLED" => Ok(ScanInterval::Disabled),
            _ => Err(format!("Unknown ScanInterval: {}", s)),
        }
    }
}

impl Library {
    pub fn new(name: String, root: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_date: now,
            last_modified_date: now,
            name,
            root,
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
            series_cover: SeriesCover::default(),
            unavailable_date: None,
            hash_files: true,
            hash_pages: false,
            analyze_dimensions: true,
            import_comicinfo_series_append_volume: true,
            oneshots_directory: None,
            scan_cbx: true,
            scan_pdf: true,
            scan_epub: true,
            scan_interval: ScanInterval::default(),
            hash_koreader: false,
        }
    }
}