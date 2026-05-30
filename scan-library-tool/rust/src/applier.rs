use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use chrono::Utc;

use crate::api::KomgaClient;
use crate::config::Config;
use crate::diff;
use crate::models::*;

fn lines_for(
    cat: &str,
    diff: &Diff,
    library_id: &str,
    analyze: bool,
    refresh: bool,
    db_series_by_url: Option<&HashMap<String, DbSeries>>,
    fs_data: Option<&FsData>,
    config: &Config,
) -> Option<Vec<String>> {
    let lines = match cat {
        "new_series" => bld_new_series(diff, library_id, analyze, refresh, config),
        "deleted_series" => bld_deleted_series(diff, library_id, config),
        "new_books" => bld_new_books(diff, library_id, db_series_by_url, fs_data, config),
        "deleted_books" => bld_deleted_books(diff, library_id, config),
        "changed_books" => bld_changed_books(diff, analyze, refresh, config),
        "pending_hash" => bld_pending_hash(diff, config),
        "to_be_analyzed" => bld_to_be_analyzed(diff, config),
        "no_metadata" => bld_no_metadata(diff, config),
        _ => return None,
    };
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

pub fn generate_curl_scripts(
    diff: &Diff,
    library_id: &str,
    categories: &[String],
    analyze: bool,
    refresh: bool,
    output_dir: &Path,
    request_id: &str,
    db_series_by_url: Option<&HashMap<String, DbSeries>>,
    fs_data: Option<&FsData>,
) -> HashMap<String, String> {
    let config = Config::from_env();
    fs::create_dir_all(output_dir).expect("Failed to create output directory");
    let ts = Utc::now().to_rfc3339();

    let mut scripts: HashMap<String, String> = HashMap::new();

    for cat in categories {
        if let Some(lines) = lines_for(cat, diff, library_id, analyze, refresh, db_series_by_url, fs_data, &config) {
            let script = format!(
                "#!/bin/bash\n\
                 # Komga Smart Scanner — script for: {cat}\n\
                 # Request: {request_id} Generated: {ts}\n\
                 # analyze={analyze} refresh={refresh}\n\
                 \n\
                 {}\n",
                lines.join("\n")
            );

            let fname = format!("{}_{}.sh", request_id, cat);
            let fpath = output_dir.join(&fname);
            fs::write(&fpath, &script).expect("Failed to write script");
            fs::set_permissions(&fpath, std::fs::Permissions::from_mode(0o755))
                .expect("Failed to set permissions");
            scripts.insert(cat.clone(), fpath.to_string_lossy().to_string());
        }
    }

    // Combined all.sh
    if categories.len() > 1 {
        let mut combined = Vec::new();
        for cat in categories {
            if let Some(lines) = lines_for(cat, diff, library_id, analyze, refresh, db_series_by_url, fs_data, &config) {
                combined.push(format!("# ── {} ──", cat));
                combined.extend(lines);
                combined.push(String::new());
            }
        }

        if !combined.is_empty() {
            let cats_joined = categories.join(", ");
            let script = format!(
                "#!/bin/bash\n\
                 # Komga Smart Scanner — combined script\n\
                 # Request: {request_id} Generated: {ts}\n\
                 # Categories: {cats_joined}\n\
                 # analyze={analyze} refresh={refresh}\n\
                 \n\
                 {}\n",
                combined.join("\n")
            );
            let fname = format!("{}_all.sh", request_id);
            let fpath = output_dir.join(&fname);
            fs::write(&fpath, &script).expect("Failed to write combined script");
            fs::set_permissions(&fpath, std::fs::Permissions::from_mode(0o755))
                .expect("Failed to set permissions");
            scripts.insert("all".to_string(), fpath.to_string_lossy().to_string());
        }
    }

    scripts
}

fn curl_cmd(method: &str, path: &str, body: Option<&serde_json::Value>, config: &Config) -> String {
    let url = format!("{}/{}", config.komga_url.trim_end_matches('/'), path.trim_start_matches('/'));
    let mut cmd = format!("curl -s -X {} '{}'", method, url);
    let auth = format!("{}:{}", config.komga_user, config.komga_password);
    if auth != ":" {
        cmd.push_str(&format!(" \\\n  -u '{}'", auth));
    }
    cmd.push_str(" \\\n  -H 'Content-Type: application/json'");
    if let Some(b) = body {
        let body_str = serde_json::to_string(b).unwrap_or_default();
        let body_safe = body_str.replace('\'', "'\\''");
        cmd.push_str(&format!(" \\\n  -d '{}'", body_safe));
    }
    cmd
}

fn book_body(b: &FsBook) -> serde_json::Value {
    let mut body = serde_json::json!({
        "name": b.name,
        "url": b.url,
        "fileSize": b.file_size,
        "fileLastModified": b.file_last_modified,
    });
    if let Some(ref hash) = b.file_hash {
        body["fileHash"] = serde_json::Value::String(hash.clone());
    }
    body
}

fn bld_new_series(diff: &Diff, library_id: &str, _analyze: bool, _refresh: bool, config: &Config) -> Vec<String> {
    diff.new_series
        .iter()
        .flat_map(|s| {
            let mut lines = Vec::new();
            lines.push(format!("# Create series: {}", s.name));
            let body = serde_json::json!({
                "libraryId": library_id,
                "name": s.name,
                "url": s.url,
                "fileLastModified": s.file_last_modified,
                "books": s.books.iter().map(book_body).collect::<Vec<_>>(),
            });
            lines.push(curl_cmd("POST", "/api/v1/series", Some(&body), config));
            lines
        })
        .collect()
}

fn bld_new_books(
    diff: &Diff,
    library_id: &str,
    db_series_by_url: Option<&HashMap<String, DbSeries>>,
    fs_data: Option<&FsData>,
    config: &Config,
) -> Vec<String> {
    if diff.new_books.is_empty() {
        return vec![];
    }

    let mut lines = vec![format!("# New books in existing series ({}) total", diff.new_books.len())];

    if let (Some(db_map), Some(fs)) = (db_series_by_url, fs_data) {
        let by_series = diff::group_new_books_by_series(&diff.new_books, &fs.series, db_map);
        for (series_id, books) in &by_series {
            lines.push(format!("#  → series {} ({} books)", series_id, books.len()));
            let body = serde_json::json!({
                "libraryId": library_id,
                "books": books.iter().map(|b| {
                    serde_json::json!({
                        "name": b.name,
                        "url": b.url,
                        "fileSize": b.file_size,
                        "fileLastModified": b.file_last_modified,
                    })
                }).collect::<Vec<_>>(),
            });
            lines.push(curl_cmd("POST", &format!("/api/v1/series/{}/books", series_id), Some(&body), config));
        }
    } else {
        lines.push("# No series mapping available — triggering library scan".to_string());
        lines.push(curl_cmd(
            "POST",
            &format!("/api/v1/libraries/{}/scan?deep=false", library_id),
            None,
            config,
        ));
    }

    lines
}

fn bld_deleted_series(diff: &Diff, library_id: &str, config: &Config) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for s in &diff.deleted_series {
        lines.push(format!("# Delete series: {}", s.name));
        lines.push(curl_cmd("DELETE", &format!("/api/v1/series/{}/file", s.id), None, config));
    }
    lines.push(curl_cmd("POST", &format!("/api/v1/libraries/{}/empty-trash", library_id), None, config));
    lines
}

