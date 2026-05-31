# Rust Hash Cache Integration for Komga Scanner

## Problem

Komga's built-in `scanRootFolder` walks the entire filesystem tree using Java's `FileVisitor` (single-threaded), then fans out to individual per-book hashing tasks via the Kotlin `Hasher`. This is slow for large libraries (50-100s for 10K books). Additionally, the exists a standalone Python scan tool (`scan-library-tool/`) that uses a Rust binary (`pdf-hasher`) to compute XXH3_128 hashes ~50x faster via parallel mmap, but the results are never shared with Komga's native scan.

## Architecture

### Task Chain

```
User clicks "Scan" (or periodic trigger)
  │
  └── TaskEmitter.hashLibrary(libraryId)  [NEW]
        │
        └── TaskHandler handles Task.HashLibrary  [NEW]
              │
              ├── Spawn: pdf-hasher --root <root> --cache <cacheFile> -j 4
              │     ├── Rust walks filesystem (parallel, mmap, ~10s for 10K files)
              │     ├── Checks each .pdf against existing cache
              │     │   ├── size+mtime match → skip
              │     │   └── new/changed → compute XXH3_128
              │     └── Writes hashes-{libraryId}.json (atomic)
              │
              └── (on success or failure) fans out:
                    └── TaskEmitter.scanLibrary(libraryId)
                          │
                          └── scanRootFolder(library)
                                ├── Try load hash cache
                                ├── If cache exists → scanWithCache()  [NEW]
                                └── If no cache → scanWithFilesystemWalk()  [fallback]
```

### Dual-Path Design

```
scanRootFolder(library)
  │
  ├── hashCacheLoader.load(library.id) ≠ null?
  │     ├── YES → scanWithCache(library, hashCache)
  │     │           ├── export_db_snapshot → flat JSON from DB
  │     │           ├── load_hash_cache → Rust JSON
  │     │           ├── diff              → ScanDiffer
  │     │           ├── delete_missing    → soft delete series/books
  │     │           ├── create_new        → new series + books
  │     │           ├── update_changed    → media OUTDATED
  │     │           ├── sort_and_refresh
  │     │           └── sidecar_reconcile
  │     │
  │     └── NO → scanWithFilesystemWalk()  [original code, unchanged]
  │                 ├── filesystem_scan (FileVisitor)
  │                 ├── load_existing (DB queries)
  │                 ├── delete_missing
  │                 ├── reconcile_series_books
  │                 ├── sort_and_refresh
  │                 ├── sidecar_reconcile
  │                 └── cleanup
```

## File Structure

### New Files (4)

| File | Package | Purpose |
|------|---------|---------|
| `domain/model/HashCache.kt` | `domain.model` | Data classes: `HashCache`, `HashCacheEntry`, `DbSnapshot`, `DbBookEntry`, `DbSeriesEntry`, `ScanDiff`, `ChangedBookEntry` |
| `domain/service/HashCacheLoader.kt` | `domain.service` | `@Component` — loads Rust hash cache JSON from disk |
| `domain/service/DbSnapshotExporter.kt` | `domain.service` | `@Component` — queries DB for all series+books in a library, exports as flat `DbSnapshot` |
| `domain/service/ScanDiffer.kt` | `domain.service` | `@Component` — compares `HashCache` entries vs `DbSnapshot` entries → produces `ScanDiff` |

### Modified Files (6)

| File | Change |
|------|--------|
| `application/tasks/Task.kt` | Added `HashLibrary(libraryId)` sealed class |
| `application/tasks/TaskEmitter.kt` | Added `hashLibrary(libraryId, priority)` method |
| `application/tasks/TaskHandler.kt` | Handles `Task.HashLibrary` — runs `pdf-hasher` subprocess, fans out to `ScanLibrary` |
| `infrastructure/configuration/KomgaProperties.kt` | Added `hasherThreads: Int = 4` config property |
| `interfaces/api/rest/LibraryController.kt` | Scan endpoint now emits `Task.HashLibrary` instead of `Task.ScanLibrary` |
| `domain/service/LibraryContentLifecycle.kt` | `scanRootFolder` now dispatches to `scanWithCache` or `scanWithFilesystemWalk` |
| `domain/service/ScanRootFolderMetrics.kt` | Added `cacheHits`, `cacheMisses`, `cacheLoadMs` |

## Cache Format

### Hash Cache (`hashes-{libraryId}.json`)

Produced by `pdf-hasher` Rust binary. Location: `{configDir}/hash-cache/hashes-{libraryId}.json`.

#### Rust Output JSON
```json
{
  "type": "XXH3_128 hash cache",
  "version": 1,
  "root": "/path/to/library",
  "generated_at_unix_secs": 1717050000,
  "elapsed_secs": 1.5,
  "total_files": 1000,
  "total_bytes": 500000000,
  "entries": {
    "file:///data/SeriesA/Book1.pdf": {
      "hash": "a1b2c3d4...",
      "size": 12345678,
      "mtime_secs": 1717050000
    }
  }
}
```

