use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComicRackMetadata {
    pub title: Option<String>,
    pub series: Option<String>,
    pub volume: Option<String>,
    pub number: Option<String>,
    pub count: Option<i32>,
    pub summary: Option<String>,
    pub writer: Option<String>,
    pub penciller: Option<String>,
    pub colorist: Option<String>,
    pub inker: Option<String>,
    pub letterer: Option<String>,
    pub cover_artist: Option<String>,
    pub editor: Option<String>,
    pub publisher: Option<String>,
    pub imprint: Option<String>,
    pub genre: Option<String>,
    pub web: Option<String>,
    pub page_count: Option<i32>,
    pub language: Option<String>,
    pubFormat: Option<String>,
    pub age_rating: Option<String>,
    pub black_and_white: Option<bool>,
    pub pages: Vec<ComicRackPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComicRackPage {
    pub image: Option<i32>,
    #[serde(rename = "type")]
    pub page_type: Option<String>,
    pub size: Option<i64>,
    pub double_page: Option<bool>,
}

pub fn parse_comicinfo_xml(
    content: &str,
) -> Result<ComicRackMetadata, Box<dyn std::error::Error + Send + Sync>> {
    let mut metadata = ComicRackMetadata::default();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_pages = false;
    let mut current_page = ComicRackPage::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if current_element == "Pages" {
                    in_pages = true;
                }
            }
            Ok(Event::Empty(e)) => {
                let element_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if in_pages && element_name == "Page" {
                    let mut page = ComicRackPage::default();
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(&attr.value).to_string();

                        match key.as_str() {
                            "Image" => page.image = value.parse().ok(),
                            "Type" => page.page_type = Some(value),
                            "Size" => page.size = value.parse().ok(),
                            "DoublePage" => page.double_page = value.parse().ok(),
                            _ => {}
                        }
                    }
                    metadata.pages.push(page);
                } else {
                    let value = e
                        .attributes()
                        .flatten()
                        .find_map(|a| {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            if key == "Value" {
                                Some(String::from_utf8_lossy(&a.value).to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    match element_name.as_str() {
                        "Title" => metadata.title = Some(value),
                        "Series" => metadata.series = Some(value),
                        "Volume" => metadata.volume = Some(value),
                        "Number" => metadata.number = Some(value),
                        "Count" => metadata.count = value.parse().ok(),
                        "Summary" => metadata.summary = Some(value),
                        "Writer" => metadata.writer = Some(value),
                        "Penciller" => metadata.penciller = Some(value),
                        "Colorist" => metadata.colorist = Some(value),
                        "Inker" => metadata.inker = Some(value),
                        "Letterer" => metadata.letterer = Some(value),
                        "CoverArtist" => metadata.cover_artist = Some(value),
                        "Editor" => metadata.editor = Some(value),
                        "Publisher" => metadata.publisher = Some(value),
                        "Imprint" => metadata.imprint = Some(value),
                        "Genre" => metadata.genre = Some(value),
                        "Web" => metadata.web = Some(value),
                        "PageCount" => metadata.page_count = value.parse().ok(),
                        "Language" => metadata.language = Some(value),
                        "Format" => metadata.pubFormat = Some(value),
                        "AgeRating" => metadata.age_rating = Some(value),
                        "BlackAndWhite" => metadata.black_and_white = value.parse().ok(),
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if !text.is_empty() && !in_pages {
                    match current_element.as_str() {
                        "Title" => metadata.title = Some(text),
                        "Series" => metadata.series = Some(text),
                        "Volume" => metadata.volume = Some(text),
                        "Number" => metadata.number = Some(text),
                        "Count" => metadata.count = text.parse().ok(),
                        "Summary" => metadata.summary = Some(text),
                        "Writer" => metadata.writer = Some(text),
                        "Penciller" => metadata.penciller = Some(text),
                        "Colorist" => metadata.colorist = Some(text),
                        "Inker" => metadata.inker = Some(text),
                        "Letterer" => metadata.letterer = Some(text),
                        "CoverArtist" => metadata.cover_artist = Some(text),
                        "Editor" => metadata.editor = Some(text),
                        "Publisher" => metadata.publisher = Some(text),
                        "Imprint" => metadata.imprint = Some(text),
                        "Genre" => metadata.genre = Some(text),
                        "Web" => metadata.web = Some(text),
                        "PageCount" => metadata.page_count = text.parse().ok(),
                        "Language" => metadata.language = Some(text),
                        "Format" => metadata.pubFormat = Some(text),
                        "AgeRating" => metadata.age_rating = Some(text),
                        "BlackAndWhite" => metadata.black_and_white = text.parse().ok(),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let element_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if element_name == "Pages" {
                    in_pages = false;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(metadata)
}

pub fn find_comicinfo_file(book_path: &Path) -> Option<std::path::PathBuf> {
    let parent = book_path.parent()?;

    let possible_names = ["ComicInfo.xml", "ComicInfo.xml"];

    for name in &possible_names {
        let path = parent.join(name);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

pub fn load_comicinfo_for_book(book_path: &Path) -> Option<ComicRackMetadata> {
    let comicinfo_path = find_comicinfo_file(book_path)?;

    let content = std::fs::read_to_string(&comicinfo_path).ok()?;
    parse_comicinfo_xml(&content).ok()
}
