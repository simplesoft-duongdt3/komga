# ScanLibrary Task — Deep Dive

## Problem Statement

> "I see ScanLibrary running 30h and a lot of other tasks running in parallel."

A `ScanLibrary` task running for 30 hours indicates one or more performance bottlenecks. This document provides a complete deep-dive into the `ScanLibrary` task — its execution flow, the sub-tasks it fans out, its database interactions, and the concurrency model — to help diagnose why scans can take extremely long.

---

## 1. Task Definition

```kotlin
class ScanLibrary(
  val libraryId: String,
  val scanDeep: Boolean,
  priority: Int = DEFAULT_PRIORITY,
) : Task(priority) {
  override val uniqueId = "SCAN_LIBRARY_${libraryId}_DEEP_$scanDeep"
}
```

**Key properties:**
- **No `groupId`**: `ScanLibrary` has `groupId = null`, meaning it does NOT block other tasks from running in parallel. Other tasks (like `AnalyzeBook`, `RefreshSeriesMetadata`, etc.) DO have groupIds and will serialize per group, but `ScanLibrary` itself does not participate in group-based concurrency.
- **`uniqueId` ties `libraryId` + `scanDeep`**: If two identical scans are submitted, the second overwrites the first (due to UPSERT behavior). However, a deep scan (`scanDeep=true`) and a regular scan (`scanDeep=false`) are different tasks.
- **`DEFAULT_PRIORITY = 4`**: Not elevated priority. The `LibraryController` can submit with `HIGHEST_PRIORITY` (8) when triggered manually.

---

## 2. How ScanLibrary Gets Triggered

There are **four triggers**:

| Trigger | Code Location | Priority | scanDeep |
|---------|--------------|----------|----------|
| **Manual (API)** — User clicks "Scan" in UI | `LibraryController.kt:235` | `HIGHEST_PRIORITY` (8) | As requested |
| **Startup** — Library has `scanOnStartup = true` | `PeriodicScannerController.kt:28` | `DEFAULT_PRIORITY` (4) | `false` |
| **Periodic** — Library has `scanInterval ≠ DISABLED` | `LibraryScanScheduler.kt:44` | `DEFAULT_PRIORITY` (4) | `false` |
| **Library created/updated** — Library is added or its settings change | `LibraryLifecycle.kt:47, 68` | `DEFAULT_PRIORITY` (4) | `false` |

---

## 3. Execution Timeline

When `TaskHandler.handleTask()` receives a `ScanLibrary` task, it runs a sequence of operations that can be grouped into **two phases**:

### Phase 1: scanRootFolder (Synchronous — runs INSIDE the ScanLibrary task)

```
ScanLibrary task starts
  └─ LibraryContentLifecycle.scanRootFolder(library, scanDeep)
       ├── 1. Filesystem Scan              ── walks the entire directory tree
       ├── 2. Clear Unavailable            ── DB write (fast)
       ├── 3. Load Existing State          ── DB reads (scales with library size)
       ├── 4. Delete Missing Series        ── DB writes (soft deletes)
       ├── 5. Delete Missing Books         ── DB writes (soft deletes)
       ├── 6. Reconcile Series & Books     ── THE EXPENSIVE PART
       │    ├── For new series: create + addBooks + tryRestore
       │    ├── For changed series: update + compare books
       │    │    ├── Same fileSize + has hash → emit verifyBookHash (deferred)
       │    │    └── Different fileSize or no hash → reset + OUTDATED
       │    └── For new books: addBooks + tryRestore
       ├── 7. Sort & Refresh Series        ── sort books + emit RefreshSeriesMetadata
       ├── 8. Reconcile Sidecars           ── compare file modified times
       ├── 9. Cleanup Sidecars             ── delete stale sidecar records
       └── 10. Cleanup                     ── emptyTrash or deleteEmptySets
```

### Phase 2: Fan-out Tasks (Emitted AFTER scanRootFolder, run in parallel via TaskProcessor)