#### Kotlin DTO (`HashCache.kt`) Field Mapping

| Rust serde field | JSON key | Kotlin property | Notes |
|---|---|---|---|
| `cache_type` `#[serde(rename = "type")]` | `"type"` | `type: String?` | |
| `version` | `"version"` | `version: Int?` | |
| `root` | `"root"` | `root: String?` | |
| `generated_at_unix_secs` | `"generated_at_unix_secs"` | *ignored* | `@JsonIgnoreProperties(ignoreUnknown = true)` |
| `elapsed_secs` | `"elapsed_secs"` | *ignored* | `@JsonIgnoreProperties(ignoreUnknown = true)` |
| `total_files` | `"total_files"` | `@JsonProperty("total_files") totalFiles: Int?` | snake_case → camelCase |
| `total_bytes` | `"total_bytes"` | `@JsonProperty("total_bytes") totalBytes: Long?` | snake_case → camelCase |
| `entries` | `"entries"` | `entries: Map<String, HashCacheEntry>` | |

**`HashCacheEntry` field mapping:**

| Rust field | JSON key | Kotlin property | Notes |
|---|---|---|---|
| `hash` | `"hash"` | `hash: String` | |
| `size` | `"size"` | `size: Long` | |
| `mtime_secs` | `"mtime_secs"` | `@JsonProperty("mtime_secs") mtimeSecs: Long` | snake_case → camelCase |

**File URI format**: Rust produces `file://{path}` (e.g. `file:///data/SeriesA/Book1.pdf`). The Kotlin side reads via `URI(key)` which correctly parses standard file URIs.

### Cache Invalidation

A cache entry is valid only if **both** `size` AND `mtime_secs` match the current file. If either differs, the entry is stale and the file must be re-hashed (by the Rust tool on next run) or hashed later by the Kotlin `Hasher` in a background task.

## Key Design Decisions

### 1. Rust hasher runs FIRST as a separate Task

Instead of integrating the hasher into `scanRootFolder`, a new `Task.HashLibrary` precedes the scan. This allows:
- The hasher to take as long as needed (600s timeout) without blocking the scan
- The scan to always find a (potentially fresh) cache file
- Graceful degradation: if the hasher fails or times out, scan proceeds

### 2. scanRootFolder checks for cache at start

The entry point `scanRootFolder()` tries `hashCacheLoader.load(library.id)` before doing anything else:
- If cache found → fast `scanWithCache()` path
- If no cache → original `scanWithFilesystemWalk()` path (unchanged)

This makes the feature **additive and non-breaking**.

### 3. Comparison is set-based O(n)

`ScanDiffer` compares two HashMaps (hash cache URIs vs DB snapshot URIs):
- Set difference for new/deleted
- Intersection with comparison for changed
- No per-series DB lookups during scan

### 4. Graceful degradation

| Scenario | Behavior |
|----------|----------|
| `pdf-hasher` binary not found | Log error, skip, proceed to filesystem scan |
| Hasher times out (10 min) | Kill, log, proceed to filesystem scan |
| Cache file missing | Log info, proceed to filesystem scan |
| Cache JSON corrupted | Log warning, proceed to filesystem scan |
| File changed since cache | Entry ignored, later hashed by `hashBooksWithoutHash` |

## Configuration

```yaml
# application.yml
komga:
  hasher-threads: 4          # KOMGA_HASHER_THREADS
```

Cache directory is derived from `komga.config-dir` (set via `KOMGA_CONFIGDIR` env var):
- Cache path: `{configDir}/hash-cache/hashes-{libraryId}.json`
- Default for Docker: `/config/hash-cache/hashes-{libraryId}.json`

## Performance

| Metric | Before | After (no cache) | After (with cache) |
|--------|--------|------------------|--------------------|
| Filesystem walk | 30-60s (Java, single-threaded) | 30-60s (same) | 5-10s (Rust, parallel mmap) |
| Hashing | Per-book ~50ms each | Same | Included in walk above |
| DB comparison | Per-series queries | Same | Set-based O(n) ~1-2s |
| **Total (10K books)** | ~50-100s | Same | ~20-40s |

## Setup

### Docker Build

Add to `komga/docker/Dockerfile.tpl`:

```dockerfile
# Rust build stage
FROM rust:1.85-slim AS rust-builder
WORKDIR /build
COPY scan-library-tool/pdf-hasher/ .
RUN apt-get update && apt-get install -y pkg-config && \
    rm -rf /var/lib/apt/lists/* && cargo build --release

# In runner stage:
COPY --from=rust-builder /build/target/release/pdf-hasher /usr/local/bin/
```

The build context must include `scan-library-tool/pdf-hasher/`. This is typically done by ensuring the Docker build runs from the project root.

### Generating the Hash Cache

Once the binary is installed, the hash cache is generated automatically before each scan. No manual steps needed. The Rust tool is incremental: on subsequent runs, only new/changed files are re-hashed.

To manually generate or inspect the cache:
```bash
pdf-hasher --root /data --cache /config/hash-cache/hashes-<libraryId>.json -j 4
```
