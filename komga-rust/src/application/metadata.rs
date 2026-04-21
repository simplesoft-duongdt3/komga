use std::path::PathBuf;
use std::fs;

use crate::infrastructure::metadata::mylar::MylarProvider;
use crate::infrastructure::metadata::local_artwork::LocalArtwork;

pub struct MetadataService {
    mylar: MylarProvider,
    artwork: LocalArtwork,
}

impl MetadataService {
    pub fn new() -> Self {
        Self {
            mylar: MylarProvider::new(),
            artwork: LocalArtwork::new(),
        }
    }

    pub fn get_series_metadata(&self, series_path: &PathBuf) -> Option<SeriesMetadataPatch> {
        let mylar_metadata = self.mylar.get_series_metadata(series_path)?;
        
        let has_local_cover = self.artwork.find_series_cover(series_path).is_some();
        
        Some(SeriesMetadataPatch {
            title: Some(mylar_metadata.title.clone()),
            title_sort: Some(mylar_metadata.title_sort.clone()),
            status: Some(mylar_metadata.status.clone()),
            summary: mylar_metadata.summary.clone(),
            publisher: mylar_metadata.publisher.clone(),
            age_rating: mylar_metadata.age_rating,
            total_book_count: mylar_metadata.total_book_count,
            has_local_cover,
        })
    }

    pub fn get_book_metadata_from_comicinfo(&self, book_path: &PathBuf) -> Option<BookMetadataPatch> {
        let book_dir = book_path.parent()?;
        let comicinfo_path = book_dir.join("ComicInfo.xml");
        
        if !comicinfo_path.exists() {
            return None;
        }
        
        let content = fs::read_to_string(&comicinfo_path).ok()?;
        
        let parsed = ComicInfo::parse(&content)?;
        
        Some(BookMetadataPatch {
            title: parsed.title,
            number: parsed.number,
            series: parsed.series,
            volume: parsed.volume,
            summary: parsed.summary,
            writer: parsed.writer,
            publisher: parsed.publisher,
            genre: parsed.genre,
            tags: parsed.tags,
            year: parsed.year,
            month: parsed.month,
            day: parsed.day,
            isbn: parsed.isbn,
            has_local_cover: self.artwork.find_book_cover(book_path).is_some(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SeriesMetadataPatch {
    pub title: Option<String>,
    pub title_sort: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub publisher: Option<String>,
    pub age_rating: Option<i32>,
    pub total_book_count: Option<i32>,
    pub has_local_cover: bool,
}

#[derive(Debug, Clone)]
pub struct BookMetadataPatch {
    pub title: Option<String>,
    pub number: Option<String>,
    pub series: Option<String>,
    pub volume: Option<i32>,
    pub summary: Option<String>,
    pub writer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub tags: Option<String>,
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
    pub isbn: Option<String>,
    pub has_local_cover: bool,
}

struct ComicInfo {
    title: Option<String>,
    number: Option<String>,
    series: Option<String>,
    volume: Option<i32>,
    summary: Option<String>,
    writer: Option<String>,
    publisher: Option<String>,
    genre: Option<String>,
    tags: Option<String>,
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
    isbn: Option<String>,
}

impl ComicInfo {
    fn parse(content: &str) -> Option<Self> {
        let mut info = ComicInfo {
            title: None,
            number: None,
            series: None,
            volume: None,
            summary: None,
            writer: None,
            publisher: None,
            genre: None,
            tags: None,
            year: None,
            month: None,
            day: None,
            isbn: None,
        };
        
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("<Title>") {
                info.title = Self::extract_value(line, "Title");
            } else if line.starts_with("<Number>") {
                info.number = Self::extract_value(line, "Number");
            } else if line.starts_with("<Series>") {
                info.series = Self::extract_value(line, "Series");
            } else if line.starts_with("<Volume>") {
                info.volume = Self::extract_value(line, "Volume").and_then(|s| s.parse().ok());
            } else if line.starts_with("<Summary>") {
                info.summary = Self::extract_value(line, "Summary");
            } else if line.starts_with("<Writer>") {
                info.writer = Self::extract_value(line, "Writer");
            } else if line.starts_with("<Publisher>") {
                info.publisher = Self::extract_value(line, "Publisher");
            } else if line.starts_with("<Genre>") {
                info.genre = Self::extract_value(line, "Genre");
            } else if line.starts_with("<Tags>") {
                info.tags = Self::extract_value(line, "Tags");
            } else if line.starts_with("<Year>") {
                info.year = Self::extract_value(line, "Year").and_then(|s| s.parse().ok());
            } else if line.starts_with("<Month>") {
                info.month = Self::extract_value(line, "Month").and_then(|s| s.parse().ok());
            } else if line.starts_with("<Day>") {
                info.day = Self::extract_value(line, "Day").and_then(|s| s.parse().ok());
            } else if line.starts_with("<ISBN>") {
                info.isbn = Self::extract_value(line, "ISBN");
            }
        }
        
        Some(info)
    }
    
    fn extract_value(line: &str, tag: &str) -> Option<String> {
        let start = format!("<{}>", tag);
        let end = format!("</{}>", tag);
        line.replace(&start, "").replace(&end, "").into()
    }
}