```
ScanLibrary task continues (inside the SAME task handler)
  ├── analyzeUnknownAndOutdatedBooks(library)
  │    └── Query: "WHERE library_id=? AND media_status IN ('UNKNOWN','OUTDATED')"
  │    └── Emit: AnalyzeBook × N books
  │         └── Each AnalyzeBook has groupId = book.seriesId
  │              → Serialized per series: only 1 AnalyzeBook per series at a time
  │
  ├── repairExtensions(library, LOW_PRIORITY)         [IF library.repairExtensions]
  │    └── Query: books with mismatched file extension
  │    └── Emit: RepairExtension × M books (groupId = seriesId)
  │
  ├── findBooksToConvert(library, LOWEST_PRIORITY)    [IF library.convertToCbz]
  │    └── Emit: FindBooksToConvert task
  │         └── When handled: query convertible books → ConvertBook × K books
  │
  ├── findBooksWithMissingPageHash(library, LOWEST_PRIORITY)  [IF library.hashPages]
  │    └── Emit: FindBooksWithMissingPageHash task
  │         └── When handled: query books missing page hashes → HashBookPages × P books
  │
  ├── findDuplicatePagesToDelete(library, LOWEST_PRIORITY)
  │    └── Emit: FindDuplicatePagesToDelete task
  │         └── When handled: query DELETEAUTO pages → RemoveHashedPages × R books
  │
  ├── hashBooksWithoutHash(library)    [IF library.hashFiles]
  │    └── Query: "WHERE library_id=? AND file_hash=''"
  │    └── Emit: HashBook × H books (LOWEST_PRIORITY)
  │
  └── hashBooksWithoutHashKoreader(library)    [IF library.hashKoreader]
       └── Query: "WHERE library_id=? AND file_hash_koreader=''"
       └── Emit: HashBookKoreader × K books (LOWEST_PRIORITY)
```

---

## 4. ScanLibrary Unique Behavior — groupId

**`ScanLibrary` has NO `groupId`.** This is important because:

- `groupId = null` means `ScanLibrary` does NOT block any other task, and no other task blocks it
- `ScanLibrary` can run in parallel with `AnalyzeBook`, `HashBook`, etc.
- The fan-out tasks (`AnalyzeBook`, `RefreshSeriesMetadata`, etc.) DO have `groupId = seriesId`, so they serialize PER SERIES

Because `ScanLibrary` is NOT serialized with the fan-out tasks it emits, those tasks can start executing **while the ScanLibrary task itself is still running** (due to the worker loop in `TaskProcessor.takeAndProcess()` picking them up).

The fan-out tasks emitted during Phase 2 use the **same priority as the parent task + level adjustments**:
- `repairExtensions` → `LOW_PRIORITY` (2)
- `findBooksToConvert` → `LOWEST_PRIORITY` (0) — but the handling emits `ConvertBook` at priority+1
- All other → `LOWEST_PRIORITY` (0) or same as parent

---

## 5. Phase 1 Deep Dive: scanRootFolder

### 5.1 Filesystem Scan

Uses `Files.walkFileTree()` with `FOLLOW_LINKS` and `Integer.MAX_VALUE` depth. For each directory, it:
- Creates a `Series` object with `name`, `url`, `fileLastModified`
- For each file matching extensions (`cbz`, `zip`, `cbr`, `rar`, `pdf`, `epub`), creates a `Book` object
- Collects sidecars separately

**Performance considerations:**
- Walks the ENTIRE directory tree — if the library is on a network filesystem (NFS/SMB), this can be very slow
- The `FOLLOW_LINKS` flag can cause exponential traversal if there are circular symlinks (though `walkFileTree` has cycle detection for symlinks)
- Each file's `BasicFileAttributes` is read, which requires at least one filesystem call per file
- No parallelization — single-threaded walk
- The `forceDirectoryModifiedTime` flag (when true) forces reading each directory's modified time from filesystem

### 5.2 Load Existing State

Reads ALL existing series and books for the library from the database:
- `seriesRepository.findAllByLibraryId(library.id)` — all series in library
- `bookRepository.findAllBySeriesIds(existingScannedSeriesIds)` — all books for matched series