fn bld_deleted_books(diff: &Diff, library_id: &str, config: &Config) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for b in &diff.deleted_books {
        lines.push(format!("# Delete book: {}", b.name));
        lines.push(curl_cmd("DELETE", &format!("/api/v1/books/{}/file", b.id), None, config));
    }
    lines.push(curl_cmd("POST", &format!("/api/v1/libraries/{}/empty-trash", library_id), None, config));
    lines
}

fn bld_changed_books(diff: &Diff, analyze: bool, refresh: bool, config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    for b in &diff.changed_books {
        if analyze {
            lines.push(format!("# Analyze: {}", b.name));
            lines.push(curl_cmd("POST", &format!("/api/v1/books/{}/analyze", b.id), None, config));
        }
        if refresh {
            lines.push(format!("# Refresh metadata: {}", b.name));
            lines.push(curl_cmd("POST", &format!("/api/v1/books/{}/metadata/refresh", b.id), None, config));
        }
    }
    if refresh {
        let mut affected = std::collections::BTreeSet::new();
        for b in &diff.changed_books {
            if let Some(ref sid) = b.series_id {
                affected.insert(sid.clone());
            }
        }
        for sid in &affected {
            lines.push(format!("# Refresh series metadata: {}", sid));
            lines.push(curl_cmd("POST", &format!("/api/v1/series/{}/metadata/refresh", sid), None, config));
        }
    }
    lines
}

