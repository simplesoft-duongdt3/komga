# Compatibility Test Results

> Generated: 2026-05-30
> Fixture: `compat-tests/fixtures/test_cases.json` (24 test cases)

---

## Summary

| Implementation | Total | Passed | Failed |
|---|---|---|---|
| Python | 24 | 24 | 0 |
| Rust | 24 | 24 | 0 |

**Verdict: ALL TESTS MATCH** — Python and Rust produce identical results for every test case.

---

## 1. URL Normalization (8 cases)

Both implementations normalize Java-style `file:/path` and Python-style `file:///path/`
into a canonical `file:///path` form (no trailing slash).

| # | Input | Expected | Python | Rust | Match |
|---|---|---|---|---|---|
| 0 | `file:/data/library-sample/sample-series/` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | PASS |
| 1 | `file:/data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | PASS |
| 2 | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | PASS |
| 3 | `file:///data/library-sample/sample-series/` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | `file:///data/library-sample/sample-series` | PASS |
| 4 | `file:/data/.../sample_random_cool.pdf` | `file:///data/.../sample_random_cool.pdf` | `file:///data/.../sample_random_cool.pdf` | `file:///data/.../sample_random_cool.pdf` | PASS |
| 5 | `file:///data/.../sample_random_cool.pdf` | `file:///data/.../sample_random_cool.pdf` | `file:///data/.../sample_random_cool.pdf` | `file:///data/.../sample_random_cool.pdf` | PASS |
| 6 | `http://example.com` | `http://example.com` | `http://example.com` | `http://example.com` | PASS |
| 7 | `file://localhost/data/foo` | `file:///localhost/data/foo` | `file:///localhost/data/foo` | `file:///localhost/data/foo` | PASS |

---

## 2. mtime Comparison (6 cases)

Both implementations compare modification times at second-level precision,
stripping sub-second fractions and timezone suffixes.

| # | FS mtime | DB mtime | Expected | Python | Rust | Match |
|---|---|---|---|---|---|---|
| 0 | `2026-05-26T10:25:23.774+00:00` | `2026-05-26T10:25:24` | `false` | `false` | `false` | PASS |
| 1 | `2026-01-01T12:00:00.123+00:00` | `2026-01-01T12:00:00` | `true` | `true` | `true` | PASS |
| 2 | `2026-01-01T12:00:00` | `2026-01-01T12:00:00` | `true` | `true` | `true` | PASS |
| 3 | `2026-01-01T12:00:00` | `2026-01-01T12:00:01` | `false` | `false` | `false` | PASS |
| 4 | `""` (empty) | `2026-01-01T12:00:00` | `null` | `null` | `null` | PASS |
| 5 | `2026-01-01T12:00:00Z` | `2026-01-01T12:00:00` | `true` | `true` | `true` | PASS |

---

## 3. Diff Computation (10 cases)

Both implementations compute identical diffs given the same DB state and filesystem state.

### 3.1 — no_changes

DB: 1 series, 1 book. FS: same series, same book. No hash.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.2 — new_series

DB: empty. FS: 1 series with 1 book.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 1 | 1 | 1 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.3 — deleted_series

DB: 1 series, 1 book. FS: empty.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 1 | 1 | 1 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 1 | 1 | 1 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.4 — new_book_in_existing_series

DB: 1 series, 1 book. FS: same series, 2 books (one new).

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 1 | 1 | 1 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.5 — changed_book_different_mtime

DB: book with mtime `2026-01-01`. FS: same book with mtime `2026-02-01`.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 1 | 1 | 1 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.6 — changed_book_different_size

DB: book with size 1000. FS: same book with size 2000.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 1 | 1 | 1 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.7 — changed_book_different_hash

DB: book with hash `aaaa`. FS: same book with hash `bbbb`.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 1 | 1 | 1 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.8 — no_change_hash_matches

DB: book with hash `xxhash123`. FS: same book, same hash, different mtime.
Hash match overrides mtime difference — no change detected.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

### 3.9 — pending_hash_fs_has_db_empty

DB: book with empty hash. FS: same book with hash `xxhash`.
FS has hash but DB doesn't — goes to pending_hash, not changed_books.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 0 | 0 | 0 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 1 | 1 | 1 | PASS |

### 3.10 — integration_java_urls

DB: Java-style URLs (`file:/path/`). FS: Python-style URLs (`file:///path`).
Tests cross-format URL normalization with real-world data.

| Metric | Expected | Python | Rust | Match |
|---|---|---|---|---|
| new_series | 1 | 1 | 1 | PASS |
| deleted_series | 0 | 0 | 0 | PASS |
| new_books | 0 | 0 | 0 | PASS |
| deleted_books | 0 | 0 | 0 | PASS |
| changed_books | 0 | 0 | 0 | PASS |
| pending_hash | 0 | 0 | 0 | PASS |

---

## Rust Unit Tests

```
running 26 tests
test diff::tests::test_mtime_missing ................ ok
test diff::tests::test_mtime_same_second ............ ok
test diff::tests::test_mtime_different .............. ok
test diff::tests::test_mtime_diff_second ............ ok
test diff::tests::test_mtime_z_suffix ............... ok
test diff::tests::test_mtime_exact .................. ok
test diff::tests::test_deleted_series ............... ok
test diff::tests::test_changed_book_different_size .. ok
test diff::tests::test_changed_book_different_mtime . ok
test diff::tests::test_changed_book_different_hash .. ok
test diff::tests::test_no_change_when_hash_matches_diff_size  ok
test diff::tests::test_no_change_when_hash_matches .. ok
test diff::tests::test_new_series ................... ok
test diff::tests::test_new_book_in_existing_series .. ok
test diff::tests::test_group_new_books_by_series .... ok
test diff::tests::test_integration_real_data ........ ok
test diff::tests::test_normalize_file_url_book ...... ok
test diff::tests::test_no_changes ................... ok
test diff::tests::test_normalize_file_url_db_fs_match  ok
test diff::tests::test_normalize_file_url_empty_host  ok
test diff::tests::test_normalize_file_url_java_db_single_slash  ok
test diff::tests::test_normalize_file_url_java_db_single_slash_no_trailing  ok
test diff::tests::test_normalize_file_url_non_file .. ok
test diff::tests::test_normalize_file_url_python_triple_slash  ok
test diff::tests::test_normalize_file_url_trailing_slash  ok
test diff::tests::test_pending_hash_when_fs_hash_exists_db_empty  ok

test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## How to Reproduce

```bash
# From repo root
python3 scan-library-tool/compat-tests/compare.py
```

Exit code 0 = all match. Exit code 1 = differences found.
