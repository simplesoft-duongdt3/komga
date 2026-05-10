# Solution: Komga Fast Library Scan & Analyze Tool

## Problem

Komga's built-in library scanner is too slow for large libraries (1500+ series, 300k+ books). A full rescan takes hours, and adding new books to an existing library is particularly slow as Komga re-analyzes all books on every scan.

## Architecture

```
                    ┌──────────────┐
                    │   config.py  │  env vars + dataclass defaults
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼─────┐     ┌──────▼──────┐    ┌──────▼──────┐
   │ scanner  │     │   syncer    │    │  analyzer   │
   │          │     │             │    │             │
   │ fs walk  │────▶│ disk vs DB  │    │ PyMuPDF     │
   │ series   │     │  diff       │    │ page count  │
   │  .json   │     │             │    │ cropBox dims│
   │ thumbs   │     │ DiffResult  │    │ SHA-256     │
   └──────────┘     └──────┬──────┘    └──────┬──────┘
                           │                  │
                    ┌──────▼──────┐    ┌──────▼──────┐
                    │  db.py      │    │ sql_exporter│
                    │             │    │             │
                    │ PostgreSQL  │    │ scan.sql    │
                    │ CRUD batch  │    │ analyze.sql │
                    └─────────────┘    └─────────────┘
```

### Phase 1: Scan (`scanner.py`)

```
Filesystem Root
  ├── series-a/           ← series directory
  │   ├── series.json      ← Mylar metadata (title, status, summary)
  │   ├── poster.jpg       ← series thumbnail (SIDECAR)
  │   ├── book-001.pdf     ← book file
  │   ├── book-001.jpg     ← book thumbnail (matched by basename)
  │   ├── book-002.pdf
  │   └── book-002-0001.jpg
  └── series-b/
```

- **Parallel traversal**: `ThreadPoolExecutor` with configurable workers (default: `cpu_count × 2`, max 32)
- **File filtering**: Only `.pdf` books and `.jpg`/`.jpeg` thumbnails
- **series.json parsing**: Extracts `name`, `description_text`, `status` (Mylar/ComicRack format), `publisher`, `age_rating`, `total_issues`
- **Natural sorting**: Books sorted by filename with numeric-aware comparison (`_natural_sort_key`)
- **Thumbnail matching**: 
  - Series: `poster.jpg`, `cover.jpg`, `default.jpg`, `folder.jpg`, `series.jpg`
  - Books: exact basename match then `{basename}-{number_padded}.jpg` fallback
- **Path mapping**: `_to_docker_path(real_path, real_root, docker_root)` — converts real filesystem paths to Docker paths stored in DB

### Phase 2: Diff (`syncer.py`)

Compares filesystem state against database state:

| Scenario | Action |
|----------|--------|
| Series on disk, not in DB | INSERT SERIES + SERIES_METADATA |
| Book on disk, not in DB | INSERT BOOK + BOOK_METADATA + MEDIA (STATUS='UNKNOWN') |
| Book on disk, was soft-deleted | Reactivate (DELETED_DATE = NULL) |
| Series in DB, not on disk | Soft-delete SERIES + cascade soft-delete BOOKs |
| Series count changed | UPDATE SERIES.BOOK_COUNT |
| series.json changed | UPDATE SERIES_METADATA |
| New/changed thumbnails | INSERT/UPDATE THUMBNAIL_SERIES, THUMBNAIL_BOOK |
| Missing thumbnails | DELETE THUMBNAIL rows |

### Phase 3: Apply (`main.py` → `db.py`)

Two modes:
- **Direct DB**: Write operations executed via `psycopg2` connection pool
- **SQL export**: All operations serialized to a `.sql` file with `BEGIN/COMMIT`

### Phase 4: Analyze (`analyzer.py` → `db.py`)

Only runs when `--analyze` flag is set. Processes books with `MEDIA.STATUS = 'UNKNOWN'`:

```
unanalyzed book → fitz.open() → doc.page_count
                    ↓
              for page in doc:
                page.cropbox → width, height
                    ↓
              hashlib.sha256(64KB chunks) → hex digest
                    ↓
         UPDATE MEDIA: STATUS='READY', PAGE_COUNT, MEDIA_TYPE
         INSERT MEDIA_PAGE: per-page dimensions (ON CONFLICT upsert)
         UPDATE BOOK: FILE_HASH
```

## Key Design Decisions

### Why two separate SQL files (scan.sql + analyze.sql)?

The analyze SQL references `BOOK.ID` values that the scan SQL creates. They have a strict dependency order:

```
psql scan.sql    → INSERT BOOK rows (STATUS='UNKNOWN')
psql analyze.sql → UPDATE those BOOKs (STATUS='READY')
```

Merging them into one file would reference IDs that don't exist yet, causing foreign key violations.

