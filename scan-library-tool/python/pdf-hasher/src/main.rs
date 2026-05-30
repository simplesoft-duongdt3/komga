use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use xxhash_rust::xxh3;

/// One-time/incremental PDF hasher — computes XXH3_128 for every .pdf
/// under ROOT, caches results, and on subsequent runs only hashes files
/// that are new or whose size/mtime has changed.
#[derive(Parser)]
#[command(name = "pdf-hasher", version, about = "Incremental XXH3_128 hasher for PDF libraries")]
struct Args {
    /// Root directory to walk recursively
    #[arg(short, long)]
    root: PathBuf,

    /// Cache file path (default: {root}/hashes.json)
    #[arg(short, long)]
    cache: Option<PathBuf>,

    /// Hash worker threads (I/O bound — 4–8 on HDD, 8–16 on SSD)
    #[arg(short = 'j', long, default_value = "4")]
    threads: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    hash: String,
    size: u64,
    mtime_secs: i64,
}

#[derive(Serialize, Deserialize)]
struct HashCache {
    #[serde(rename = "type")]
    cache_type: String,
    version: u8,
    root: String,
    generated_at_unix_secs: u64,
    elapsed_secs: f64,
    total_files: usize,
    total_bytes: u64,
    entries: HashMap<String, CacheEntry>,
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn hash_file_mmap(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
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

fn hash_file(path: &Path, size: u64, mmap_threshold: u64) -> io::Result<String> {
    if size <= mmap_threshold {
        hash_file_mmap(path)
    } else {
        hash_file_stream(path)
    }
}

fn load_cache(path: &Path) -> HashMap<String, CacheEntry> {
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(data) => match serde_json::from_str::<HashCache>(&data) {
            Ok(cached) => {
                println!("Loaded {} cached entries from {}", cached.entries.len(), path.display());
                cached.entries
            }
            Err(e) => {
                eprintln!("Corrupted cache at {} ({}), will regenerate", path.display(), e);
                HashMap::new()
            }
        },
        Err(e) => {
            eprintln!("Cannot read cache at {}: {}", path.display(), e);
            HashMap::new()
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let start = SystemTime::now();
    let cache_path = args.cache.unwrap_or_else(|| args.root.join("hashes.json"));
    let mmap_threshold: u64 = 100 * 1024 * 1024; // 100 MB

    // ── 1. Walk filesystem ──────────────────────────────────
    let walk_bar = ProgressBar::new_spinner();
    walk_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} Walking directory tree... {pos}")
            .unwrap(),
    );

    let files: Vec<(PathBuf, u64, i64)> = WalkDir::new(&args.root)
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
            walk_bar.inc(1);
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

    walk_bar.finish_and_clear();
    println!("Found {} .pdf files", files.len());

    // ── 2. Load existing cache ──────────────────────────────
    let old_cache = load_cache(&cache_path);

    // ── 3. Classify files ───────────────────────────────────
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
    println!(
        "Status: {} skip (cached, unchanged), {} new, {} changed — {} to hash",
        skip_count, hash_count, rehash_count, total_to_hash,
    );

    // ── 4. Hash in parallel ─────────────────────────────────
    let results = Mutex::new(HashMap::new());
    let count = AtomicUsize::new(0);
    let total_bytes = AtomicU64::new(0);

    if total_to_hash > 0 {
        let hash_bar = ProgressBar::new(total_to_hash as u64);
        hash_bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len}  {per_sec:.cyan}")
                .unwrap()
                .progress_chars("##-"),
        );

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build()
            .unwrap();

        pool.install(|| {
            files_to_hash.par_iter().for_each(|(path, size, mtime)| {
                let hash = hash_file(path, *size, mmap_threshold);
                match hash {
                    Ok(h) => {
                        let uri = file_uri(path);
                        let entry = CacheEntry {
                            hash: h,
                            size: *size,
                            mtime_secs: *mtime,
                        };
                        results.lock().unwrap().insert(uri, entry);
                        total_bytes.fetch_add(*size, Ordering::Relaxed);
                    }
                    Err(e) => eprintln!("[ERR] {}: {}", path.display(), e),
                }
                count.fetch_add(1, Ordering::SeqCst);
                hash_bar.inc(1);
            });
        });

        hash_bar.finish_and_clear();
        let hashed_n = count.load(Ordering::Relaxed);
        let hashed_bytes = total_bytes.load(Ordering::Relaxed);
        println!(
            "Hashed {} files ({} bytes new) in {:.1}s",
            hashed_n,
            hashed_bytes,
            SystemTime::now().duration_since(start).unwrap().as_secs_f64(),
        );
    }

    // ── 5. Merge: old (skip) + new (computed), remove stale ─
    let new_entries = results.into_inner().unwrap();
    let current_uris: HashSet<String> =
        files.iter().map(|(path, _, _)| file_uri(path)).collect();

    let mut merged: HashMap<String, CacheEntry> = HashMap::new();
    let mut stale_count: usize = 0;

    for (uri, entry) in &old_cache {
        if current_uris.contains(uri) {
            merged.insert(uri.clone(), entry.clone());
        } else {
            stale_count += 1;
        }
    }
    // Overwrite with freshly hashed entries
    for (uri, entry) in new_entries {
        merged.insert(uri, entry);
    }

    if stale_count > 0 {
        println!("Stale entries removed: {}", stale_count);
    }

    // ── 6. Atomic write (if anything changed) ──────────────
    let needs_write = total_to_hash > 0 || stale_count > 0;
    if needs_write {
        let elapsed = SystemTime::now().duration_since(start).unwrap();
        let total_bytes_val: u64 = files.iter().map(|(_, s, _)| s).sum();
        let cache = HashCache {
            cache_type: "XXH3_128 hash cache".to_string(),
            version: 1,
            root: args.root.to_string_lossy().to_string(),
            generated_at_unix_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            elapsed_secs: elapsed.as_secs_f64(),
            total_files: merged.len(),
            total_bytes: total_bytes_val,
            entries: merged,
        };

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let tmp_path = cache_path.with_extension("json.tmp");
        let tmp_file = File::create(&tmp_path)?;
        serde_json::to_writer(&tmp_file, &cache)?;
        tmp_file.sync_all()?;
        drop(tmp_file);
        fs::rename(&tmp_path, &cache_path)?;

        println!("Cache written to {} ({} entries)", cache_path.display(), cache.total_files);
    } else {
        println!("All {} files already cached and unchanged — nothing to do.", files.len());
    }

    let elapsed = SystemTime::now().duration_since(start).unwrap();
    println!(
        "Total: {} files, {:.0} MB, {:.1}s ({:.0} MB/s)",
        files.len(),
        files.iter().map(|(_, s, _)| s).sum::<u64>() as f64 / 1_048_576.0,
        elapsed.as_secs_f64(),
        files.iter().map(|(_, s, _)| s).sum::<u64>() as f64 / elapsed.as_secs_f64() / 1_048_576.0,
    );

    Ok(())
}