fn bld_to_be_analyzed(diff: &Diff, config: &Config) -> Vec<String> {
    diff.to_be_analyzed
        .iter()
        .flat_map(|b| {
            vec![
                format!("# Analyze: {}", b.name),
                curl_cmd("POST", &format!("/api/v1/books/{}/analyze", b.id), None, config),
            ]
        })
        .collect()
}

fn bld_no_metadata(diff: &Diff, config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    for b in &diff.no_metadata {
        lines.push(format!("# Refresh metadata: {}", b.name));
        lines.push(curl_cmd("POST", &format!("/api/v1/books/{}/metadata/refresh", b.id), None, config));
    }
    let mut affected = std::collections::BTreeSet::new();
    for b in &diff.no_metadata {
        if let Some(ref sid) = b.series_id {
            affected.insert(sid.clone());
        }
    }
    for sid in &affected {
        lines.push(format!("# Refresh series metadata: {}", sid));
        lines.push(curl_cmd("POST", &format!("/api/v1/series/{}/metadata/refresh", sid), None, config));
    }
    lines
}

fn bld_pending_hash(diff: &Diff, config: &Config) -> Vec<String> {
    diff.pending_hash
        .iter()
        .flat_map(|b| {
            let mut lines = Vec::new();
            if !b.file_hash.is_empty() {
                lines.push(format!("# Hash book: {} (pre-computed)", b.name));
                let body = serde_json::json!({"fileHash": b.file_hash});
                lines.push(curl_cmd("PUT", &format!("/api/v1/books/{}/hash", b.id), Some(&body), config));
            } else {
                lines.push(format!("# Hash book: {}", b.name));
                lines.push(curl_cmd("PUT", &format!("/api/v1/books/{}/hash", b.id), None, config));
            }
            lines
        })
        .collect()
}

// ── Live API apply ─────────────────────────────────────────

pub async fn apply(
    diff: &Diff,
    library_id: &str,
    api: &KomgaClient,
    dry_run: bool,
) {
    for s in &diff.new_series {
        if dry_run {
            tracing::info!("[DRY RUN] + Creating series: {}", s.name);
        } else {
            tracing::info!("+ Creating series: {}", s.name);
            let req = CreateSeriesRequest {
                library_id: library_id.to_string(),
                name: s.name.clone(),
                url: s.url.clone(),
                file_last_modified: s.file_last_modified.clone(),
                books: s
                    .books
                    .iter()
                    .map(|b| BookDto {
                        name: b.name.clone(),
                        url: b.url.clone(),
                        file_size: b.file_size,
                        file_last_modified: b.file_last_modified.clone(),
                        file_hash: b.file_hash.clone(),
                    })
                    .collect(),
            };
            if let Ok(created) = api.create_series(&req).await {
                if let Ok(books) = api.get_series_books(&created.id).await {
                    for b in &books {
                        let _ = api.analyze_book(&b.id).await;
                        let _ = api.refresh_book_metadata(&b.id).await;
                    }
                }
            }
        }
    }

    for s in &diff.deleted_series {
        if dry_run {
            tracing::info!("[DRY RUN] - Deleting series: {}", s.name);
        } else {
            tracing::info!("- Deleting series: {}", s.name);
            let _ = api.delete_series(&s.id).await;
        }
    }

    if !diff.deleted_books.is_empty() || !diff.deleted_series.is_empty() {
        if !dry_run {
            let _ = api.empty_trash(library_id).await;
        }
    }

    for b in &diff.changed_books {
        if dry_run {
            tracing::info!("[DRY RUN] ~ Analyzing: {}", b.name);
        } else {
            tracing::info!("~ Analyzing: {}", b.name);
            let _ = api.analyze_book(&b.id).await;
            let _ = api.refresh_book_metadata(&b.id).await;
        }
    }

    let mut affected = std::collections::BTreeSet::new();
    for b in &diff.changed_books {
        if let Some(ref sid) = b.series_id {
            affected.insert(sid.clone());
        }
    }
    for sid in &affected {
        if !dry_run {
            let _ = api.refresh_series_metadata(sid).await;
        }
    }
}
