use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use walkdir::WalkDir;
use xxhash_rust::xxh3;

use crate::models::{HashCache, HashCacheEntry};

fn file_uri(path: &Path) -> String {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| format!("file://{}", path.display()))
}

fn hash_file_mmap(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    Ok(format!("{:032x}", xxh3::xxh3_128(&mmap)))
}

fn hash_file_stream(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);
    let mut hasher = xxh3::Xxh3::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:032x}", hasher.digest()))
}

pub(crate) fn hash_file(path: &Path, size: u64) -> io::Result<String> {
    const MMAP_THRESHOLD: u64 = 100 * 1024 * 1024; // 100 MB
    if size <= MMAP_THRESHOLD {
        hash_file_mmap(path)
    } else {
        hash_file_stream(path)
    }
}

pub fn load_cache(path: &Path) -> HashMap<String, HashCacheEntry> {
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(data) => match serde_json::from_str::<HashCache>(&data) {
            Ok(cached) => {
                tracing::info!(entries = cached.entries.len(), cache_path = %path.display(), "Loaded hash cache");
                cached.entries
            }
            Err(e) => {
                tracing::warn!(error = %e, cache_path = %path.display(), "Corrupted hash cache, will regenerate");
                HashMap::new()
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, cache_path = %path.display(), "Cannot read hash cache");
            HashMap::new()
        }
    }
}

pub fn build_cache(
    root: &Path,
    cache_path: &Path,
    threads: usize,
) -> io::Result<HashCache> {
    let start = SystemTime::now();

    // 1. Walk filesystem
    let files: Vec<(PathBuf, u64, i64)> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
        })
        .map(|e| {
            let meta = e.metadata().unwrap();
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            (e.path().to_owned(), meta.len(), mtime)
        })
        .collect();

    tracing::info!(count = files.len(), "Found .pdf files");

    // 2. Load existing cache
    let old_cache = load_cache(cache_path);

    // 3. Classify files
    let mut skip_count: usize = 0;
    let mut hash_count: usize = 0;
    let mut rehash_count: usize = 0;

    let files_to_hash: Vec<&(PathBuf, u64, i64)> = files
        .iter()
        .filter(|(path, size, mtime)| {
            let uri = file_uri(path);
            match old_cache.get(&uri) {
                Some(entry) if entry.size == *size && entry.mtime_secs == *mtime => {
                    skip_count += 1;
                    false
                }
                Some(_) => {
                    rehash_count += 1;
                    true
                }
                None => {
                    hash_count += 1;
                    true
                }
            }
        })
        .collect();

    let total_to_hash = hash_count + rehash_count;
    tracing::info!(
        skip = skip_count,
        new = hash_count,
        changed = rehash_count,
        total_to_hash,
        "Hash cache classification"
    );

    // 4. Hash in parallel — collect results without global Mutex contention
    let new_entries: HashMap<String, HashCacheEntry> = if total_to_hash > 0 {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();

        let results: Vec<(String, HashCacheEntry, u64)> = pool.install(|| {
            files_to_hash
                .par_iter()
                .filter_map(|(path, size, mtime)| {
                    match hash_file(path, *size) {
                        Ok(h) => {
                            let uri = file_uri(path);
                            let entry = HashCacheEntry {
                                hash: h,
                                size: *size,
                                mtime_secs: *mtime,
                            };
                            Some((uri, entry, *size))
                        }
                        Err(e) => {
                            tracing::warn!(file = %path.display(), error = %e, "Hash error");
                            None
                        }
                    }
                })
                .collect()
        });

        let hashed_n = results.len();
        let hashed_bytes: u64 = results.iter().map(|(_, _, s)| s).sum();
        tracing::info!(
            hashed = hashed_n,
            bytes = hashed_bytes,
            elapsed_secs = SystemTime::now().duration_since(start).unwrap().as_secs_f64(),
            "Hashing complete"
        );

        results.into_iter().map(|(k, v, _)| (k, v)).collect()
    } else {
        HashMap::new()
    };

    // 5. Merge: old (skip) + new (computed), remove stale
    // Build current URIs from file paths for stale detection
    let current_uri_set: HashSet<String> =
        files.iter().map(|(path, _, _)| file_uri(path)).collect();

    let mut merged: HashMap<String, HashCacheEntry> = HashMap::new();
    let mut stale_count: usize = 0;

    for (uri, entry) in &old_cache {
        if current_uri_set.contains(uri) {
            merged.insert(uri.clone(), entry.clone());
        } else {
            stale_count += 1;
        }
    }
    for (uri, entry) in new_entries {
        merged.insert(uri, entry);
    }

    if stale_count > 0 {
        tracing::info!(stale = stale_count, "Removed stale entries");
    }

    // 6. Build result
    let elapsed = SystemTime::now().duration_since(start).unwrap();
    let total_bytes_val: u64 = files.iter().map(|(_, s, _)| s).sum();

    let cache = HashCache {
        cache_type: "XXH3_128 hash cache".to_string(),
        version: 1,
        root: root.to_string_lossy().to_string(),
        generated_at_unix_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        elapsed_secs: elapsed.as_secs_f64(),
        total_files: merged.len(),
        total_bytes: total_bytes_val,
        entries: merged,
    };

    Ok(cache)
}

pub fn write_cache(cache: &HashCache, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension("json.tmp");
    let tmp_file = File::create(&tmp_path)?;
    serde_json::to_writer(&tmp_file, cache)?;
    tmp_file.sync_all()?;
    drop(tmp_file);
    fs::rename(&tmp_path, path)?;

    tracing::info!(path = %path.display(), entries = cache.total_files, "Hash cache written");
    Ok(())
}
