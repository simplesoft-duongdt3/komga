use std::collections::{HashMap, HashSet};

use crate::models::*;

pub fn normalize_file_url(url: &str) -> String {
    if !url.contains("file:") {
        return url.to_string();
    }
    // strip scheme (file:, file://, file:///) and leading/trailing slashes
    let path = url
        .trim_start_matches("file:")
        .trim_start_matches('/')
        .trim_end_matches('/');
    format!("file:///{}", path)
}

pub fn mtime_equals(fs_mtime: &str, db_mtime: &str) -> Option<bool> {
    let fs_trimmed = fs_mtime
        .split('.')
        .next()
        .unwrap_or(fs_mtime)
        .replace('Z', "")
        .replace('z', "");
    let db_trimmed = db_mtime
        .split('.')
        .next()
        .unwrap_or(db_mtime)
        .replace('Z', "")
        .replace('z', "");

    if fs_trimmed.is_empty() || db_trimmed.is_empty() {
        return None;
    }
    Some(fs_trimmed == db_trimmed)
}

fn get_parent_series_url(book_url: &str, fs_series: &HashMap<String, &FsSeries>) -> Option<String> {
    let parsed_book = url::Url::parse(book_url).ok()?;
    let book_path = std::path::Path::new(parsed_book.path());

    for series_url in fs_series.keys() {
        let parsed_series = url::Url::parse(series_url).ok()?;
        let series_path = std::path::Path::new(parsed_series.path());
        if book_path.strip_prefix(series_path).is_ok() {
            return Some(series_url.clone());
        }
    }
    None
}

pub fn compute(
    series_from_db: &[DbSeries],
    books_from_db: &[DbBook],
    fs_data: &FsData,
) -> Diff {
    let db_series_by_url: HashMap<String, &DbSeries> = series_from_db
        .iter()
        .map(|s| (normalize_file_url(&s.url), s))
        .collect();
    let db_books_by_url: HashMap<String, &DbBook> = books_from_db
        .iter()
        .map(|b| (normalize_file_url(&b.url), b))
        .collect();

    let fs_series_normalized: HashMap<String, &FsSeries> = fs_data
        .series
        .iter()
        .map(|(url, s)| (normalize_file_url(url), s))
        .collect();

    let mut fs_books_by_url: HashMap<String, &FsBook> = HashMap::new();
    for s in fs_data.series.values() {
        for b in &s.books {
            fs_books_by_url.insert(normalize_file_url(&b.url), b);
        }
    }
    for b in &fs_data.oneshots {
        fs_books_by_url.insert(normalize_file_url(&b.url), b);
    }

    let db_urls_s: HashSet<&str> = db_series_by_url.keys().map(|s| s.as_str()).collect();
    let fs_urls_s: HashSet<&str> = fs_series_normalized.keys().map(|s| s.as_str()).collect();
    let db_urls_b: HashSet<&str> = db_books_by_url.keys().map(|s| s.as_str()).collect();
    let fs_urls_b: HashSet<&str> = fs_books_by_url.keys().map(|s| s.as_str()).collect();

    let mut diff = Diff::default();

    // New series
    for url in fs_urls_s.difference(&db_urls_s) {
        if let Some(series) = fs_series_normalized.get(*url) {
            diff.new_series.push((*series).clone());
        }
    }

    // Deleted series
    for url in db_urls_s.difference(&fs_urls_s) {
        if let Some(series) = db_series_by_url.get(*url) {
            diff.deleted_series.push((*series).clone());
        }
    }

    // Build set of new series URLs for exclusion
    let new_series_urls: HashSet<String> = diff
        .new_series
        .iter()
        .map(|s| normalize_file_url(&s.url))
        .collect();

    // New books (excluding those in new series)
    for url in fs_urls_b.difference(&db_urls_b) {
        let parent = get_parent_series_url(url, &fs_series_normalized);
        if let Some(p) = parent {
            if new_series_urls.contains(&p) {
                continue;
            }
        }
        if let Some(book) = fs_books_by_url.get(*url) {
            diff.new_books.push((*book).clone());
        }
    }

    // Deleted books
    for url in db_urls_b.difference(&fs_urls_b) {
        if let Some(book) = db_books_by_url.get(*url) {
            diff.deleted_books.push((*book).clone());
        }
    }

    // Changed books (same URL, different content)
    for url in db_urls_b.intersection(&fs_urls_b) {
        let fs_b = fs_books_by_url[*url];
        let db_b = db_books_by_url[*url];

        let mut changed = false;

        let fs_hash = fs_b.file_hash.as_deref().unwrap_or("");
        let db_hash = db_b.file_hash.as_deref().unwrap_or("");

        if !fs_hash.is_empty() && !db_hash.is_empty() {
            changed = fs_hash != db_hash;
        } else if !db_hash.is_empty() && fs_hash.is_empty() {
            changed = {
                let mtime_match = match (
                    fs_b.file_last_modified.as_str(),
                    db_b.file_last_modified.as_deref(),
                ) {
                    (fs_m, Some(db_m)) => mtime_equals(fs_m, db_m),
                    _ => None,
                };
                let size_match = Some(fs_b.file_size) == db_b.file_size.map(|s| s as u64);
                mtime_match != Some(true) || !size_match
            };
        } else if !fs_hash.is_empty() && db_hash.is_empty() {
            diff.pending_hash.push(PendingHashBook {
                id: db_b.id.clone(),
                name: db_b.name.clone(),
                series_id: db_b.series_id.clone(),
                url: url.to_string(),
                file_hash: fs_hash.to_string(),
            });
        } else {
            let mtime_match = match (
                fs_b.file_last_modified.as_str(),
                db_b.file_last_modified.as_deref(),
            ) {
                (fs_m, Some(db_m)) => mtime_equals(fs_m, db_m),
                _ => None,
            };
            let size_match = Some(fs_b.file_size) == db_b.file_size.map(|s| s as u64);
            changed = mtime_match != Some(true) || !size_match;
        }

        if changed {
            diff.changed_books.push(ChangedBook {
                id: db_b.id.clone(),
                name: db_b.name.clone(),
                series_id: db_b.series_id.clone(),
                url: url.to_string(),
            });
        }
    }

    diff
}

