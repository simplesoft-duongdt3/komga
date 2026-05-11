# ScanLibrary Task — Performance Analytics Guide

> **Purpose:** Provide a single, actionable document for analysing the performance of every step in a `ScanLibrary` task, identifying bottlenecks, and monitoring ongoing performance.
>
> **Audience:** Komga operators and developers debugging slow scans or planning capacity.
>
> **Last updated:** 2026-05-11

---

## Table of Contents

1. [Anatomy of a ScanLibrary Run](#1-anatomy-of-a-scanlibrary-run)
2. [Measurement Points — What to Log & How](#2-measurement-points--what-to-log--how)
3. [Phase-by-Phase Breakdown](#3-phase-by-phase-breakdown)
4. [Post-Scan Fan-out (Phase 2) Analytics](#4-post-scan-fan-out-phase-2-analytics)
5. [System-Level Metrics to Collect](#5-system-level-metrics-to-collect)
6. [Bottleneck Diagnosis Flowchart](#6-bottleneck-diagnosis-flowchart)
7. [Dashboard / Monitoring Recommendations](#7-dashboard--monitoring-recommendations)
8. [Existing Instrumentation Summary](#8-existing-instrumentation-summary)
9. [Appendix: Key Source Files](#9-appendix-key-source-files)

---

## 1. Anatomy of a ScanLibrary Run

A `ScanLibrary` task executes in **two phases**:

```
┌─ Phase 1 (synchronous, inside the ScanLibrary task) ─────────────────────────────┐
│                                                                                   │
│  scanRootFolder()                                                                 │
│   ├── 1. Filesystem Scan        (FileSystemScanner)                               │
│   ├── 2. Clear Unavailable      (DB write — clear unreachable flag)               │
│   ├── 3. Load Existing State    (DB reads — all series + books for library)       │
│   ├── 4. Delete Missing Series  (DB writes — soft delete vanished series)         │
│   ├── 5. Delete Missing Books   (DB writes — soft delete vanished books)          │
│   ├── 6. Reconcile Series & Books  (THE EXPENSIVE PART)                           │
│   │    ├── New series: create + addBooks + tryRestore                             │
│   │    ├── Changed series: update + match books; emit VerifyBookHash or OUTDATED  │
│   │    └── New books: addBooks + tryRestore                                       │
│   ├── 7. Sort & Refresh Series   (sort books + emit RefreshSeriesMetadata tasks)   │
│   ├── 8. Reconcile Sidecars      (compare file mtimes, emit refresh tasks)         │
│   ├── 9. Cleanup Sidecars        (delete stale sidecar records)                   │
│   └── 10. Cleanup                (emptyTrash or deleteEmptySets)                  │
│                                                                                   │
└───────────────────────────────────────────────────────────────────────────────────┘

┌─ Phase 2 (still inside ScanLibrary — inline fan-out queries and task emission) ──┐
│                                                                                   │
│  TaskHandler calls (inline, blocking ScanLibrary completion):                     │
│   ├── analyzeUnknownAndOutdatedBooks()     → N× AnalyzeBook tasks                 │
│   ├── repairExtensions()                   → M× RepairExtension tasks     [opt]  │
│   ├── findBooksToConvert()                 → 1× FindBooksToConvert task   [opt]  │
│   ├── findBooksWithMissingPageHash()       → 1× FindBooksWithMissingPageHash [opt]│
│   ├── findDuplicatePagesToDelete()         → 1× FindDuplicatePagesToDelete       │
│   ├── hashBooksWithoutHash()               → H× HashBook tasks           [opt]  │
│   └── hashBooksWithoutHashKoreader()       → K× HashBookKoreader tasks   [opt]  │
│                                                                                   │
└───────────────────────────────────────────────────────────────────────────────────┘

┌─ Post-Scan (fan-out tasks execute in TaskProcessor threads) ─────────────────────┐
│                                                                                   │
│  Each AnalyzeBook may emit: GenerateBookThumbnail, RefreshBookMetadata            │
│  Each RefreshBookMetadata → RefreshSeriesMetadata → AggregateSeriesMetadata       │
│  Each FindBooksToConvert → N× ConvertBook                                         │
│  Each FindBooksWithMissingPageHash → N× HashBookPages                             │
│  Each FindDuplicatePagesToDelete → N× RemoveHashedPages                           │
│                                                                                   │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Measurement Points — What to Log & How

### 2.1 Already Instrumented (no code changes needed)

The following measurements already exist in `LibraryContentLifecycle.scanRootFolder()` and `TaskHandler.handleTask()`:

**Phase timings (logged per phase as `scanRootFolder phase=...`):**

| Logged Name | What It Measures | Line in LCL.kt |
|---|---|---|
| `filesystem_scan` | `Files.walkFileTree()` duration | ~94-117 |
| `clear_unavailable` | DB write to clear unreachable flag | ~122-136 |
| `load_existing` | DB reads for all series + books | ~145-179 |
| `delete_missing_series` | DB soft-deletes for vanished series | ~185-206 |
| `delete_missing_books` | DB soft-deletes for vanished books | ~209-232 |
| `reconcile_series_books` | Create/update/restore series+books | ~239-341 |
| `sort_and_refresh_series` | Sort books + enqueue metadata refresh | ~343-355 |
| `reconcile_sidecars` | Sidecar file mtime comparison | ~357-400 |
| `cleanup_sidecars` | Remove stale sidecar records | ~402-416 |
| `cleanup` / `empty_trash` | Final cleanup | ~418-427 |

**Aggregate summary (logged once at end as `scanRootFolder completed ...`):**

| Counter | Meaning |
|---|---|
| `scannedSeries` | Series directories found on disk |
| `scannedBooks` | Book files found on disk |
| `scannedSidecars` | Sidecar files found on disk |
| `existingSeries` | Series in DB before scan |
| `existingScannedSeries` | Series in DB that match scanned paths |
| `preloadedBooks` | Books preloaded from DB |
| `deletedSeries` | Series soft-deleted (no longer on disk) |
| `deletedBooks` | Books soft-deleted (no longer on disk) |
| `createdSeries` | New series created |
| `updatedSeries` | Existing series updated (mtime changed) |
| `addedBooks` | New books added |
| `deferredHashBooks` | Books with same size + existing hash → async VerifyBookHash |
| `outdatedBooks` | Books marked OUTDATED (different file size or no hash) |
| `seriesRefreshQueued` | Series metadata refresh tasks emitted |
| `changedSidecars` | Sidecars with changed mtime |
| `deletedSidecars` | Stale sidecar records cleaned up |

**Task-level timing (Micrometer + TaskExecution table):**

| Metric | Type | Tags | Location |
|---|---|---|---|
| `komga.tasks.execution` | Timer | `type=ScanLibrary` | TaskHandler.kt:205 |
| `komga.tasks.failure` | Counter | `type=ScanLibrary` | TaskHandler.kt:220 |
| `TaskExecution` table | Row per execution | startDate, endDate, durationMillis, success | TaskHandler.kt:249 |

### 2.2 Partially Instrumented — Needs Additional Monitoring

These steps happen but have **no dedicated timing or counter metrics**:

| Step | Location | What's Missing |
|---|---|---|
| `analyzeUnknownAndOutdatedBooks` query | TaskHandler.kt:68 | Elapsed time of the search query; count of results |
| `hashBooksWithoutHash` query | TaskHandler.kt:73 | Elapsed time of query; count of books returned |
| `hashBooksWithoutHashKoreader` query | TaskHandler.kt:74 | Elapsed time of query; count of books returned |
| `repairExtensions` query | TaskHandler.kt:69 | Elapsed time; count of mismatched books |
| `findBooksToConvert` emission | TaskHandler.kt:70 | Elapsed time of query in FindBooksToConvert handler |
| `findBooksWithMissingPageHash` emission | TaskHandler.kt:71 | Elapsed time of query in FindBooksWithMissingPageHash handler |
| `findDuplicatePagesToDelete` emission | TaskHandler.kt:72 | Elapsed time of query in FindDuplicatePagesToDelete handler |
| Queue wait time per fan-out task | TaskProcessor.kt | Time between task `save()` and `takeFirst()` |
| Total task queue drain time | N/A | Wall-clock from ScanLibrary finish to last fan-out task completion |

### 2.3 Suggested New Instrumentation (for advanced monitoring)

For a complete picture, consider adding:

```kotlin
// In TaskHandler.kt, around each fan-out call in ScanLibrary handling:
logger.info {
  "fanoutQuery name=analyzeUnknownAndOutdatedBooks " +
  "libraryId=${library.id} count=$count durationMs=${duration.inWholeMilliseconds}"
}
```

**Recommended new log lines for Phase 2:**

```text
fanoutQuery name=analyzeUnknownAndOutdatedBooks libraryId=xxx count=15000 durationMs=23400
fanoutQuery name=hashBooksWithoutHash libraryId=xxx count=50000 durationMs=1200
fanoutQuery name=hashBooksWithoutHashKoreader libraryId=xxx count=0 durationMs=800
fanoutQuery name=repairExtensions libraryId=xxx count=0 durationMs=500
```

**Recommended new metrics counters:**

| Metric | Type | Tags |
|---|---|---|
| `komga.scan.fanout.query.time` | Timer | `phase=analyzeUnknownAndOutdatedBooks` |
| `komga.scan.fanout.books` | Counter | `phase=analyzeUnknownAndOutdatedBooks` |
| `komga.scan.fanout.books` | Counter | `phase=hashBooksWithoutHash` |
| `komga.scan.filesystem.files` | Counter | — |
| `komga.scan.filesystem.bytes` | Counter | — |

---

## 3. Phase-by-Phase Breakdown

### 3.1 Phase 1.1 — Filesystem Scan

**Code:** `FileSystemScanner.scanRootFolder()` (FileSystemScanner.kt:48-210)

**Mechanism:** Single-threaded `Files.walkFileTree()` with `FOLLOW_LINKS` and `Integer.MAX_VALUE` depth.

**Logged:** `scanRootFolder phase=filesystem_scan status=ok scanId=xxx libraryId=yyy durationMs=N series=N books=N sidecars=N`

**Key signals of a bottleneck:**

| Signal | What It Indicates |
|---|---|
| `filesystemScanMs` > 50% of `totalMs` | Filesystem walk is the dominant bottleneck |
| `durationMs` grows linearly with file count | Expected; no pathological behavior |
| `durationMs` is high for a small file count | Network filesystem (NFS/SMB) latency |
| `FOLLOW_LINKS` with circular symlinks | Exponential traversal (rare, walkFileTree has cycle detection) |

**Expected baseline times (reference):**

| Storage | Files | Expected Time |
|---|---|---|
| Local SSD | 10,000 | ~0.5–3s |
| Local SSD | 100,000 | ~5–30s |
| Local HDD | 100,000 | ~30–120s |
| NFS (1ms latency) | 100,000 | ~2–10 min |
| NFS/SMB (high latency, 10-50ms) | 100,000 | ~15 min – 1h+ |
| NFS/SMB (very high latency, 100ms) | 100,000 | ~3h+ |

**What you can measure externally (OS-level):**

```bash
# On Linux — count filesystem calls during scan
sudo strace -p $(pgrep -f komga) -e trace=stat,newfstatat,lstat 2>&1 | wc -l

# On macOS — filesystem usage per process
sudo fs_usage -w -f filesys komga 2>&1 | head -100

# Generic — measure directory walk time
time find /path/to/library -type f | wc -l
```

### 3.2 Phase 1.2 — Clear Unavailable

**Code:** `LibraryContentLifecycle.scanRootFolder()` (~line 122-136)

**Mechanism:** If the library was previously marked as unreachable, clears the flag.

**Expected:** Always very fast (< 100ms). If slow, check DB connection pool or write contention.

### 3.3 Phase 1.3 — Load Existing State

**Code:** `LibraryContentLifecycle.scanRootFolder()` (~line 145-179)

**Mechanism:**
- `seriesRepository.findAllByLibraryId(library.id)` — loads ALL series for this library into memory
- `bookRepository.findAllBySeriesIds(existingScannedSeriesIds)` — loads ALL books for matched series

**Logged:** `scanRootFolder phase=load_existing status=ok scanId=xxx libraryId=yyy durationMs=N existingSeries=N existingScannedSeries=N preloadedBooks=N`

**Key signals:**

| Signal | What It Indicates |
|---|---|
| High `durationMs` with high `existingSeries` + `preloadedBooks` | Expected; scales with library size |
| `durationMs` much higher than expected | DB query plan issues; missing indexes; connection pool contention |
| OOM or high heap usage during this phase | Too many books loaded into memory; library is too large for available heap |

**Memory estimation:**

```
Memory ≈ (existingSeries × ~500 bytes) + (preloadedBooks × ~300 bytes)
Example: 10,000 series + 100,000 books ≈ 35 MB (acceptable)
Example: 50,000 series + 500,000 books ≈ 175 MB (check heap settings)
```

### 3.4 Phase 1.4–1.5 — Delete Missing Series & Books

**Code:** `LibraryContentLifecycle.scanRootFolder()` (~line 185-232)

**Mechanism:** Compares scanned paths against existing records. Any series/book found in DB but not on disk is soft-deleted. Uses batched operations.

**Logged:**
- `scanRootFolder phase=delete_missing_series status=ok scanId=xxx libraryId=yyy durationMs=N deletedSeries=N`
- `scanRootFolder phase=delete_missing_books status=ok scanId=xxx libraryId=yyy durationMs=N deletedBooks=N`

**Key signals:**

| Signal | What It Indicates |
|---|---|
| `deletedSeries` or `deletedBooks` is very large | Bulk delete; library path may have changed or storage was temporarily unavailable |
| `durationMs` high but `deletedSeries` low | DB write contention or slow FK cascade on soft-delete |

### 3.5 Phase 1.6 — Reconcile Series & Books (THE BOTTLENECK)

**Code:** `LibraryContentLifecycle.scanRootFolder()` (~line 239-341)

**Mechanism:** Iterates over every scanned series and:
- **New series:** `createSeries()` → INSERT into series, metadata, aggregation tables → `addBooks()` → INSERT books, media, metadata → `tryRestoreSeries()` → queries deleted series, computes file hashes → `tryRestoreBooks()` → queries deleted books by file size, computes file hash
- **Changed series:** Update series record → compare existing books by URL using HashMap (O(1) per book) → for changed books: if same file size + existing hash → emit `VerifyBookHash` (deferred); if different file size or no hash → reset `fileHash=""`, mark `Media.Status.OUTDATED` → detect and add new books → `tryRestore` for new books
- **Unchanged series:** Skip (no changes detected)
- **After reconciliation:** `sortBooks()` + emit `RefreshSeriesMetadata` for changed series

**Logged:** `scanRootFolder phase=reconcile_series_books status=ok scanId=xxx libraryId=yyy durationMs=N createdSeries=N updatedSeries=N addedBooks=N deferredHashBooks=N outdatedBooks=N seriesRefreshQueued=N`

**Performance characteristics of sub-operations (not individually logged):**

| Sub-operation | When It Runs | Cost (per call) |
|---|---|---|
| `seriesLifecycle.createSeries()` | Per new series | ~50-200ms (3× INSERTs: series, metadata, aggregation) |
| `seriesLifecycle.addBooks()` | Per new/updated series | ~10-100ms per book (INSERT book + media + metadata) |
| `tryRestoreSeries()` | Per new series | Queries ALL deleted series + hashes each new book file |
| `tryRestoreBooks()` | Per new book | Queries deleted books by fileSize (N+1 pattern) + hashes file |
| `hasher.computeHash()` | In tryRestore | Reads ENTIRE file (0.5-2s/file for 500MB comic) |
| `seriesLifecycle.sortBooks()` | Per changed series | Sorts book records, updates DB |

**Key bottleneck signals:**

| Signal | What It Indicates |
|---|---|
| High `createdSeries` with high `reconcileSeriesBooksMs` | tryRestore is computing file hashes for many new books |
| High `addedBooks` but `seriesRefreshQueued` is low | Series may not have been marked as changed (check update logic) |
| High `outdatedBooks` | Many files changed size; these will need AnalyzeBook |
| High `deferredHashBooks` | Many files changed mtime but not size; VerifyBookHash will still run async |
| `reconcileSeriesBooksMs` >> 50% of totalMs | Reconciliation is the dominant bottleneck |
| Memory grows during reconcile | Book objects held in memory during iteration |

**tryRestore cost analysis:**

```
Total tryRestore cost = Σ (seriesRestoreCost + Σ bookRestoreCost per new book)

seriesRestoreCost per series:
  - 1× query: findAll(Deleted(IsTrue)) for ALL deleted series in library
  - 1× computeHash per new book file

bookRestoreCost per new book:
  - 1× query: findAllDeletedByFileSize(fileSize)
  - computeHash (if restoration match found)
```

**Optimization impact of existing improvements:**

| Improvement | Before | After |
|---|---|---|
| Series/URL comparison | O(n²) HashMap lookup per series | O(1) HashMap lookup per book |
| Series+books loading | N+1 queries (1 per series) | 2 queries (batch preload) |
| Sidecar parent lookup | N+1 queries (1 per sidecar) | Preloaded HashMap lookup |
| Hash verification for same-size files | Synchronous computeHash | Async VerifyBookHash task |

### 3.6 Phase 1.7 — Sort & Refresh Series

**Code:** `LibraryContentLifecycle.scanRootFolder()` (~line 343-355)

**Mechanism:** For each changed series, calls `sortBooks()` and emits `RefreshSeriesMetadata`.

**Logged:** `scanRootFolder phase=sort_and_refresh_series status=ok scanId=xxx libraryId=yyy durationMs=N seriesRefreshQueued=N`

**Key insight:** `RefreshSeriesMetadata` tasks are enqueued here. They will be picked up by the TaskProcessor when pool threads are available. They do NOT block ScanLibrary.

### 3.7 Phase 1.8 — Reconcile Sidecars

**Code:** `LibraryContentLifecycle.scanRootFolder()` (~line 357-400)

**Mechanism:** Compares each scanned sidecar's `lastModifiedTime` with stored value. If changed, emits `RefreshSeriesLocalArtwork` or `RefreshSeriesMetadata` or `RefreshBookLocalArtwork` or `RefreshBookMetadata`.

**Logged:** `scanRootFolder phase=reconcile_sidecars status=ok scanId=xxx libraryId=yyy durationMs=N changedSidecars=N`

**Key signals:**

| Signal | What It Indicates |
|---|---|
| High `changedSidecars` | Many artwork/metadata sidecars were updated on disk |
| `durationMs` high even with low `changedSidecars` | Parent lookup overhead; check if sidecar preloading is effective |

### 3.8 Phase 1.9–1.10 — Cleanup Sidecars & Cleanup

**Logged:**
- `scanRootFolder phase=cleanup_sidecars status=ok scanId=xxx libraryId=yyy durationMs=N deletedSidecars=N`
- `scanRootFolder phase=cleanup status=ok scanId=xxx libraryId=yyy durationMs=N`
- (or `scanRootFolder phase=empty_trash status=ok ...`)

**Expected:** Fast (< 1s) unless there are many stale sidecar records to delete.

---

## 4. Post-Scan Fan-out (Phase 2) Analytics

Phase 2 runs inline inside the `ScanLibrary` handler after `scanRootFolder()` returns. It is **NOT** currently instrumented with per-step timing logs. The following table shows what can be measured with the current code vs. what needs additional instrumentation.

### 4.1 Current Visibility

| Fan-out Step | Log Level | What You See | Missing |
|---|---|---|---|
| `analyzeUnknownAndOutdatedBooks` | INFO | "Sending {N} tasks, sampleType=AnalyzeBook" | Query duration; count of books |
| `hashBooksWithoutHash` | INFO | "Sending {N} tasks, sampleType=HashBook" | Query duration; count |
| `hashBooksWithoutHashKoreader` | INFO | Same pattern | Query duration; count |
| `repairExtensions` | INFO | Same pattern | Query duration; count |
| `findBooksToConvert` | INFO | "Sending task: FindBooksToConvert" | Only 1 task, no count |
| `findBooksWithMissingPageHash` | INFO | Same pattern | Only 1 task, no count |
| `findDuplicatePagesToDelete` | INFO | Same pattern | Only 1 task, no count |

### 4.2 Task Queue Analytics (Post-Scan)

After `ScanLibrary` finishes, the real performance impact shifts to the TaskProcessor draining the fan-out tasks. To analyze this:

**Check the task queue API:**

```bash
# How many tasks are queued per type?
GET /api/v1/tasks
# Look at simpleType distribution

# How many tasks are actively running?
# Running tasks have owner != null

# How deep is the queue?
# Queued tasks have owner == null
```

**Queue drain time estimation:**

```
Total drain time ≈ Σ (task count per type × avg execution time per task)
                    ÷ min(corePoolSize, available DB connections)

With poolSize = 1:
  drainTime = sum(taskCount × avgTime)

With poolSize = 4:
  drainTime ≈ sum(taskCount × avgTime) / 4
              (if no groupId serialization conflicts)

Example (100K fan-out tasks, 1s avg, poolSize=1):
  drainTime ≈ 100,000s ≈ 28 hours

Example (100K fan-out tasks, 1s avg, poolSize=4):
  drainTime ≈ 100,000s / 4 ≈ 7 hours
```

**Group serialization impact:**

Tasks with `groupId = seriesId` (AnalyzeBook, ConvertBook, RepairExtension, RefreshBookMetadata, etc.) serialize per series:
```
seriesA: AnalyzeBook → RefreshBookMetadata → RefreshSeriesMetadata
seriesB: AnalyzeBook → RefreshBookMetadata → RefreshSeriesMetadata

With poolSize=2:
  Thread 1: AnalyzeBook[seriesA] → RefreshBookMetadata[seriesA] → ...
  Thread 2: AnalyzeBook[seriesB] → RefreshBookMetadata[seriesB] → ...
```

### 4.3 Task Execution History

The `TaskExecution` table records every task execution. Query to analyse bottlenecks:

```sql
-- Top 10 longest individual tasks from a specific library scan
SELECT simple_type, library_id, series_id, book_id, duration_ms
FROM task_execution
WHERE library_id = 'your-library-id'
  AND start_date >= NOW() - INTERVAL '24 hours'
ORDER BY duration_ms DESC
LIMIT 10;

-- Average duration by task type (for queue drain estimation)
SELECT simple_type,
       COUNT(*) AS count,
       AVG(duration_ms) AS avg_ms,
       PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_ms
FROM task_execution
WHERE start_date >= NOW() - INTERVAL '7 days'
GROUP BY simple_type
ORDER BY avg_ms DESC;

-- Task execution volume by hour
SELECT DATE_TRUNC('hour', start_date) AS hour,
       simple_type,
       COUNT(*) AS count
FROM task_execution
WHERE start_date >= NOW() - INTERVAL '24 hours'
GROUP BY hour, simple_type
ORDER BY hour;
```

### 4.4 Chain Reaction Analysis

Each fan-out task can itself emit more tasks. Understanding the full chain is critical:

```
ScanLibrary
  ├── RepairExtension → (none — terminal)
  ├── FindBooksToConvert → ConvertBook × N → (none — terminal)
  ├── FindBooksWithMissingPageHash → HashBookPages × N → (none — terminal)
  ├── FindDuplicatePagesToDelete → RemoveHashedPages × N → may GenerateBookThumbnail
  ├── HashBook → (none — terminal)
  ├── HashBookKoreader → (none — terminal)
  └── AnalyzeBook × N
        ├── GenerateBookThumbnail × M   [if action contains GENERATE_THUMBNAIL]
        │     └── (currently DISABLED — TODO no-op)
        └── RefreshBookMetadata × K     [if action contains REFRESH_METADATA]
              └── RefreshSeriesMetadata × K
                    └── AggregateSeriesMetadata × K
                          └── (none — terminal)
```

**When analysing total pipeline time, sum ALL of these tasks.**

The `TaskExecution` table makes this visible — just query by library and time window.

---

## 5. System-Level Metrics to Collect

For a complete performance picture, collect system-level metrics during a scan:

### 5.1 Database

```sql
-- Connection pool usage
SELECT count(*) AS active_connections FROM pg_stat_activity
WHERE state = 'active' AND application_name LIKE '%komga%';

-- Query performance during scan
SELECT query, calls, total_exec_time / calls AS avg_ms,
       rows, shared_blks_hit, shared_blks_read
FROM pg_stat_statements
WHERE query LIKE '%book%' OR query LIKE '%series%' OR query LIKE '%media%'
ORDER BY total_exec_time DESC
LIMIT 20;

-- Lock contention
SELECT blocked_locks.pid AS blocked_pid,
       blocking_locks.pid AS blocking_pid
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_stat_activity blocked_activity ON blocked_activity.pid = blocked_locks.pid
JOIN pg_catalog.pg_locks blocking_locks ON blocking_locks.locktype = blocked_locks.locktype
WHERE NOT blocked_locks.granted;
```

### 5.2 JVM

```bash
# Heap usage during scan
jstat -gc <komga_pid> 5s

# Thread dump during a suspected hang
jstack <komga_pid> > threaddump_scanning.txt

# Full GC analysis
jstat -gcutil <komga_pid> 5s
```

### 5.3 Filesystem / OS

```bash
# Linux: I/O stats during scan
iostat -x 5

# Linux: filesystem latency
# If using NFS:
nfsiostat 5

# macOS: filesystem usage per tool
sudo fs_usage -w -f filesys komga

# Network filesystem packet tracing (NFS)
# tcpdump -i any port 2049
```

### 5.4 Task Queue (Application)

```bash
# Active tasks (owning a worker)
curl -s http://localhost:8080/api/v1/tasks | jq '[.content[] | select(.owner != null)] | length'

# Queued tasks (waiting for a worker)
curl -s http://localhost:8080/api/v1/tasks | jq '[.content[] | select(.owner == null)] | length'

# Top task types in queue
curl -s http://localhost:8080/api/v1/tasks | jq '.content | group_by(.simpleType) | map({type: .[0].simpleType, count: length}) | sort_by(-.count)'
```

---

## 6. Bottleneck Diagnosis Flowchart

```
START: ScanLibrary running for 30h
│
├─ Is filesystemScanMs > 50% of totalMs?
│   ├─ YES → Filesystem walk is the bottleneck
│   │   ├─ Is storage on NFS/SMB?
│   │   │   ├─ YES → Optimise network filesystem (see section 3.1)
│   │   │   └─ NO  → Consider parallel filesystem walk (Phase 4 in perf-plan)
│   │   └─ Check: strace/fs_usage for syscall count
│   │
│   └─ NO  → Continue
│
├─ Is reconcileSeriesBooksMs > 50% of totalMs?
│   ├─ YES → Reconciliation is the bottleneck
│   │   ├─ Are createdSeries > 0?
│   │   │   ├─ YES → tryRestore is computing file hashes
│   │   │   │   ├─ Check hasher.computeHash() calls (read entire files)
│   │   │   │   └─ Consider disabling hashFiles if not needed
│   │   │   └─ NO  → Continue
│   │   │
│   │   ├─ Are addedBooks > 0?
│   │   │   ├─ YES → Book INSERT + tryRestore overhead
│   │   │   │   └─ Check tryRestoreBooks N+1 query pattern (findAllDeletedByFileSize)
│   │   │   └─ NO  → Continue
│   │   │
│   │   ├─ Is outdatedBooks high?
│   │   │   ├─ YES → Many files changed (size diff or no hash)
│   │   │   │   └─ Expected during first scan; normal after
│   │   │   └─ NO  → Continue
│   │   │
│   │   └─ Is deferredHashBooks high?
│   │       └─ YES → Many VerifyBookHash tasks queued (expected)
│   │
│   └─ NO  → Continue
│
├─ Is Phase 2 (fan-out queries) taking long?
│   │  (Check by diff: totalMs - sum of phase timestamps)
│   ├─ YES → 
│   │  ├─ analyzeUnknownAndOutdatedBooks query slow?
│   │  │   ├─ Check index on (library_id, media_status)
│   │  │   └─ Check media table join performance
│   │  ├─ hashBooksWithoutHash query returns many rows?
│   │  │   └─ Consider disabling hashFiles
│   │  └─ hashBooksWithoutHashKoreader query returns many rows?
│   │       └─ Consider disabling hashKoreader
│   │
│   └─ NO  → Continue
│
├─ Is the task queue drain taking most of the 30h?
│   │  (ScanLibrary finished early, but tasks keep running)
│   ├─ Is taskPoolSize = 1?
│   │   ├─ YES → All fan-out tasks run sequentially — INCREASE pool size!
│   │   └─ NO  → Continue
│   │
│   ├─ Check Queue size: /api/v1/tasks
│   │   ├─ Many HashBook tasks?
│   │   │   └─ Consider disabling hashFiles/hashKoreader
│   │   ├─ Many AnalyzeBook tasks?
│   │   │   └─ Normal after first scan; monitor declining trend
│   │   └─ Many RefreshSeriesMetadata tasks?
│   │       └─ Chain reaction from RefreshBookMetadata → RefreshSeriesMetadata
│   │
│   └─ Check groupId serialization bottlenecks
│       └─ tasks with same seriesId cannot run in parallel
│
└─ All phases look normal but total is still slow
    ├─ Check JVM heap / GC pauses
    ├─ Check DB connection pool exhaustion
    ├─ Check DB CPU / I/O
    └─ Check for lock contention (FOR UPDATE SKIP LOCKED)
```

---

## 7. Dashboard / Monitoring Recommendations

### 7.1 Summary Log Extraction (what you already have)

After each scan, grep the logs and save to a structured format:

```bash
# Extract all phase timings from a scan
grep 'scanRootFolder phase=' /path/to/komga.log

# Extract summary line
grep 'scanRootFolder completed' /path/to/komga.log

# Cross-reference with task execution table
# Query TaskExecution for ScanLibrary entries in the same time window
```

### 7.2 Suggested Structured Analytics Output

For each `ScanLibrary` run, produce a structured analytics record like this:

```json
{
  "scanId": "abc123",
  "libraryId": "lib-1",
  "scanDeep": false,
  "totalMs": 36000000,
  "fileSystemScan": {
    "durationMs": 3000000,
    "series": 5000,
    "books": 50000,
    "sidecars": 0,
    "pctOfTotal": 8.3
  },
  "loadExisting": {
    "durationMs": 2000,
    "pctOfTotal": 0.01
  },
  "reconcile": {
    "durationMs": 32000000,
    "createdSeries": 200,
    "updatedSeries": 300,
    "addedBooks": 2000,
    "deferredHashBooks": 500,
    "outdatedBooks": 500,
    "pctOfTotal": 88.9
  },
  "fanout": {
    "analyzeUnknownAndOutdatedBooks": {
      "booksEnqueued": 15000
    },
    "hashBooksWithoutHash": {
      "booksEnqueued": 50000
    },
    "hashBooksWithoutHashKoreader": {
      "booksEnqueued": 0
    }
  },
  "totalTaskQueueDrainMs": 100800000,
  "summary": {
    "status": "bottleneck_reconciliation",
    "primaryBottleneck": "tryRestore file hashing",
    "recommendation": "Disable hashFiles if not needed"
  }
}
```

This could be logged as a single JSON line at the end for programmatic parsing.

### 7.3 Suggested Grafana / Monitoring Queries

Using Micrometer metrics already exported:

```promql
# Average ScanLibrary duration (window)
rate(komga_tasks_execution_seconds_sum{type="ScanLibrary"}[1h])
/
rate(komga_tasks_execution_seconds_count{type="ScanLibrary"}[1h])

# P95 ScanLibrary duration
histogram_quantile(0.95,
  sum(rate(komga_tasks_execution_seconds_bucket{type="ScanLibrary"}[1h])) by (le)
)

# Failure rate
rate(komga_tasks_failure_total{type="ScanLibrary"}[1h])
```

### 7.4 Alert Thresholds

| Condition | Alert Level | Action |
|---|---|---|
| ScanLibrary execution > 1h | WARN | Check logs; likely network filesystem |
| ScanLibrary execution > 6h | CRITICAL | Investigate bottleneck; check pool size |
| filesystemScanMs > 50% of totalMs | INFO | Benchmark filesystem walk with `time find` |
| reconcileSeriesBooksMs > 80% of total | WARN | Try restoring files; check hashFiles setting |
| > 50K fan-out tasks per scan | INFO | Large library; ensure pool size is adequate |
| Task queue queue depth > 10K for > 1h | WARN | Pool too small; check execution times |
| Task queue queue depth > 50K for > 1h | CRITICAL | Pool severely undersized |

---

## 8. Existing Instrumentation Summary

### 8.1 What's Already Instrumented

| Component | Location | Measurement |
|---|---|---|
| Phase timings (10 phases) | `LibraryContentLifecycle.kt:442-462` | `logScanPhase()` — logs `phase`, `durationMs`, counters |
| Aggregate summary | `LibraryContentLifecycle.kt:429-436` | Logs all counters + per-phase `Ms` at end |
| Filesystem scan | `FileSystemScanner.kt:74` | `measureTime { }` — logs series/books/sidecars count |
| Task execution (all types) | `TaskHandler.kt:63` | `measureTime { }` — duration to Micrometer |
| Task failure | `TaskHandler.kt:220` | Counter increment |
| Task execution persistence | `TaskHandler.kt:249` | `TaskExecution` table (id, type, library/series/book IDs, duration, success) |
| Task submission | `TaskEmitter.kt:308-311` | "Sending {N} tasks, sampleType={type}" |

### 8.2 What's NOT Instrumented (gaps)

| Gap | Impact | Effort to Add |
|---|---|---|
| Phase 2 fan-out query durations | Cannot tell if ScanLibrary is slow due to queries in fan-out | Low (add log line per call) |
| Fan-out result counts | Cannot see how many AnalyseBook/HashBook tasks were created | Low (add counter per call) |
| Queue wait time | Cannot measure how long fan-out tasks wait before execution | Medium (requires timestamp in TaskExecution) |
| Filesystem per-file stats | Cannot tell if walk is slow due to large files, many dirs, or latency | High (requires custom FileVisitor) |
| Memory/GC during scan | Cannot correlate scan phases with GC pauses | Medium (requires external monitoring) |

### 8.3 Key Log Formats for Programmatic Parsing

```
# Phase start (useful for timeline reconstruction)
scanRootFolder started scanId=xxx libraryId=yyy scanDeep=false totalMs=N series=N books=N sidecars=N

# Individual phase timing
scanRootFolder phase=filesystem_scan status=ok scanId=xxx libraryId=yyy durationMs=N series=N books=N sidecars=N
scanRootFolder phase=load_existing status=ok scanId=xxx libraryId=yyy durationMs=N existingSeries=N existingScannedSeries=N preloadedBooks=N
scanRootFolder phase=reconcile_series_books status=ok scanId=xxx libraryId=yyy durationMs=N createdSeries=N updatedSeries=N addedBooks=N deferredHashBooks=N outdatedBooks=N seriesRefreshQueued=N
scanRootFolder phase=reconcile_sidecars status=ok scanId=xxx libraryId=yyy durationMs=N changedSidecars=N
scanRootFolder phase=cleanup_sidecars status=ok scanId=xxx libraryId=yyy durationMs=N deletedSidecars=N
scanRootFolder phase=cleanup status=ok scanId=xxx libraryId=yyy durationMs=N

# Summary
scanRootFolder completed status=ok scanId=xxx libraryId=yyy scanDeep=false totalMs=N scannedSeries=N scannedBooks=N ...

# Task emission (fan-out)
Sending {N} tasks, sampleType=AnalyzeBook
Sending {N} tasks, sampleType=HashBook
Sending task: FindBooksToConvert
```

**To parse a scan session from logs:**

```bash
grep 'scanRootFolder\|Sending.*tasks\|Sending task:' /path/to/komga.log | \
  grep 'scanId=<YOUR_SCAN_ID>' > scan-session.log
```

---

## 9. Appendix: Key Source Files

| Component | Absolute Path |
|---|---|
| Task sealed class + ScanLibrary | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/application/tasks/Task.kt` |
| TaskHandler (dispatches all tasks) | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskHandler.kt` |
| TaskProcessor (worker loop, pool) | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskProcessor.kt` |
| TaskEmitter (creates + submits tasks) | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskEmitter.kt` |
| TaskExecution model | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskExecution.kt` |
| TasksRepository (DB access for queue) | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/application/tasks/TasksRepository.kt` |
| LibraryContentLifecycle (scanRootFolder) | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/domain/service/LibraryContentLifecycle.kt` |
| FileSystemScanner (filesystem walk) | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/domain/service/FileSystemScanner.kt` |
| MetricsPublisherController | `/Users/duong/Documents/GitHub/komga/komga/src/main/kotlin/org/gotson/komga/interfaces/scheduler/MetricsPublisherController.kt` |

### Related Analytics Documents

| File | Content |
|---|---|
| `ai-docs/komga-task-scanlibrary-deep-dive.md` | Full architectural deep-dive of ScanLibrary |
| `ai-docs/performance-scan-library.md` | Vietnamese-language performance analysis + optimization plan |
| `ai-docs/perf-issue.md` | Raw DB lock contention evidence |
| `ai-docs/perf-solutions.md` | Solution proposals (SKIP LOCKED, indexes, worker loop) |
| `ai-docs/komga-task-scanlibrary-perf-improve-filesystem-walk.md` | Filesystem walk optimization deep-dive |
| `ai-docs/metrics-tasks-list-plan.md` | Task list UI metrics plan |

---

## Quick Reference: Most Common Bottleneck Patterns

| Pattern | Signature | Most Likely Cause | First Action |
|---|---|---|---|
| **"First scan is slow"** | High `createdSeries`, high `addedBooks`, high `reconcileSeriesBooksMs` | tryRestore computing file hashes for all new books | Disable `hashFiles` if not needed |
| **"Periodic scan is slow"** | High `filesystemScanMs` vs low `createdSeries`/`updatedSeries` | Network filesystem walk is the bottleneck | Benchmark `find` on the same path |
| **"Scan finished but UI still shows analyzing"** | `ScanLibrary` completes quickly; queue filled with tasks | Task pool size too small for fan-out volume | Increase `task-pool-size` |
| **"Scan never finishes"** | Single `ScanLibrary` task runs for days | Extreme case of above; check all phases | Check pool size, DB locks, filesystem |
| **"Scan completed but nothing happened"** | `scannedBooks = 0` or very low | Library path is empty or inaccessible | Check library root path |
| **"All tasks are slow"** | Every phase time is elevated | DB connection pool contention or slow DB | Check `pg_stat_activity` for locks |