**Performance consideration:** With a large library (10,000+ series, 100,000+ books), this loads a significant amount of data into memory.

### 5.3 Reconcile Series & Books

This is the most CPU- and DB-intensive phase. For each scanned series:

**For new series:**
1. `seriesLifecycle.createSeries(newSeries)` — INSERT into `series`, `series_metadata`, `book_metadata_aggregation` tables
2. `seriesLifecycle.addBooks(createdSeries, newBooks)` — INSERT books, media, book_metadata records
3. `tryRestoreSeries(createdSeries, newBooks)` — queries deleted series, computes file hashes, optionally restores metadata/thumbnails/collections
4. `tryRestoreBooks(newBooks)` — for each new book, queries deleted books by file size, computes file hash, optionally restores media/thumbnails/metadata/read-progress/read-lists

**For changed series:**
1. Compares `fileLastModified` — if changed, updates series
2. If `scanDeep` or `seriesChanged`, iterates through ALL existing books by URL
3. For each matched book with changed `fileLastModified`:
   - Same file size + existing hash → emits `VerifyBookHash` (deferred)
   - Different file size or no hash → resets `fileHash=""`, marks `Media.Status.OUTDATED`
4. Detects new books not matched by URL → adds + restore

**Performance considerations:**
- `tryRestoreSeries` computes file hash for each new book (`hasher.computeHash(book.path)`) — this reads the ENTIRE file
- `tryRestoreBooks` may also compute file hash for restoration matching
- Each `tryRestoreSeries` call does `seriesRepository.findAll(Deleted(IsTrue))` — querying ALL deleted series
- Each new book restoration queries `findAllDeletedByFileSize(fileSize)` — one query per new book
- The entire reconciliation is single-threaded and sequential within the scan task

### 5.4 Sidecar Reconciliation

For each sidecar, compares `lastModifiedTime` with the stored value. If changed:
- Emits `RefreshSeriesLocalArtwork` or `RefreshSeriesMetadata` or `RefreshBookLocalArtwork` or `RefreshBookMetadata`

---

## 6. Phase 2 Deep Dive: Post-Scan Fan-out

After `scanRootFolder` completes, the handler emits 7 groups of sub-tasks (inline, without returning control). The timing is critical:

**The fan-out happens INSIDE the ScanLibrary task execution**, meaning `ScanLibrary` is still "RUNNING" while these queries execute. Only after ALL fan-out queries complete does `ScanLibrary` finish and get deleted from the queue.

### 6.1 `analyzeUnknownAndOutdatedBooks`

Executes a complex search query:
```sql
SELECT book.* FROM book
JOIN media ON book.id = media.book_id
WHERE book.library_id = ?
  AND (media.status = 'UNKNOWN' OR media.status = 'OUTDATED')
ORDER BY seriesId ASC, number ASC
```

The search framework translates the `SearchCondition` tree into a parameterized SQL query. For a library that was just scanned and had many changed files, this can return a very large result set.

Each returned book gets an `AnalyzeBook` task with `groupId = book.seriesId`.

### 6.2 `hashBooksWithoutHash` / `hashBooksWithoutHashKoreader`

```sql
SELECT * FROM book WHERE library_id = ? AND file_hash = ''
```
```sql
SELECT * FROM book WHERE library_id = ? AND file_hash_koreader = ''
```

These queries run even if the library has 0 books with empty hashes. For a first-time scan where no hashes exist yet, this returns ALL books.

Each returned book gets a `HashBook` or `HashBookKoreader` task at `LOWEST_PRIORITY`.

### 6.3 `repairExtensions`

Only runs if `library.repairExtensions = true`. Queries:
```sql
SELECT book.* FROM book
LEFT JOIN media ON book.id = media.book_id
WHERE book.library_id = ?
  AND media.media_type = ?
  AND book.url NOT LIKE '%<correctExtension>'
```

Each returned book gets a `RepairExtension` task at `LOW_PRIORITY`.

### 6.4 `findBooksToConvert`

