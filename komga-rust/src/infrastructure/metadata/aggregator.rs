use crate::domain::model::book::{Author, BookMetadata};
use crate::infrastructure::metadata::comicrack::ComicRackMetadata;
use crate::infrastructure::metadata::mylar::{MylarMetadata, MylarSeries};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum MetadataSource {
    ComicRack,
    Mylar,
    LocalArtwork,
    UserDefined,
}

pub struct MetadataAggregator;

impl MetadataAggregator {
    pub fn aggregate_book_metadata(
        book_path: &PathBuf,
        comicrack: Option<ComicRackMetadata>,
        user_metadata: Option<BookMetadata>,
    ) -> BookMetadata {
        let now = chrono::Utc::now();
        let mut aggregated = BookMetadata {
            created_date: now,
            last_modified_date: now,
            number: String::new(),
            number_lock: false,
            number_sort: 0.0,
            number_sort_lock: false,
            release_date: None,
            release_date_lock: false,
            summary: String::new(),
            summary_lock: false,
            title: String::new(),
            title_lock: false,
            authors: vec![],
            authors_lock: false,
            tags: vec![],
            tags_lock: false,
            book_id: uuid::Uuid::nil(),
            isbn: String::new(),
            isbn_lock: false,
            links: vec![],
            links_lock: false,
        };

        if let Some(ref comic) = comicrack {
            if !aggregated.title_lock {
                if let Some(title) = &comic.title {
                    aggregated.title = title.clone();
                } else if let Some(series) = &comic.series {
                    let vol = comic.volume.as_deref().unwrap_or("");
                    let num = comic.number.as_deref().unwrap_or("");
                    if !vol.is_empty() || !num.is_empty() {
                        aggregated.title = format!("{} {} {}", series, vol, num).trim().to_string();
                    }
                }
            }

            if !aggregated.summary_lock {
                if let Some(summary) = &comic.summary {
                    aggregated.summary = summary.clone();
                }
            }

            if !aggregated.number_lock {
                if let Some(number) = &comic.number {
                    aggregated.number = number.clone();
                }
            }

            if let Some(count) = comic.page_count {
                aggregated.tags.retain(|t| t != "武装");
                aggregated.tags.push(format!("{} pages", count));
            }
        }

        if let Some(ref user) = user_metadata {
            if user.title_lock && !aggregated.title_lock {
                aggregated.title = user.title.clone();
                aggregated.title_lock = true;
            }
            if user.summary_lock && !aggregated.summary_lock {
                aggregated.summary = user.summary.clone();
                aggregated.summary_lock = true;
            }
            if user.number_lock && !aggregated.number_lock {
                aggregated.number = user.number.clone();
                aggregated.number_lock = true;
            }
            if user.authors_lock && !aggregated.authors_lock {
                aggregated.authors = user.authors.clone();
                aggregated.authors_lock = true;
            }
            if user.tags_lock && !aggregated.tags_lock {
                aggregated.tags = user.tags.clone();
                aggregated.tags_lock = true;
            }
        }

        aggregated
    }

    pub fn aggregate_series_metadata(
        mylar: Option<MylarSeries>,
        user_metadata: Option<&crate::domain::model::series::SeriesMetadata>,
    ) -> crate::domain::model::series::SeriesMetadata {
        let now = chrono::Utc::now();
        let mut aggregated = crate::domain::model::series::SeriesMetadata {
            created_date: now,
            last_modified_date: now,
            status: "OK".to_string(),
            status_lock: false,
            title: String::new(),
            title_lock: false,
            title_sort: String::new(),
            title_sort_lock: false,
            series_id: uuid::Uuid::nil(),
            publisher: String::new(),
            publisher_lock: false,
            reading_direction: None,
            reading_direction_lock: false,
            age_rating: None,
            age_rating_lock: false,
            summary: String::new(),
            summary_lock: false,
            language: "en".to_string(),
            language_lock: false,
            genres: vec![],
            genres_lock: false,
            tags: vec![],
            tags_lock: false,
            total_book_count: None,
            total_book_count_lock: false,
            sharing_labels: vec![],
            sharing_labels_lock: false,
            links: vec![],
            links_lock: false,
            alternate_titles: vec![],
            alternate_titles_lock: false,
        };

        if let Some(mylar_series) = mylar {
            let mylar_meta = &mylar_series.metadata;
            if !aggregated.title_lock {
                aggregated.title = mylar_meta.name.clone();
            }
            if !aggregated.publisher_lock {
                aggregated.publisher = mylar_meta.publisher.clone().unwrap_or_default();
            }
            if !aggregated.summary_lock {
                aggregated.summary = mylar_meta
                    .description_text
                    .clone()
                    .or(mylar_meta.description_formatted.clone())
                    .unwrap_or_default();
            }
            if let Some(year) = mylar_meta.year {
                aggregated.tags.retain(|t| !t.starts_with("Year:"));
                aggregated.tags.push(format!("Year: {}", year));
            }
            if let Some(total) = mylar_meta.total_issues {
                aggregated.total_book_count = Some(total);
            }
        }

        if let Some(user) = user_metadata {
            if user.title_lock && !aggregated.title_lock {
                aggregated.title = user.title.clone();
                aggregated.title_lock = true;
            }
            if user.publisher_lock && !aggregated.publisher_lock {
                aggregated.publisher = user.publisher.clone();
                aggregated.publisher_lock = true;
            }
            if user.summary_lock && !aggregated.summary_lock {
                aggregated.summary = user.summary.clone();
                aggregated.summary_lock = true;
            }
            if user.genres_lock && !aggregated.genres_lock {
                aggregated.genres = user.genres.clone();
                aggregated.genres_lock = true;
            }
        }

        aggregated
    }
}