pub fn group_new_books_by_series(
    books: &[FsBook],
    fs_series: &HashMap<String, FsSeries>,
    db_series_by_url: &HashMap<String, DbSeries>,
) -> HashMap<String, Vec<FsBook>> {
    let mut result: HashMap<String, Vec<FsBook>> = HashMap::new();

    let fs_series_norm: HashMap<String, &FsSeries> = fs_series
        .iter()
        .map(|(u, s)| (normalize_file_url(u), s))
        .collect();
    let db_series_norm: HashMap<String, &DbSeries> = db_series_by_url
        .iter()
        .map(|(u, d)| (normalize_file_url(u), d))
        .collect();

    for b in books {
        if let Some(parent_url) = get_parent_series_url(&b.url, &fs_series_norm) {
            if let Some(db_s) = db_series_norm.get(&parent_url) {
                result
                    .entry(db_s.id.clone())
                    .or_default()
                    .push(b.clone());
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_file_url_java_db_single_slash() {
        assert_eq!(
            normalize_file_url("file:/data/library-sample/sample-series/"),
            "file:///data/library-sample/sample-series"
        );
    }

    #[test]
    fn test_normalize_file_url_java_db_single_slash_no_trailing() {
        assert_eq!(
            normalize_file_url("file:/data/library-sample/sample-series"),
            "file:///data/library-sample/sample-series"
        );
    }

    #[test]
    fn test_normalize_file_url_python_triple_slash() {
        assert_eq!(
            normalize_file_url("file:///data/library-sample/sample-series"),
            "file:///data/library-sample/sample-series"
        );
    }

    #[test]
    fn test_normalize_file_url_trailing_slash() {
        assert_eq!(
            normalize_file_url("file:///data/library-sample/sample-series/"),
            "file:///data/library-sample/sample-series"
        );
    }

    #[test]
    fn test_normalize_file_url_db_fs_match() {
        let db = normalize_file_url("file:/data/library-sample/sample-series/");
        let fs = normalize_file_url("file:///data/library-sample/sample-series");
        assert_eq!(db, fs);
        assert_eq!(db, "file:///data/library-sample/sample-series");
    }

    #[test]
    fn test_normalize_file_url_book() {
        let db = normalize_file_url("file:/data/library-sample/sample-series/sample_random_cool.pdf");
        let fs = normalize_file_url("file:///data/library-sample/sample-series/sample_random_cool.pdf");
        assert_eq!(db, fs);
    }

    #[test]
    fn test_normalize_file_url_non_file() {
        assert_eq!(normalize_file_url("http://example.com"), "http://example.com");
    }

    #[test]
    fn test_normalize_file_url_empty_host() {
        assert_eq!(
            normalize_file_url("file://localhost/data/foo"),
            "file:///localhost/data/foo"
        );
    }

    #[test]
    fn test_mtime_diff_second() {
        assert_eq!(
            mtime_equals("2026-05-26T10:25:23.774395+00:00", "2026-05-26T10:25:24"),
            Some(false)
        );
    }

    #[test]
    fn test_mtime_same_second() {
        assert_eq!(
            mtime_equals("2026-01-01T12:00:00.123456+00:00", "2026-01-01T12:00:00"),
            Some(true)
        );
    }

    #[test]
    fn test_mtime_exact() {
        assert_eq!(
            mtime_equals("2026-01-01T12:00:00", "2026-01-01T12:00:00"),
            Some(true)
        );
    }

    #[test]
    fn test_mtime_different() {
        assert_eq!(
            mtime_equals("2026-01-01T12:00:00", "2026-01-01T12:00:01"),
            Some(false)
        );
    }

    #[test]
    fn test_mtime_missing() {
        assert!(mtime_equals("", "2026-01-01T12:00:00").is_none());
    }

    #[test]
    fn test_mtime_z_suffix() {
        assert_eq!(
            mtime_equals("2026-01-01T12:00:00Z", "2026-01-01T12:00:00"),
            Some(true)
        );
    }

    fn make_series(id: &str, name: &str, url: &str, mtime: &str) -> DbSeries {
        DbSeries {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            file_last_modified: Some(mtime.to_string()),
        }
    }

    fn make_book(id: &str, name: &str, url: &str, mtime: &str, size: i64, hash: &str, sid: &str) -> DbBook {
        DbBook {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            file_last_modified: Some(mtime.to_string()),
            file_size: Some(size),
            file_hash: Some(hash.to_string()),
            series_id: Some(sid.to_string()),
        }
    }

    fn make_fs_book(url: &str, name: &str, size: u64, mtime: &str, hash: Option<&str>) -> FsBook {
        FsBook {
            url: url.to_string(),
            name: name.to_string(),
            file_size: size,
            file_last_modified: mtime.to_string(),
            file_hash: hash.map(|s| s.to_string()),
        }
    }

    fn make_fs_series(url: &str, name: &str, mtime: &str, books: Vec<FsBook>) -> FsSeries {
        FsSeries {
            url: url.to_string(),
            name: name.to_string(),
            file_last_modified: mtime.to_string(),
            books,
        }
    }

    #[test]
    fn test_no_changes() {
        let series = vec![make_series("s1", "Series A", "file:///root/SeriesA", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/SeriesA/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/SeriesA".to_string(),
                make_fs_series("file:///root/SeriesA", "Series A", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/SeriesA/Ch01.pdf", "Ch01", 1000, "2026-01-01T00:00:00+00:00", None),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.new_series.len(), 0);
        assert_eq!(d.deleted_series.len(), 0);
        assert_eq!(d.new_books.len(), 0);
        assert_eq!(d.deleted_books.len(), 0);
        assert_eq!(d.changed_books.len(), 0);
    }

    #[test]
    fn test_new_series() {
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/SeriesA".to_string(),
                make_fs_series("file:///root/SeriesA", "Series A", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/SeriesA/Ch01.pdf", "Ch01", 1000, "2026-01-01T00:00:00+00:00", None),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&[], &[], &fs);
        assert_eq!(d.new_series.len(), 1);
        assert_eq!(d.new_series[0].name, "Series A");
        assert_eq!(d.new_books.len(), 0);
    }

    #[test]
    fn test_deleted_series() {
        let series = vec![make_series("s1", "Series A", "file:///root/SeriesA", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/SeriesA/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1")];
        let fs = FsData { series: HashMap::new(), oneshots: vec![] };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.deleted_series.len(), 1);
        assert_eq!(d.deleted_series[0].id, "s1");
        assert_eq!(d.deleted_books.len(), 1);
        assert_eq!(d.deleted_books[0].id, "b1");
    }

    #[test]
    fn test_new_book_in_existing_series() {
        let series = vec![make_series("s1", "Series A", "file:///root/SeriesA", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/SeriesA/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/SeriesA".to_string(),
                make_fs_series("file:///root/SeriesA", "Series A", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/SeriesA/Ch01.pdf", "Ch01", 1000, "2026-01-01T00:00:00+00:00", None),
                    make_fs_book("file:///root/SeriesA/Ch02.pdf", "Ch02", 2000, "2026-01-02T00:00:00+00:00", None),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.new_books.len(), 1);
        assert_eq!(d.new_books[0].name, "Ch02");
    }

    #[test]
    fn test_changed_book_different_mtime() {
        let series = vec![make_series("s1", "S", "file:///root/S", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/S/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/S".to_string(),
                make_fs_series("file:///root/S", "S", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/S/Ch01.pdf", "Ch01", 1000, "2026-02-01T00:00:00+00:00", None),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.changed_books.len(), 1);
        assert_eq!(d.changed_books[0].id, "b1");
    }

    #[test]
    fn test_changed_book_different_size() {
        let series = vec![make_series("s1", "S", "file:///root/S", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/S/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/S".to_string(),
                make_fs_series("file:///root/S", "S", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/S/Ch01.pdf", "Ch01", 2000, "2026-01-01T00:00:00+00:00", None),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.changed_books.len(), 1);
        assert_eq!(d.changed_books[0].id, "b1");
    }

    #[test]
    fn test_changed_book_different_hash() {
        let series = vec![make_series("s1", "S", "file:///root/S", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/S/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "aaaa", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/S".to_string(),
                make_fs_series("file:///root/S", "S", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/S/Ch01.pdf", "Ch01", 1000, "2026-01-01T00:00:00+00:00", Some("bbbb")),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.changed_books.len(), 1);
    }

    #[test]
    fn test_no_change_when_hash_matches() {
        let series = vec![make_series("s1", "S", "file:///root/S", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/S/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "xxhash123", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/S".to_string(),
                make_fs_series("file:///root/S", "S", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/S/Ch01.pdf", "Ch01", 1000, "2026-02-02T00:00:00+00:00", Some("xxhash123")),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.changed_books.len(), 0);
    }

    #[test]
    fn test_no_change_when_hash_matches_diff_size() {
        let series = vec![make_series("s1", "S", "file:///root/S", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/S/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "xxhash456", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/S".to_string(),
                make_fs_series("file:///root/S", "S", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/S/Ch01.pdf", "Ch01", 9999, "2026-01-01T00:00:00+00:00", Some("xxhash456")),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.changed_books.len(), 0);
    }

    #[test]
    fn test_pending_hash_when_fs_hash_exists_db_empty() {
        let series = vec![make_series("s1", "S", "file:///root/S", "2026-01-01T00:00:00+00:00")];
        let books = vec![make_book("b1", "Ch01", "file:///root/S/Ch01.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1")];
        let fs = FsData {
            series: HashMap::from([(
                "file:///root/S".to_string(),
                make_fs_series("file:///root/S", "S", "2026-01-01T00:00:00+00:00", vec![
                    make_fs_book("file:///root/S/Ch01.pdf", "Ch01", 1000, "2026-01-01T00:00:00+00:00", Some("xxhash")),
                ]),
            )]),
            oneshots: vec![],
        };

        let d = compute(&series, &books, &fs);
        assert_eq!(d.pending_hash.len(), 1);
        assert_eq!(d.pending_hash[0].id, "b1");
        assert_eq!(d.changed_books.len(), 0);
    }

    #[test]
    fn test_group_new_books_by_series() {
        let new_books = vec![
            make_fs_book("file:///root/SeriesA/Ch03.pdf", "Ch03", 3000, "...", None),
            make_fs_book("file:///root/SeriesB/New.pdf", "New", 4000, "...", None),
        ];
        let fs_series = HashMap::from([
            ("file:///root/SeriesA".to_string(), make_fs_series("file:///root/SeriesA", "A", "...", vec![])),
            ("file:///root/SeriesB".to_string(), make_fs_series("file:///root/SeriesB", "B", "...", vec![])),
        ]);
        let db_series_by_url = HashMap::from([
            ("file:///root/SeriesA".to_string(), make_series("id-A", "A", "file:///root/SeriesA", "...")),
            ("file:///root/SeriesB".to_string(), make_series("id-B", "B", "file:///root/SeriesB", "...")),
        ]);

        let result = group_new_books_by_series(&new_books, &fs_series, &db_series_by_url);
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("id-A"));
        assert_eq!(result["id-A"].len(), 1);
        assert_eq!(result["id-A"][0].name, "Ch03");
        assert!(result.contains_key("id-B"));
        assert_eq!(result["id-B"].len(), 1);
        assert_eq!(result["id-B"][0].name, "New");
    }

    #[test]
    fn test_integration_real_data() {
        let db_series = vec![
            make_series("s1", "sample-series", "file:/data/library-sample/sample-series/", "2026-01-01T00:00:00+00:00"),
        ];
        let db_books = vec![
            make_book("b1", "sample_random_cool", "file:/data/library-sample/sample-series/sample_random_cool.pdf", "2026-01-01T00:00:00+00:00", 1000, "", "s1"),
            make_book("b2", "sample_random_sage", "file:/data/library-sample/sample-series/sample_random_sage.pdf", "2026-01-01T00:00:00+00:00", 2000, "", "s1"),
            make_book("b3", "sample_random_warm", "file:/data/library-sample/sample-series/sample_random_warm.pdf", "2026-01-01T00:00:00+00:00", 3000, "", "s1"),
        ];
        let fs = FsData {
            series: HashMap::from([
                (
                    "file:///data/library-sample/sample-series".to_string(),
                    make_fs_series("file:///data/library-sample/sample-series", "sample-series", "2026-01-01T00:00:00+00:00", vec![
                        make_fs_book("file:///data/library-sample/sample-series/sample_random_cool.pdf", "sample_random_cool", 1000, "2026-01-01T00:00:00+00:00", None),
                        make_fs_book("file:///data/library-sample/sample-series/sample_random_sage.pdf", "sample_random_sage", 2000, "2026-01-01T00:00:00+00:00", None),
                        make_fs_book("file:///data/library-sample/sample-series/sample_random_warm.pdf", "sample_random_warm", 3000, "2026-01-01T00:00:00+00:00", None),
                    ]),
                ),
                (
                    "file:///data/library-sample/sample-series-2".to_string(),
                    make_fs_series("file:///data/library-sample/sample-series-2", "sample-series-2", "2026-01-02T00:00:00+00:00", vec![
                        make_fs_book("file:///data/library-sample/sample-series-2/sample_random_cool.pdf", "sample_random_cool", 1000, "2026-01-02T00:00:00+00:00", None),
                        make_fs_book("file:///data/library-sample/sample-series-2/sample_random_sage.pdf", "sample_random_sage", 2000, "2026-01-02T00:00:00+00:00", None),
                    ]),
                ),
            ]),
            oneshots: vec![],
        };

        let d = compute(&db_series, &db_books, &fs);
        assert_eq!(d.new_series.len(), 1);
        assert_eq!(d.new_series[0].name, "sample-series-2");
        assert_eq!(d.deleted_series.len(), 0);
        assert_eq!(d.deleted_books.len(), 0);
        assert_eq!(d.changed_books.len(), 0);
    }
}
