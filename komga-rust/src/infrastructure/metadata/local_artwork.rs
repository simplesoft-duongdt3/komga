use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct LocalArtwork;

impl LocalArtwork {
    pub fn new() -> Self {
        Self
    }

    pub fn find_book_cover(&self, book_path: &Path) -> Option<LocalArtworkFile> {
        let book_dir = book_path.parent()?;

        let cover_names = [
            "cover.jpg",
            "cover.jpeg",
            "cover.png",
            "folder.jpg",
            "folder.jpeg",
            "folder.png",
            "thumb.jpg",
            "thumb.jpeg",
            "thumb.png",
            "poster.jpg",
            "poster.jpeg",
            "poster.png",
        ];

        for name in &cover_names {
            let path = book_dir.join(name);
            if path.exists() {
                return Some(LocalArtworkFile {
                    path,
                    file_name: name.to_string(),
                });
            }
        }

        None
    }

    pub fn find_series_cover(&self, series_path: &Path) -> Option<LocalArtworkFile> {
        let cover_names = [
            "cover.jpg",
            "cover.jpeg",
            "cover.png",
            "folder.jpg",
            "folder.jpeg",
            "folder.png",
        ];

        for name in &cover_names {
            let path = series_path.join(name);
            if path.exists() {
                return Some(LocalArtworkFile {
                    path,
                    file_name: name.to_string(),
                });
            }
        }

        if let Ok(entries) = fs::read_dir(series_path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.is_file() {
                    if let Some(name) = file_path.file_name() {
                        let name_str = name.to_string_lossy().to_lowercase();
                        if name_str.starts_with("cover.")
                            || name_str.starts_with("folder.")
                            || name_str.starts_with("poster.")
                        {
                            let file_name = name.to_string_lossy().to_string();
                            return Some(LocalArtworkFile {
                                path: file_path,
                                file_name,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    pub fn find_all_images_in_dir(&self, dir_path: &Path) -> Vec<LocalArtworkFile> {
        let mut images = Vec::new();

        for entry in WalkDir::new(dir_path).max_depth(2).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if ["jpg", "jpeg", "png", "webp", "gif"].contains(&ext_lower.as_str()) {
                        if let Some(name) = path.file_name() {
                            let file_name = name.to_string_lossy().to_string();
                            let path_buf = path.to_path_buf();
                            images.push(LocalArtworkFile {
                                path: path_buf,
                                file_name,
                            });
                        }
                    }
                }
            }
        }

        images.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        images
    }
}

#[derive(Debug, Clone)]
pub struct LocalArtworkFile {
    pub path: std::path::PathBuf,
    pub file_name: String,
}
