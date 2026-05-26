# Komga Smart Scanner

External diff-based scanner for Komga. Walks the filesystem and reads
PostgreSQL directly to compute what changed, then drives Komga's REST API
— avoiding the full `ScanLibrary` pipeline.

Architecture docs: [plan file](../.kilo/plans/1779787463034-nimble-cactus.md)

---

## Quick Start

### Prerequisites

- Komga running with PostgreSQL (not SQLite)
- Python 3.12+

### Run via Docker Compose

```bash
build-local-docker.sh
docker tag komga-local:$VERSION komga-local:latest

docker compose build --no-cache smart-scanner && docker compose up -d
```

Open http://localhost:5050

### Local development

```bash
cd scan-library-tool
pip install -r requirements.txt

# Set env vars (see config.py)
export KOMGA_BASE_URL=http://localhost:25600
export KOMGA_USER=admin@example.com
export KOMGA_PASSWORD=your-password
export PG_HOST=localhost
export PG_PORT=5432
export PG_DB=komga
export PG_USER=komga
export PG_PASSWORD=komga123

python server.py
```

---

## Web UI

The tool runs a web server at port 5050 using [FuncToWeb](https://github.com/offerrall/FuncToWeb) — auto-generated UI from typed Python functions. No HTML templates required.

### Flow

1. **List libraries** — `/list-libraries` shows an ActionTable of all Komga libraries. Click one to scan.

2. **Scan library** — Reads PostgreSQL + walks filesystem, computes a diff, and displays results:

   | Category | Description |
   |---|---|
   | ✨ New series | Directory exists on disk, not in DB |
   | 🗑 Deleted series | In DB but directory missing from disk |
   | 📄 New books | PDF file on disk, not in DB (inside an existing series) |
   | 🗑 Deleted books | In DB but file missing from disk |
   | ✏ Changed books | Same file, different mtime/size/hash |

   JSON snapshots are exported to `./exports/` for offline comparison.

3. **Choose action** — Two action rows appear below the diff tables:

   | Action | What it does |
   |---|---|
   | 📋 Export curl commands | Generates a bash script with all API calls |
   | 🚀 Apply via API | Calls Komga APIs directly with live streaming output |

### Scan options

| Option | Default | Description |
|---|---|---|
| 🔐 **Compute file hashes** | `False` | SHA-256 hash each PDF file. Slower but detects content changes even when mtime/size match. |

### Per-action options

Set when you click **Export curl** or **Apply via API**:

| Option | Default | Description |
|---|---|---|
| 📖 **Analyze books** | `True` | Whether to include `POST /api/v1/books/{id}/analyze` calls |
| 🔄 **Refresh metadata** | `True` | Whether to include `POST /api/v1/books/{id}/metadata/refresh` + `POST /api/v1/series/{id}/metadata/refresh` |

---

## Files

```
scan-library-tool/
├── server.py        # FuncToWeb web server (entry point)
├── main.py          # CLI entry point (manual mode)
├── config.py        # Environment variable configuration
├── db.py            # PostgreSQL reader
├── walker.py        # PDF-only filesystem walker
├── diff.py          # Diff engine with URL normalization
├── api.py           # Komga REST API client
├── applier.py       # Orchestrator (apply diff via API)
├── export_json.py   # J1 (DB) / J2 (FS) JSON export
├── Dockerfile       # Container build
├── requirements.txt # Python dependencies
└── tests/           # Unit tests
```

---

## CLI Mode

```bash
python main.py
```

Interactive prompt — select a library, see diff, confirm to apply.

---

## JSON Exports

Every scan writes two files to the `./exports/` directory:

| File | Contents |
|---|---|
| `{library}_J1_db_{timestamp}.json` | DB snapshot (current series + books from PostgreSQL) |
| `{library}_J2_fs_{timestamp}.json` | Filesystem snapshot (directories + PDF files) |

Use these to debug URL matching issues (see `_normalize_file_url` in `diff.py`).

---

## Curl Export

When you select **Export curl commands**, a bash script is written to
`./exports/{library}_curl_{timestamp}.sh`. It contains every API call needed
to synchronize Komga with the detected changes:

```bash
# Create new series with books
curl -X POST 'http://komga:25600/api/v1/series' \
  -u 'admin@example.com:password' \
  -d '{"libraryId": "...", "name": "...", "books": [...]}'

# Delete removed books/series
curl -X DELETE 'http://komga:25600/api/v1/books/{id}/file'
curl -X DELETE 'http://komga:25600/api/v1/series/{id}/file'

# Empty trash (if any deletions)
curl -X POST 'http://komga:25600/api/v1/libraries/{id}/empty-trash'

# Analyze and refresh changed books
curl -X POST 'http://komga:25600/api/v1/books/{id}/analyze'
curl -X POST 'http://komga:25600/api/v1/books/{id}/metadata/refresh'

# Refresh affected series metadata
curl -X POST 'http://komga:25600/api/v1/series/{id}/metadata/refresh'
```

Run the script: `bash /exports/file.sh`

---

## Architecture

```
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│  Python Tool  │────▶│  Komga REST   │────▶│  PostgreSQL   │
│  (diff+apply) │     │  (new +       │     │  (read-only)  │
│               │     │   existing)   │     │               │
└───────┬───────┘     └───────────────┘     └───────────────┘
        │
        │ reads
        ▼
┌───────────────┐
│  Filesystem   │
│  (pdf only)   │
└───────────────┘
```

The tool reads PostgreSQL directly for the current DB state and walks the
filesystem for the current disk state. All mutations go through the Komga
REST API — no DB writes from Python.

A new `POST /api/v1/series` endpoint was added to Komga to support creating
series + registering books that already exist on disk. See the Kotlin changes
in `src/main/kotlin/org/gotson/komga/interfaces/api/rest/SeriesController.kt`.

---

## URL Normalization

Java's `URL.toString()` on JDK 21+ produces `file:/path` (single slash),
while Python's `Path.as_uri()` produces `file:///path` (triple slash). The
diff engine normalizes all URLs via `_normalize_file_url()` before comparison.

Handles:
- `file:/path` → `file:///path`
- `file:///path/` → `file:///path` (strips trailing slash from directories)

---

## Testing

```bash
cd scan-library-tool
PYTHONPATH=. python3 -m unittest tests.test_walker tests.test_diff -v
```

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `KOMGA_BASE_URL` | `http://localhost:8080` | Komga API base URL |
| `KOMGA_USER` | `admin@example.com` | Komga admin username |
| `KOMGA_PASSWORD` | `""` | Komga password |
| `PG_HOST` | `localhost` | PostgreSQL host |
| `PG_PORT` | `5432` | PostgreSQL port |
| `PG_DB` | `komga` | PostgreSQL database |
| `PG_USER` | `komga` | PostgreSQL user |
| `PG_PASSWORD` | `""` | PostgreSQL password |
| `PORT` | `5050` | Web server port |
| `EXPORT_DIR` | `/exports` | JSON + curl export directory |
| `DRY_RUN` | `false` | Preview only (no API calls) |
