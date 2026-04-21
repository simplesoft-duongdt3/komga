use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct MylarSeries {
    #[serde(rename = "metadata")]
    pub metadata: MylarMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MylarMetadata {
    pub name: String,
    pub year: Option<i32>,
    pub volume: Option<i32>,
    pub publisher: Option<String>,
    pub description_text: Option<String>,
    pub description_formatted: Option<String>,
    pub status: Option<String>,
    pub total_issues: Option<i32>,
    pub age_rating: Option<MylarAgeRating>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MylarAgeRating {
    #[serde(rename = "ageRating")]
    pub age_rating: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct MylarMetadataResult {
    pub title: String,
    pub title_sort: String,
    pub status: String,
    pub summary: Option<String>,
    pub publisher: Option<String>,
    pub age_rating: Option<i32>,
    pub total_book_count: Option<i32>,
}

pub struct MylarProvider;

impl MylarProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn get_series_metadata(&self, series_path: &Path) -> Option<MylarMetadataResult> {
        let series_json_path = series_path.join("series.json");
        
        if !series_json_path.exists() {
            return None;
        }
        
        let content = fs::read_to_string(&series_json_path).ok()?;
        let mylar: MylarSeries = serde_json::from_str(&content).ok()?;
        let meta = mylar.metadata;
        
        let title = if meta.volume == Some(1) || meta.volume.is_none() {
            meta.name.clone()
        } else {
            let year_str = meta.year.map(|y| format!(" ({})", y)).unwrap_or_default();
            format!("{}{}", meta.name, year_str)
        };
        
        let status = match meta.status.as_deref() {
            Some("Ended") => "ENDED".to_string(),
            Some("Continuing") | _ => "ONGOING".to_string(),
        };
        
        Some(MylarMetadataResult {
            title: title.clone(),
            title_sort: title,
            status,
            summary: meta.description_formatted.or(meta.description_text),
            publisher: meta.publisher,
            age_rating: meta.age_rating.and_then(|a| a.age_rating),
            total_book_count: meta.total_issues,
        })
    }
}