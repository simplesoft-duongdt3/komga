use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::hasher;
use crate::models::{FsBook, FsData, FsSeries, HashCacheEntry};

fn mtime_to_iso(mtime: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = mtime.into();
    dt.to_rfc3339()
}

fn file_uri(path: &Path) -> String {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| format!("file://{}", path.display()))
}

fn compute_hash(
    path: &Path,
    hash_cache: &Option<HashMap<String, HashCacheEntry>>,
    hash_files: bool,
) -> Option<String> {
    if hash_files {
        if let Some(cache) = hash_cache {
            let uri = file_uri(path);
            if let Some(entry) = cache.get(&uri) {
                return Some(entry.hash.clone());
            }
        }
        // Fall back to computing the hash directly
        match hasher::hash_file(path, path.metadata().ok()?.len()) {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::warn!(file = %path.display(), error = %e, "Hash computation failed");
                None
            }
        }
    } else {
        None
    }
}

fn scan_dir(
    dir: &Path,
    hash_cache: &Option<HashMap<String, HashCacheEntry>>,
    hash_files: bool,
) -> Option<FsSeries> {
    let meta = dir.metadata().ok()?;
    let mtime = meta.modified().ok()?;
    let file_last_modified = mtime_to_iso(mtime);

    let mut books = Vec::new();

    let entries: Vec<_> = WalkDir::new(dir)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
        })
        .collect();

    for entry in &entries {
        let fpath = entry.path();
        let Ok(fmeta) = fpath.metadata() else { continue };
        let Some(fmtime) = fmeta.modified().ok() else { continue };
        let url = file_uri(fpath);
        let name = fpath
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_hash = compute_hash(fpath, hash_cache, hash_files);

        books.push(FsBook {
            url,
            name,
            file_size: fmeta.len(),
            file_last_modified: mtime_to_iso(fmtime),
            file_hash,
        });
    }

    if books.is_empty() {
        return None;
    }

    // Sort books by name case-insensitively
    books.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Some(FsSeries {
        url: file_uri(dir),
        name: dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        file_last_modified,
        books,
    })
}

pub fn walk_library(
    root: &str,
    exclusions: &[String],
    oneshots_dir: Option<&str>,
    hash_files: bool,
    hash_cache: &Option<HashMap<String, HashCacheEntry>>,
    max_workers: usize,
    on_progress: Option<impl Fn(usize, usize, &str) + Send + Sync>,
) -> FsData {
    let root_path = PathBuf::from(root);
    let ex_set: std::collections::HashSet<&str> =
        exclusions.iter().map(|s| s.as_str()).collect();

    // Collect top-level directories
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(read_dir) = root_path.read_dir() {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !ex_set.contains(name) {
                        dirs.push(path);
                    }
                }
            }
        }
    }

    // Parallel walk directories
    let series_map = Mutex::new(HashMap::new());
    let completed = AtomicUsize::new(0);
    let total = dirs.len();

    if !dirs.is_empty() {
        let effective_threads = if max_workers == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get() * 2)
                .unwrap_or(8)
        } else {
            max_workers
        };

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(effective_threads)
            .build()
            .unwrap();

        pool.install(|| {
            dirs.par_iter().for_each(|dir| {
                let result = scan_dir(dir, hash_cache, hash_files);
                let idx = completed.fetch_add(1, Ordering::SeqCst) + 1;
                if let Some(ref cb) = on_progress {
                    let dir_name = dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    cb(idx, total, &dir_name);
                }
                if let Some(series) = result {
                    series_map.lock().unwrap().insert(series.url.clone(), series);
                }
            });
        });
    }

    // Oneshots at root level
    let mut oneshots: Vec<FsBook> = Vec::new();
    if let Ok(read_dir) = root_path.read_dir() {
        let mut entries: Vec<_> = read_dir
            .flatten()
            .filter(|e| {
                e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && e.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
            })
            .collect();
        entries.sort_by_key(|e| e.file_name().to_ascii_lowercase());

        for entry in &entries {
            let fpath = entry.path();
            let Ok(fmeta) = fpath.metadata() else { continue };
            let Some(fmtime) = fmeta.modified().ok() else { continue };
            let url = file_uri(&fpath);
            let name = fpath
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let file_hash = compute_hash(&fpath, hash_cache, hash_files);

            oneshots.push(FsBook {
                url,
                name,
                file_size: fmeta.len(),
                file_last_modified: mtime_to_iso(fmtime),
                file_hash,
            });
        }
    }

    // Oneshots in dedicated oneshots directory
    if let Some(od) = oneshots_dir {
        let oneshots_path = root_path.join(od);
        if oneshots_path.is_dir() {
            if let Ok(read_dir) = oneshots_path.read_dir() {
                let mut entries: Vec<_> = read_dir
                    .flatten()
                    .filter(|e| {
                        e.file_type().map(|t| t.is_file()).unwrap_or(false)
                            && e.path().extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
                    })
                    .collect();
                entries.sort_by_key(|e| e.file_name().to_ascii_lowercase());

                for entry in &entries {
                    let fpath = entry.path();
                    let Ok(fmeta) = fpath.metadata() else { continue };
                    let Some(fmtime) = fmeta.modified().ok() else { continue };
                    let url = file_uri(&fpath);
                    let name = fpath
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let file_hash = compute_hash(&fpath, hash_cache, hash_files);

                    oneshots.push(FsBook {
                        url,
                        name,
                        file_size: fmeta.len(),
                        file_last_modified: mtime_to_iso(fmtime),
                        file_hash,
                    });
                }
            }
        }
    }

    FsData {
        series: series_map.into_inner().unwrap(),
        oneshots,
    }
}
