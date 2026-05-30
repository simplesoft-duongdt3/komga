use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::models::*;

pub struct ExportPaths {
    pub db: String,
    pub fs: String,
}

pub fn export_snapshots(
    library_id: &str,
    library_name: &str,
    library_root: &str,
    db_series: &[DbSeries],
    db_books: &[DbBook],
    fs_data: &FsData,
    request_id: &str,
    export_dir: &Path,
) -> ExportPaths {
    let folder = export_dir.join(request_id);
    fs::create_dir_all(&folder).expect("Failed to create export directory");

    let j1 = serde_json::json!({
        "type": "J1 — DB Snapshot",
        "request_id": request_id,
        "library": {"id": library_id, "name": library_name, "root": library_root},
        "exported_at": Utc::now().to_rfc3339(),
        "series_count": db_series.len(),
        "books_count": db_books.len(),
        "series": db_series,
        "books": db_books,
    });

    let j1_path = folder.join(format!("{}_{}.json", request_id, "db"));
    fs::write(&j1_path, serde_json::to_string_pretty(&j1).unwrap()).expect("Failed to write J1");

    let fs_series_list: Vec<serde_json::Value> = fs_data
        .series
        .values()
        .map(|s| {
            serde_json::json!({
                "url": s.url,
                "name": s.name,
                "file_last_modified": s.file_last_modified,
                "books": s.books,
            })
        })
        .collect();

    let fs_books_count: usize = fs_data
        .series
        .values()
        .map(|s| s.books.len())
        .sum::<usize>()
        + fs_data.oneshots.len();

    let j2 = serde_json::json!({
        "type": "J2 — Filesystem Snapshot",
        "request_id": request_id,
        "library": {"root": library_root},
        "exported_at": Utc::now().to_rfc3339(),
        "series_count": fs_series_list.len(),
        "books_count": fs_books_count,
        "series": fs_series_list,
        "oneshots": fs_data.oneshots,
    });

    let j2_path = folder.join(format!("{}_{}.json", request_id, "fs"));
    fs::write(&j2_path, serde_json::to_string_pretty(&j2).unwrap()).expect("Failed to write J2");

    ExportPaths {
        db: j1_path.to_string_lossy().to_string(),
        fs: j2_path.to_string_lossy().to_string(),
    }
}