Only runs if `library.convertToCbz = true`. Emits a single `FindBooksToConvert` task at `LOWEST_PRIORITY`. When that task later executes, it queries:
```sql
SELECT book.* FROM book
LEFT JOIN media ON book.id = media.book_id
WHERE book.library_id = ? AND media.media_type IN ('application/vnd.rar', 'application/x-rar-compressed')
```
And emits `ConvertBook` tasks (each with `groupId = book.seriesId`).

### 6.5 `findBooksWithMissingPageHash`

Only runs if `library.hashPages = true`. Emits a single `FindBooksWithMissingPageHash` task at `LOWEST_PRIORITY`. When that task later executes, it queries for books with ZIP media type that have missing page hashes (limited by `komgaProperties.pageHashing` batch size).

### 6.6 `findDuplicatePagesToDelete`

Always runs. Emits `FindDuplicatePagesToDelete` at `LOWEST_PRIORITY`. When that task later executes, it queries page hash matches with `Action.DELETE_AUTO`.

---

## 7. Concurrency Model

### 7.1 Task Queue Claiming

The `takeFirst()` method in `TasksDao` uses this CTE:

```sql
WITH candidate AS (
  SELECT "ID" FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR NOT EXISTS (
        SELECT 1 FROM "TASK" t2
        WHERE t2."GROUP_ID" = "TASK"."GROUP_ID"
          AND t2."OWNER" IS NOT NULL
          AND t2."GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
  -- PostgreSQL: FOR UPDATE SKIP LOCKED
)
UPDATE "TASK" SET "OWNER" = ? WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"
```

**This means:**
- Tasks with `groupId = null` (like `ScanLibrary`, `HashBook`, `DeleteBook`) can run in **unlimited parallel** (up to `corePoolSize`)
- Tasks with `groupId = seriesId` (like `AnalyzeBook`, `ConvertBook`, `RefreshSeriesMetadata`) serialize **per series** — only one per series can run at a time
- Higher priority tasks jump the queue

### 7.2 Thread Pool

```kotlin
val executor = taskExecutorBuilder
  .threadNamePrefix("taskProcessor-")
  .corePoolSize(settingsProvider.taskPoolSize)  // Default: 1
  .build()
```

**Default pool size = 1**. This means:
- Only **1 task runs at a time** by default
- All fan-out tasks queue up behind `ScanLibrary` if it's still running
- If pool size > 1, multiple tasks can run in parallel

### 7.3 Parallelism Analysis

Given the default pool size of 1:

```
Time ──────────────────────────────────────────────────────►
┌──────────────────────────────────────────────────────────┐
│ ScanLibrary (group=null)                                 │
│  ├─ scanRootFolder() ────────── 10h ──────────►          │
│  └─ fan-out queries ── 1h ──►                             │
└──────────────────────────────────────────────────────────┘
                                                              ← ScanLibrary finishes
┌──────────────────────────────────────────────────────────┐
│ AnalyzeBook[seriesA] (group=seriesA)                     │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ AnalyzeBook[seriesB] (group=seriesB)                     │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ AnalyzeBook[seriesA-2] (group=seriesA)                   │
│  (waits until seriesA-1 AnalyzeBook finishes)             │
└──────────────────────────────────────────────────────────┘
```

With pool size > 1:

```
Time ──────────────────────────────────────────────────────►
┌──────────────────────────────────────────────────────────┐
│ ScanLibrary (group=null)                                 │
│  ├─ scanRootFolder() ────────── 10h ──────────►          │
│  └─ fan-out ──►                                          │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐  Thread 2
│ HashBook (group=null) [LOWEST]                           │
│  ├─ hash file 1                                          │
│  ├─ hash file 2                                          │
│  └─ ...                                                  │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐  Thread 3
│ AnalyzeBook[seriesA] (group=seriesA)                     │
│  (runs in parallel with HashBook)                        │
└──────────────────────────────────────────────────────────┘
```

**Key insight**: With default pool size = 1, `ScanLibrary` blocks ALL other tasks from running until it completes. With pool size > 1, `ScanLibrary` and other tasks (like `HashBook` with `groupId=null` or `AnalyzeBook` for different series) can run concurrently.

---

