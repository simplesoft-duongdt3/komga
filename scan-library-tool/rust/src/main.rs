mod api;
mod applier;
mod config;
mod db;
mod diff;
mod export;
mod hasher;
mod models;
mod perf;
mod server;
mod walker;

use std::path::Path;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "komga-smart-scanner", version, about = "External diff-based scanner for Komga")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start the web server (default)")]
    Serve {
        #[arg(short, long, env = "PORT")]
        port: Option<u16>,
    },
    #[command(about = "CLI mode — interactive library scanner")]
    Scan,
    #[command(about = "Pre-compute XXH3_128 hash cache")]
    HashCache {
        #[arg(short, long)]
        root: std::path::PathBuf,
        #[arg(short, long)]
        cache: Option<std::path::PathBuf>,
        #[arg(short = 'j', long, default_value = "4")]
        threads: usize,
    },
    #[command(about = "Run compatibility tests against fixture JSON")]
    CompatTest {
        #[arg(short, long)]
        fixtures: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let cfg = config::Config::from_env();

    match cli.command.unwrap_or(Commands::Serve { port: None }) {
        Commands::Serve { port } => {
            let port = port.unwrap_or(cfg.port);
            run_server(cfg, port).await?;
        }
        Commands::Scan => {
            run_cli(cfg).await?;
        }
        Commands::HashCache { root, cache, threads } => {
            run_hash_cache(&root, cache.as_deref(), threads, &cfg).await?;
        }
        Commands::CompatTest { fixtures } => {
            run_compat_test(&fixtures)?;
        }
    }

    Ok(())
}