### Why `cropBox` not `mediaBox`?

Komga's `PdfExtractor.getPages()` uses `cropBox` for page dimensions. Using `mediaBox` would produce different dimensions than Komga, potentially breaking layout-dependent features. PyMuPDF's `page.rect` gives `mediaBox` — we explicitly use `page.cropbox` to match.

### Why no GENERATED thumbnails?

Komga generates thumbnails from the first page of each PDF. Our scan tool inserts SIDECAR JPGs (`poster.jpg`, `{book}-0001.jpg`) from the filesystem, which are sufficient. Komga's frontend uses SIDECAR thumbnails first, falling back to GENERATED only when SIDECAR is absent. Skipping thumbnail generation reduces the analyze phase time significantly.

### Why batch commits?

For 300k books averaging 50 pages each = 15M `MEDIA_PAGE` rows. Inserting all at once would require holding 15M rows in memory. Batching at 5000 books per commit (~250k page rows per batch) keeps memory usage bounded.

### Why `ThreadPoolExecutor` for analysis?

PDF analysis is CPU-bound for small files and I/O-bound for large ones. `ThreadPoolExecutor` provides good throughput for mixed workloads. Each thread opens/closes its own `fitz.Document` — PyMuPDF is thread-safe per document instance.

## Database Schema Mapping

### MEDIA (1:1 with BOOK)

| Column | Source | Scan value | Analyze value |
|--------|--------|-----------|---------------|
| `BOOK_ID` | PK | Generated UUID | — |
| `STATUS` | — | `'UNKNOWN'` | `'READY'` or `'ERROR'` |
| `PAGE_COUNT` | PyMuPDF | `0` | Actual count |
| `MEDIA_TYPE` | — | `NULL` | `'application/pdf'` |
| `COMMENT` | Error | `NULL` | Error message (truncated to 2000 chars) |

### MEDIA_PAGE (composite PK: BOOK_ID + NUMBER)

| Column | Source |
|--------|--------|
| `BOOK_ID` | FK to BOOK |
| `NUMBER` | 1-indexed page number |
| `FILE_NAME` | `str(page_number)` (e.g., `"3"`) |
| `MEDIA_TYPE` | `""` (empty string, not null) |
| `WIDTH` | `int(cropbox.width)` |
| `HEIGHT` | `int(cropbox.height)` |
| `FILE_HASH` | Not set (default `""`) |

### BOOK

| Column | Analyze source |
|--------|---------------|
| `FILE_HASH` | `hashlib.sha256()` of entire file |

## Path Mapping

Komga stores Docker container paths in the database (e.g., `/data/books/series/book.pdf`). The tool runs on the host filesystem with real paths (e.g., `/volume1/Shared/books/series/book.pdf`).

```python
# Scanner: real → docker (stored in DB)
def _to_docker_path(real_path, real_root, docker_root):
    return docker_root + real_path[len(real_root):]

# Analyzer: docker → real (to open files)
def _docker_to_real_path(docker_path, real_root, docker_root):
    return real_root + docker_path[len(docker_root):]
```

## Performance Model

| Phase | Operation | Time per book | 300k books (16 workers) |
|-------|-----------|---------------|------------------------|
| Scan | Filesystem walk + diff | ~0.5 ms | ~2.5 min |
| Analyze | Page count only | ~5 ms | ~1.6 min |
| Analyze | + dimensions | ~10 ms | ~3 min |
| Analyze | + SHA-256 hash | ~100 ms (30 MB) | ~30 min |
| Analyze | Full | ~110 ms | ~35 min |
| Analyze | Full (no-hash) | ~10 ms | ~3 min |

**SHA-256 hashing is the bottleneck** (~90% of full analysis time). Use `--no-hash` when hashing is not needed.

## Error Handling

| Error | MEDIA.STATUS | MEDIA.COMMENT |
|-------|-------------|---------------|
| PDF file not found | `'ERROR'` | `str(FileNotFoundError)` |
| PDF corrupted / can't open | `'ERROR'` | `str(PyMuPDF error)` |
| All other exceptions | `'ERROR'` | `str(e)[:2000]` |

Komga uses error codes like `ERR_1000`, `ERR_1018` in its COMMENT column. Our tool uses the exception message directly — simpler and equally informative for debugging.

## Concurrency Model

- **Scanner**: `ThreadPoolExecutor` for parallel directory traversal (I/O-bound)
- **Analyzer**: `ThreadPoolExecutor` for parallel PDF processing (mixed CPU/I/O-bound)
- **DB writes**: Serial per batch — one connection at a time for write operations
- **Connection pool**: `psycopg2.pool.ThreadedConnectionPool` with configurable min/max connections
- **Thread safety**: Each thread opens/closes its own PyMuPDF document. No shared state.