## 8. Why ScanLibrary Can Run for 30 Hours

### 8.1 Filesystem Walk (Phase 1.1)

The most common bottleneck. `Files.walkFileTree()` is:
- **Single-threaded** — does not use parallel traversal
- **I/O bound** — each file requires a `stat` syscall
- **Network filesystem dependent** — NFS/SMB/CIFS add significant latency per file

**Example**: On a local SSD, walking 100,000 files takes ~5-30 seconds. On NFS with 100ms latency, this balloons to ~3 hours.

### 8.2 Reconciliation (Phase 1.6)

For each changed or new series:
- File hash computation (`hasher.computeHash()`) reads the entire file contents
- For a 500MB comic, hashing takes ~0.5-2 seconds per book
- 10,000 new books × 1 second = ~3 hours just for hashing
- `tryRestoreSeries` and `tryRestoreBooks` add additional file hashing + DB queries

### 8.3 Fan-out Queries (Phase 2)

Even though these are queries + task emissions, they block the `ScanLibrary` task from finishing:
- Querying all books with `file_hash = ''` returns ALL books on first scan
- The search query for `analyzeUnknownAndOutdatedBooks` can be slow on large tables
- Each result row creates a task object in memory

### 8.4 Task Queue Saturation

After `ScanLibrary` finishes, the task queue may contain tens of thousands of tasks:
- `AnalyzeBook` × 50,000 books
- `HashBook` × 50,000 books (if first scan)
- `HashBookKoreader` × 50,000 books (if enabled)
- `RefreshSeriesMetadata` × 10,000 series
- etc.

With pool size = 1, these execute sequentially. Even at 1 second per task, 100,000 tasks = ~28 hours.

---

## 9. Diagnosing Problems

### 9.1 Logs to Examine

The `scanRootFolder` method logs detailed metrics per phase at the end:

```
scanRootFolder completed status=ok scanId=xxx libraryId=yyy scanDeep=false totalMs=36000000
  scannedSeries=5000 scannedBooks=50000 scannedSidecars=0
  existingSeries=4800 existingScannedSeries=4700 preloadedBooks=47000
  deletedSeries=0 deletedBooks=300
  createdSeries=200 updatedSeries=300 addedBooks=2000
  deferredHashBooks=0 outdatedBooks=500
  seriesRefreshQueued=500
  changedSidecars=0 deletedSidecars=0
  filesystemScanMs=3000000       ← 50 min for filesystem walk?
  loadExistingMs=2000
  deleteMissingSeriesMs=100
  deleteMissingBooksMs=500
  reconcileSeriesBooksMs=32000000  ← 9 hours for reconciliation?
  sortAndRefreshMs=10000
  reconcileSidecarsMs=500
  cleanupSidecarsMs=100
  cleanupMs=5000
```

Each phase also logs individually:
```
scanRootFolder phase=filesystem_scan status=ok scanId=xxx libraryId=yyy durationMs=3000000 series=5000 books=50000 sidecars=0
scanRootFolder phase=reconcile_series_books status=ok scanId=xxx libraryId=yyy durationMs=32000000 createdSeries=200 updatedSeries=300 ...
```

### 9.2 Queue Status

Check `GET /api/v1/tasks` to see:
- How many tasks are queued and running
- The `simpleType` distribution (which task types dominate)
- The `priority` and `durationMillis` for each

### 9.3 Metrics

Micrometer metrics:
- `komga.tasks.execution` timer with tag `type=ScanLibrary` — shows min/max/avg/p90 duration
- `komga.tasks.failure` counter with tag `type=ScanLibrary` — shows failure count

### 9.4 Key Questions

| Question | What to Check |
|----------|--------------|
| Is the filesystem walk slow? | `filesystemScanMs` in logs |
| Are too many books being hashed during reconciliation? | `deferredHashBooks` vs `outdatedBooks` — high `outdatedBooks` means files have different sizes, forcing re-analysis |
| Is restoration expensive? | If `createdSeries > 0` and `tryRestore` runs, file hashing adds significant time |
| Is the pool size too small? | Check `taskPoolSize` setting |
| How many fan-out tasks were created? | Compare `scannedBooks` to number of `AnalyzeBook` tasks in the queue |
| Are there infinite fan-out loops? | `RefreshBookMetadata` → `RefreshSeriesMetadata` → `AggregateSeriesMetadata` is a chain, but terminates |

