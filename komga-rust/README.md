# Komga Rust

Rust implementation of Komga media server for comics/manga.

## Requirements

- Rust 1.70+
- PostgreSQL 14+
- (Optional) Redis for session/cache

## Setup

1. Copy `.env.example` to `.env` and configure:
   ```bash
   cp .env.example .env
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run database migrations:
   ```bash
   cargo run -- migrate
   ```

4. Start the server:
   ```bash
   cargo run --release
   ```

## API Documentation

OpenAPI 3.1.0 specification available at [`openapi.json`](./openapi.json) — documents all 162 API endpoints across 13 tags with 28 DTO schemas.

## Development

Run tests:
```bash
cargo test
```

Run with debug logging:
```bash
RUST_LOG=debug cargo run
```

Run integration tests (requires Docker):
```bash
cargo test --test integration_tests
```

## Changelog

### 2026-04-29 — DTO alignment with Java OpenAPI spec, OpenAPI export, missing API task list

**DTO alignment** — all 27 DTOs updated to match the Java Komga OpenAPI spec field-for-field:
- Added `#[serde(rename_all = "camelCase")]` to every DTO struct (JSON keys now match Java)
- **LibraryDto**: Removed `type`/`library_type` field; renamed `unavailable_date`→`unavailable` (bool); renamed field prefixes (`import_comicinfo_*`→`import_comic_info_*`); added `scanDirectoryExclusions`; removed extra timestamps to match Java
- **SeriesDto**: Added `booksInProgressCount`, `booksReadCount`, `booksUnreadCount`, `deleted`, timestamps; renamed `bookCount`→`booksCount`; added nested `BookMetadataAggregationDto` and `SeriesMetadataDto` refs
- **BookDto**: Added `seriesTitle`, `size` (human-readable), timestamps, `deleted`; renamed `fileSize`→`sizeBytes`; removed `fileHashKoreader`; added nested `MediaDto`, `BookMetadataDto`, `ReadProgressDto` refs
- **PageDto**: Added `size` (human-readable string)
- **ReadProgressDto**: Added `deviceId`, `deviceName`, timestamps
- **ReadListDto**: Replaced `bookCount` with `bookIds` array; added `filtered`, timestamps
- **CollectionDto**: Replaced `seriesCount` with `seriesIds` array; added `filtered`, timestamps
- **SeriesMetadataDto**: Added all 13 `*Lock` booleans + timestamps
- **BookMetadataDto**: Added all 9 `*Lock` booleans + timestamps
- **ApiKeyDto**: Renamed `name`→`comment`; added `userId`; renamed `lastUsedDate`→`lastModifiedDate`
- **Page DTOs** (Series/Book/ReadList/Collection/Task): Added `empty`, `first`, `last`, `numberOfElements` fields
- **New structs**: `MediaDto`, `BookMetadataAggregationDto`
- **UpdateReadListRequest**: Added `bookIds` field

**OpenAPI spec exported:**
- `openapi.json` — comprehensive OpenAPI 3.1.0 spec covering all 162 endpoints and 28 DTO schemas
- 78 of 130 Java paths covered (60% path parity)
- 100% field parity for all DTOs present in both specs

**Integration tests expanded** (50 total):
- Dockerized PostgreSQL via testcontainers per test function
- Tests: auth flow, CRUD for all entities, settings, search, API keys, Tachiyomi sync, thumbnails
- Runs in parallel (26s) or single-thread (90s)

**Missing API endpoint task list:**
- 28 tasks created tracking the 52 Java-only endpoints not yet in Rust
- Priority categories: thumbnail CRUD, progression, page hashes, series list endpoints, filesystem browsing, transient books, author metadata

**Test results:** 139 total — 56 unit/lib + 50 integration + 23 existing + 5 API + 3 coverage + 2 serialization — **all passing**.

### 2026-04-28 — API stub fill-in, route fixes, integration test suite

**Stubbed endpoint fill-in** — replaced 40+ empty/placeholder handlers with real implementations:
- **Search** — connected Tantivy search index with DB fallback (ILIKE)
- **Auth** — real JWT token generation/validation, bcrypt password hashing, login cookie/logout
- **User CRUD** — list, get, create, update, delete users + password management
- **Task triggers** — analyze/refresh/import/delete endpoints create real background tasks
- **Library** — PATCH (field update), empty-trash, settings persistence (SERVER_SETTINGS table)
- **Settings** — server + client settings persisted in DB
- **Referential data** — authors, genres, tags, publishers from DB
- **Page hashes** — real DB queries with pagination
- **Series/ReadList ZIP download** — full file bundling in-memory
- **Tachiyomi/Mihon sync** — read progress sync for series and readlists
- **Thumbnails** — readlist thumbnail repository + API handlers
- **ReadList navigation** — next/previous book from readlist order
- **Historical events, announcements, auth activity** — real DB queries
- **Transient books & fonts** — filesystem-based support
- **ReadList comicrack match** — creates real ReadList in DB
- **Release info** — reads version from Cargo.toml

**Bug fixes:**
- `{param}` → `:param` route syntax (Axum 0.7 uses `:` not `{}`, affected 73 route definitions across 7 files)
- `TIMESTAMP` → `TIMESTAMPTZ` in migration (sqlx `DateTime<Utc>` compatibility)
- Nullable `AGE_RESTRICTION_ALLOW_ONLY` → `Option<bool>` in User model
- Library `INSERT` column/value count mismatch (32 VALUES vs 31 columns)
- Missing `TASK` and `API_KEY` tables in migration
- `bcrypt::verify()` not checking the boolean return value (invalid passwords were accepted)
- Auth handlers using hardcoded `"admin@localhost"` → `find_all()` first user

**New repository modules (7 files):**
- `ServerSettingsRepository` — SERVER_SETTINGS key-value CRUD
- `ClientSettingsRepository` — CLIENT_SETTINGS_GLOBAL/USER CRUD
- `PageHashRepository` — page hash query + pagination
- `HistoricalEventRepository` — historical event + properties queries
- `ThumbnailRepository` — multi-table thumbnail CRUD (BOOK/SERIES/READLIST/COLLECTION)

**Integration test suite** — first-ever integration tests using testcontainers:
- `tests/common/mod.rs` — TestContext with dockerized PostgreSQL, auto-migration, random-port HTTP server
- 50 integration tests across 5 modules (auth, library, series, readlist, collection)
- Tests real HTTP endpoints against a fresh PostgreSQL container per test run
- Covers: auth flow, CRUD for all entity types, settings, search, API keys, Tachiyomi sync, thumbnails

**Test results:** 134 total tests — 56 unit/lib + 5 API + 50 integration + 23 existing — **all passing**.

### 2026-04-27 — Metadata enhancement, search infrastructure

- Series/book metadata PATCH endpoints
- ComicRack XML sidecar parsing
- Metadata aggregation from multiple sources
- Tantivy search index integration (lazy loading)
- Redis caching infrastructure
- Image processing (thumbnail generation, resize, format conversion)
- Mylar metadata support (series.json reader)
- Local artwork detection
- Aggregated series/book metadata DTOs

### 2026-04-26 — Core API scaffold, initial release

- Axum-based HTTP server with PgPool state
- JWT authentication (register, login, session)
- Library CRUD (create, read, update, delete)
- Series CRUD (basic listing by library)
- Book CRUD (basic listing by series)
- ReadList CRUD API
- Collection CRUD API
- Read progress tracking (per user/book)
- Task system (26 task types defined with worker)
- File parsing (CBZ/ZIP, EPUB, PDF extraction)
- 23 unit tests for models and infrastructure