async fn run_server(cfg: config::Config, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(port, "Starting Komga Smart Scanner server");

    let pool = db::connect(&cfg.pg_conn_string()).await?;
    let api_client = api::KomgaClient::new(&cfg);

    let state = server::AppState {
        config: cfg,
        api_client,
        pg_pool: pool,
        generating_libs: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        library_root_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    };

    let app = server::build_router(state);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn run_cli(cfg: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("Komga Smart Scanner v{}", cfg.version);

    let api_client = api::KomgaClient::new(&cfg);
    let pool = db::connect(&cfg.pg_conn_string()).await?;

    println!("Fetching libraries...");
    let libs = api_client.list_libraries().await?;
    if libs.is_empty() {
        println!("No libraries found.");
        return Ok(());
    }

    println!("\nLibraries:");
    for (i, lib) in libs.iter().enumerate() {
        println!("  [{}] {}  (id={})", i, lib.name, lib.id);
    }

    let choice = loop {
        let mut input = String::new();
        print!("\nSelect library: ");
        std::io::Write::flush(&mut std::io::stdout())?;
        std::io::stdin().read_line(&mut input)?;
        if let Ok(n) = input.trim().parse::<usize>() {
            if n < libs.len() {
                break n;
            }
        }
        println!("Invalid selection.");
    };

    let library = &libs[choice];
    let lid = &library.id;
    let lib_name = &library.name;
    let root = library.root.as_deref().unwrap_or("").strip_prefix("file://").unwrap_or("").to_string();

    if root.is_empty() {
        println!("Library root is empty.");
        return Ok(());
    }

    println!("\n── Reading DB state for '{}' ──", lib_name);
    let db_series = db::read_series(&pool, lid).await?;
    let db_books = db::read_books(&pool, lid).await?;
    println!("  DB: {} series, {} books", db_series.len(), db_books.len());

    println!("\n── Walking filesystem ──");
    let fs_data = walker::walk_library(
        &root, &[], None, false, &None, 0, None::<fn(usize, usize, &str)>,
    );
    let fs_series_count = fs_data.series.len();
    let fs_books_count: usize = fs_data.series.values().map(|s| s.books.len()).sum::<usize>() + fs_data.oneshots.len();
    println!("  FS: {} series, {} PDF files", fs_series_count, fs_books_count);

    let d = diff::compute(&db_series, &db_books, &fs_data);
    println!("\n── Diff ──");
    println!("  New series:      {}", d.new_series.len());
    println!("  Deleted series:  {}", d.deleted_series.len());
    println!("  New books:       {}", d.new_books.len());
    println!("  Deleted books:   {}", d.deleted_books.len());
    println!("  Changed books:   {}", d.changed_books.len());

    let total_actions = d.new_series.len() + d.deleted_series.len() + d.new_books.len()
        + d.deleted_books.len() + d.changed_books.len();

    if total_actions == 0 {
        println!("\n  No changes detected. Library is up to date.");
        return Ok(());
    }

    if cfg.dry_run {
        println!("\n── [DRY RUN] Preview ({} actions) ──", total_actions);
    } else {
        print!("\n── Apply {} changes? [y/N] ── ", total_actions);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("\n── Applying ({}) ──", if cfg.dry_run { "DRY RUN" } else { "live" });
    applier::apply(&d, lid, &api_client, cfg.dry_run).await;
    println!("\nDone.");

    Ok(())
}

async fn run_hash_cache(
    root: &Path, cache: Option<&Path>, threads: usize, _cfg: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = cache.map(|p| p.to_path_buf()).unwrap_or_else(|| root.join("hashes.json"));

    println!("Root: {}", root.display());
    println!("Cache: {}", cache_path.display());
    println!("Threads: {}", threads);

    let cache = hasher::build_cache(root, &cache_path, threads)?;
    hasher::write_cache(&cache, &cache_path)?;

    println!("Total: {} files, {:.1} MB", cache.total_files, cache.total_bytes as f64 / 1_048_576.0);
    Ok(())
}

fn run_compat_test(fixtures_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(fixtures_path)?;
    let fixtures: serde_json::Value = serde_json::from_str(&data)?;

    let mut output = serde_json::json!({"implementation": "rust"});

    // normalize_url tests
    if let Some(cases) = fixtures.get("normalize_url").and_then(|v| v.as_array()) {
        let results: Vec<serde_json::Value> = cases.iter().enumerate().map(|(i, case)| {
            let input = case["input"].as_str().unwrap();
            let expected = case["expected"].as_str().unwrap();
            let actual = diff::normalize_file_url(input);
            let ok = actual == expected;
            serde_json::json!({
                "index": i, "input": input, "expected": expected, "actual": actual, "pass": ok
            })
        }).collect();
        output["normalize_url"] = serde_json::json!(results);
    }

    // mtime_equals tests
    if let Some(cases) = fixtures.get("mtime_equals").and_then(|v| v.as_array()) {
        let results: Vec<serde_json::Value> = cases.iter().enumerate().map(|(i, case)| {
            let fs = case["fs"].as_str().unwrap();
            let db = case["db"].as_str().unwrap();
            let expected = case["expected"].clone();
            let actual = diff::mtime_equals(fs, db);
            let actual_json = match actual {
                Some(b) => serde_json::json!(b),
                None => serde_json::Value::Null,
            };
            let ok = actual_json == expected;
            serde_json::json!({
                "index": i, "fs": fs, "db": db, "expected": expected, "actual": actual_json, "pass": ok
            })
        }).collect();
        output["mtime_equals"] = serde_json::json!(results);
    }

    // diff tests
    if let Some(cases) = fixtures.get("diff").and_then(|v| v.as_array()) {
        let results: Vec<serde_json::Value> = cases.iter().enumerate().map(|(i, case)| {
            let name = case["name"].as_str().unwrap_or("");
            let db_series: Vec<models::DbSeries> = serde_json::from_value(case["db_series"].clone()).unwrap_or_default();
            let db_books: Vec<models::DbBook> = serde_json::from_value(case["db_books"].clone()).unwrap_or_default();
            let fs_series: std::collections::HashMap<String, models::FsSeries> = serde_json::from_value(case["fs_series"].clone()).unwrap_or_default();
            let fs_oneshots: Vec<models::FsBook> = serde_json::from_value(case["fs_oneshots"].clone()).unwrap_or_default();
            let fs_data = models::FsData { series: fs_series, oneshots: fs_oneshots };

            let d = diff::compute(&db_series, &db_books, &fs_data);

            let actual = serde_json::json!({
                "new_series": d.new_series.len(),
                "deleted_series": d.deleted_series.len(),
                "new_books": d.new_books.len(),
                "deleted_books": d.deleted_books.len(),
                "changed_books": d.changed_books.len(),
                "pending_hash": d.pending_hash.len(),
            });

            let expected = case["expected"].clone();
            let ok = actual == expected;
            serde_json::json!({
                "index": i, "name": name, "expected": expected, "actual": actual, "pass": ok
            })
        }).collect();
        output["diff"] = serde_json::json!(results);
    }

    // xxh3_128 tests
    if let Some(cases) = fixtures.get("xxh3_128").and_then(|v| v.as_array()) {
        let results: Vec<serde_json::Value> = cases.iter().enumerate().map(|(i, case)| {
            let data: Vec<u8> = if let Some(input) = case.get("input").and_then(|v| v.as_str()) {
                input.as_bytes().to_vec()
            } else if let Some(hex) = case.get("input-hex").and_then(|v| v.as_str()) {
                hex::decode(hex).unwrap_or_default()
            } else {
                vec![]
            };
            let expected = case["expected"].as_str().unwrap_or("");
            let actual = format!("{:032x}", xxhash_rust::xxh3::xxh3_128(&data));
            let ok = actual == expected;
            let label = case.get("desc").or_else(|| case.get("input")).and_then(|v| v.as_str()).unwrap_or("");
            serde_json::json!({
                "index": i, "input": label, "expected": expected, "actual": actual, "pass": ok
            })
        }).collect();
        output["xxh3_128"] = serde_json::json!(results);
    }

    // hash_cache tests
    if let Some(cases) = fixtures.get("hash_cache").and_then(|v| v.as_array()) {
        let results: Vec<serde_json::Value> = cases.iter().enumerate().map(|(i, case)| {
            let name = case["name"].as_str().unwrap_or("");
            let cache_json = serde_json::to_string(&case["cache"]).unwrap_or_default();
            let parsed: Result<crate::models::HashCache, _> = serde_json::from_str(&cache_json);
            let (actual, ok) = match parsed {
                Ok(hc) => {
                    let mut entries: Vec<_> = hc.entries.iter().collect();
                    entries.sort_by(|a, b| a.0.cmp(b.0));
                    let first_hash = entries.first().map(|(_, e)| e.hash.clone()).unwrap_or_default();
                    let first_size = entries.first().map(|(_, e)| e.size).unwrap_or(0);
                    let last_hash = entries.last().map(|(_, e)| e.hash.clone()).unwrap_or_default();
                    let actual = serde_json::json!({
                        "total_files": hc.total_files,
                        "total_bytes": hc.total_bytes,
                        "root": hc.root,
                        "entry_count": hc.entries.len(),
                        "first_entry_hash": first_hash,
                        "first_entry_size": first_size,
                        "last_entry_hash": last_hash,
                    });
                    let checks = &case["checks"];
                    let ok = checks["total_files"] == actual["total_files"]
                        && checks["total_bytes"] == actual["total_bytes"]
                        && checks["root"] == actual["root"]
                        && checks["entry_count"] == actual["entry_count"]
                        && checks["first_entry_hash"] == actual["first_entry_hash"]
                        && checks["first_entry_size"] == actual["first_entry_size"]
                        && checks["last_entry_hash"] == actual["last_entry_hash"];
                    (actual, ok)
                }
                Err(e) => {
                    (serde_json::json!({"error": e.to_string()}), false)
                }
            };
            serde_json::json!({"index": i, "name": name, "checks": case["checks"], "actual": actual, "pass": ok })
        }).collect();
        output["hash_cache"] = serde_json::json!(results);
    }
    let all_results: Vec<&serde_json::Value> = output.as_object().unwrap()
        .values()
        .filter(|v| v.is_array())
        .flat_map(|v| v.as_array().unwrap().iter())
        .collect();
    let total = all_results.len();
    let passed = all_results.iter().filter(|r| r["pass"].as_bool().unwrap_or(false)).count();
    output["summary"] = serde_json::json!({"total": total, "passed": passed, "failed": total - passed});

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