---

## 10. Optimization Strategies

### 10.1 Increase Thread Pool Size

```
komga.settings.task-pool-size = 4
```

This allows `ScanLibrary` and multiple `AnalyzeBook`/`HashBook` tasks to run concurrently. However, be mindful of:
- I/O contention on the filesystem/disk
- Database connection pool exhaustion
- Memory pressure from multiple concurrent book analyses

### 10.2 Disable Unnecessary Features Per Library

| Setting | Impact if Enabled |
|---------|------------------|
| `hashFiles = false` | Eliminates `HashBook` × ALL books (most impactful for first scans) |
| `hashKoreader = false` | Eliminates `HashBookKoreader` × ALL books |
| `hashPages = false` | Eliminates page-level hashing entirely |
| `repairExtensions = false` | No extension repair queries/tasks |
| `convertToCbz = false` | No conversion tasks |
| `analyzeDimensions = false` | Faster book analysis (skip page dimension computation) |

### 10.3 Incremental Scans

- `scanDeep = false` only checks `fileLastModified` for changed files
- Periodic scans (every 6h/12h/day) should use `scanDeep = false`
- Use `scanDeep = true` only when you explicitly want to re-evaluate every book
- First scan of a large library is always expensive — consider doing it during off-hours

### 10.4 Network Filesystem Optimization

- If using NFS: enable `noatime`, increase `rsize`/`wsize`, use `actimeo` to cache attributes
- If using SMB: enable directory caching
- Consider local SSDs for the comic library if possible
- Filesystem walk is purely sequential I/O — HDDs perform poorly for this

### 10.5 Batch Book Hashing

`HashBook` and `HashBookKoreader` are emitted at `LOWEST_PRIORITY`. This means:
- They will be picked up last
- Higher priority tasks like `AnalyzeBook` run first
- The queue backlog clears faster for user-facing operations

However, all tasks still need to be processed eventually. If your library doesn't need file hashing, disable it.

---

## 11. Complete Component Interaction Diagram

@startuml
title ScanLibrary Task — Full Component Interaction

actor User
participant "LibraryController" as API
participant "TaskEmitter" as Emitter
participant "TaskQueue" as Queue
participant "TaskProcessor" as Processor
participant "TaskHandler" as Handler
participant "LibraryContentLifecycle" as LCL
participant "FileSystemScanner" as FSS
participant "SeriesLifecycle" as SL
participant "BookLifecycle" as BL
participant "BookRepository" as BookRepo
participant "SeriesRepository" as SeriesRepo
database "Main DB" as MainDB
database "Tasks DB" as TasksDB

== Trigger ==
API -> Emitter: scanLibrary(libId, scanDeep, HIGHEST_PRIORITY)
Emitter -> TasksDB: save(ScanLibrary)
Emitter -> Processor: emit(TaskAddedEvent)

== Claim & Execute ==
Processor -> TasksDB: takeFirst()
TasksDB --> Processor: ScanLibrary task
Processor -> Handler: handleTask(ScanLibrary)
Handler -> LCL: scanRootFolder(library, scanDeep)
|||
== Phase 1: scanRootFolder ==
LCL -> FSS: scanRootFolder(root, settings)
FSS -> FSS: Files.walkFileTree(root)
note right: Single-threaded walk of\nthe entire directory tree
FSS --> LCL: ScanResult(series→books, sidecars)
|||
LCL -> MainDB: load existing series
LCL -> MainDB: load existing books
LCL -> MainDB: soft-delete missing series
LCL -> MainDB: soft-delete missing books
|||
LCL -> LCL: reconcile series & books (forEach scanned series)
note right #LightCoral: Most expensive phase\nMay compute file hashes\nfor restoration matching
LCL -> SL: createSeries(newSeries)
SL -> MainDB: INSERT series, metadata, aggregation
LCL -> SL: addBooks(series, books)
SL -> MainDB: INSERT books + media + metadata
LCL -> LCL: tryRestoreSeries(createdSeries, newBooks)
LCL -> LCL: tryRestoreBooks(newBooks)
|||
LCL -> SL: sortBooks(series)
SL -> BookRepo: update book numbers
SL -> Emitter: refreshBookMetadata(book, NUMBER)
|||
LCL -> LCL: reconcile sidecars
LCL -> Emitter: refreshSeriesMetadata() / refreshBookMetadata()
|||
LCL -> LCL: cleanup (emptyTrash or deleteEmptySets)
LCL -> MainDB: publishEvent(LibraryScanned)
LCL --> Handler: return
|||
== Phase 2: Fan-out (inline in ScanLibrary handler) ==
Handler -> Emitter: analyzeUnknownAndOutdatedBooks(library)
Emitter -> BookRepo: query WHERE media_status IN ('UNKNOWN','OUTDATED')
BookRepo --> Emitter: result
Emitter -> TasksDB: save(AnalyzeBook × N)
Emitter -> Processor: emit(TaskAddedEvent)
|||
Handler -> Emitter: hashBooksWithoutHash(library)
Emitter -> BookRepo: query WHERE file_hash=''
Emitter -> TasksDB: save(HashBook × H)
Emitter -> Processor: emit(TaskAddedEvent)
|||
Handler -> Emitter: hashBooksWithoutHashKoreader(library)
Emitter -> BookRepo: query WHERE file_hash_koreader=''
Emitter -> TasksDB: save(HashBookKoreader × K)
Emitter -> Processor: emit(TaskAddedEvent)
|||
Handler -> Emitter: repairExtensions(library, LOW_PRIORITY)
Handler -> Emitter: findBooksToConvert(library, LOWEST_PRIORITY)
Handler -> Emitter: findBooksWithMissingPageHash(library, LOWEST_PRIORITY)
Handler -> Emitter: findDuplicatePagesToDelete(library, LOWEST_PRIORITY)

Handler --> Processor: return (ScanLibrary complete)
Processor -> TasksDB: delete(ScanLibrary)
note right #LightGreen: ScanLibrary task finished.\nFan-out tasks are now\nbeing processed concurrently.

== Concurrent Task Processing (other threads) ==
Processor -> TasksDB: takeFirst() → AnalyzeBook
Processor -> TasksDB: takeFirst() → HashBook
Processor -> TasksDB: takeFirst() → HashBookKoreader
note right: Up to corePoolSize tasks run in parallel.\nTasks with same groupId serialize.\nTasks without groupId run freely.

@enduml

---

## 12. Summary

| Aspect | Detail |
|--------|--------|
| **What it does** | Full library scan: walks filesystem, reconciles DB state, fans out sub-tasks |
| **groupId** | `null` — does NOT block other tasks, but is also NOT serialized with any task |
| **Default priority** | 4 (DEFAULT), can be 8 (HIGHEST) when triggered manually |
| **sync/async** | Phase 1 is fully synchronous and single-threaded; Phase 2 emits async sub-tasks |
| **Fan-out tasks** | Up to 7 different task types, potentially 100,000+ individual tasks |
| **Primary performance risk** | Filesystem walk on network storage + file hash computation + huge fan-out |
| **30h scenario causes** | (1) Slow filesystem walk, (2) Large reconciliation with file hashing, (3) Pool size = 1 causing serial execution of 100K+ fan-out tasks |
| **Mitigation** | Increase pool size, disable unnecessary features, use incremental scans, optimize storage |

The 30-hour runtime is typically caused by a combination of:
1. A very large library on a slow filesystem (NFS/SMB) taking hours just to walk the directory tree
2. The reconciliation phase computing file hashes for many new/changed books
3. Default pool size of 1 causing the entire queue of 50K-100K+ sub-tasks to execute sequentially after ScanLibrary finishes
4. Features like `hashFiles=true`, `hashKoreader=true`, or `hashPages=true` multiplying the task count
