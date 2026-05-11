# ScanLibrary with Pre-Computed Filesystem-DB Diff — Analysis

## Question

> What if the external tool generates a "filesystem vs database diff" JSON file as input, instead of raw filesystem state?

This is a **qualitatively better** approach than passing raw `ScanResult`. Instead of the external tool just walking the filesystem, it also has knowledge of the database state and computes only the **delta** — what changed, what's new, what's gone.

---

## 1. Concept

```
                    ┌──────────────────────────────┐
                    │     External Tool             │
                    │   (Go/Rust/Python daemon)     │
                    │                               │
                    │  1. Walk filesystem (fast)    │
                    │  2. Query DB state (API/SQL)  │
                    │  3. Compute DIFF              │
                    │     ┌──────────────────┐      │
                    │     │ Series: NEW       │      │
                    │     │ Series: DELETED   │      │
                    │     │ Book: CHANGED     │      │
                    │     │ Series: SAME      │(skip)│
                    │     └──────────────────┘      │
                    │  4. Output: diff JSON         │
                    └──────────┬───────────────────┘
                               │
                               ▼
                    ScanLibrary task receives diff
                      └─ Only processes CHANGED items
                      └─ Skips everything SAME
                      └─ NO "load existing" needed
                      └─ NO "delete missing" loop
```

---

## 2. What the Current Code Does vs What the Diff Replaces

Let's map every phase of `LibraryContentLifecycle.scanRootFolder()` and see what the diff eliminates.

### Current Phase Mapping

| Phase | Current Behavior | Lines | DB Ops | Can Diff Replace? |
|-------|-----------------|-------|--------|------------------|
| **1. Filesystem Scan** | `Files.walkFileTree()` — walks everything | 94-116 | 0 | ✅ Replaced entirely |
| **2. Clear Unavailable** | Check if dir was previously unavailable | 122-136 | 1 UPDATE | ❌ Stays (external tool can't know this) |
| **3. Load Existing** | `findAllByLibraryId` + `findAllBySeriesIds` — loads ALL series/books | 145-175 | 2 SELECTs | ✅ **REPLACED** — tool already queried DB |
| **4. Delete Missing Series** | `findAllNotDeletedByLibraryIdAndUrlNotIn` — find series not on disk | 185-204 | 1 SELECT + N UPDATE/SOFTDELETE | ✅ **REPLACED** — diff contains `removedSeries` |
| **5. Delete Missing Books** | `findAllNotDeletedByLibraryIdAndUrlNotIn(books)` — find books not on disk | 209-232 | 1 SELECT + N SOFTDELETE | ✅ **REPLACED** — diff contains `removedBooks` |
| **6a. Create New Series** | `createSeries` + `addBooks` + `tryRestoreSeries` + `tryRestoreBooks` | 246-275 | ~5 INSERTs per series + file hashing | ✅ **REPLACED** — diff contains `newSeries` |
| **6b. Update Changed Series** | Compare `fileLastModified` per series, iterate ALL books | 276-338 | N UPDATEs/INSERTs | ✅ **REPLACED** — diff contains `changedBooks` only |
| **7. Sort & Refresh** | `sortBooks` + `emit RefreshSeriesMetadata` for ALL touched series | 343-355 | SELECTs + UPDATEs | ⚠️ **Stays, but smaller scope** (only affected series) |
| **8. Reconcile Sidecars** | `findAll()` sidecars, compare timestamps | 357-400 | SELECT + N INSERTs | ⚠️ **Partially replaced** — diff can contain sidecar changes |
| **9. Cleanup Sidecars** | Delete stale sidecar records | 402-416 | N DELETEs | ✅ **REPLACED** — diff contains removal list |
| **10. Cleanup** | `emptyTrash` or `deleteEmptySets` | 418-427 | N DELETEs | ❌ Stays (independent logic) |

### What's Left After Diff Input

After replacing everything the diff covers, the ScanLibrary handler becomes:

```
ScanLibrary task (with diff input)
  ├── Clear Unavailable   (1 UPDATE, trivial)
  ├── For each newSeries in diff:
  │     createSeries + addBooks (5 INSERTs per series)
  │     tryRestoreSeries + tryRestoreBooks (file hashing — still needed)
  ├── For each removedSeries in diff:
  │     softDeleteMany (1 UPDATE per series)
  ├── For each removedBook in diff:
  │     softDeleteMany (1 UPDATE per book)
  ├── For each changedBook in diff:
  │     update + reset media or defer hash verification
  ├── Sort & Refresh (only touched series)
  ├── Reconcile sidecars from diff
  ├── Cleanup empty sets
  └── Phase 2 fan-out (AnalyzeBook, HashBook, etc.)
```

---

## 3. Proposed Diff JSON Format

```jsonc
{
  "libraryId": "abc123",
  "scanTimestamp": "2026-05-11T12:00:00Z",
  "scanDeep": false,

  // Everything in "series" key is CHANGED or NEW — no unchanged series included
  "series": {
    "seriesUrl1": {
      "url": "file:///comics/series1",
      "name": "Series 1",
      "fileLastModified": "2026-05-10T10:00:00Z",
      "oneshot": false,
      "status": "NEW",              // <-- "NEW" = series doesn't exist in DB

      "books": [
        {
          "url": "file:///comics/series1/book1.cbz",
          "name": "Book 1",
          "fileLastModified": "2026-05-10T10:00:00Z",
          "fileSize": 123456789,
          "status": "NEW"           // <-- "NEW" = book doesn't exist in DB
        }
      ]
    },
    "seriesUrl2": {
      "url": "file:///comics/series2",
      "name": "Series 2",
      "fileLastModified": "2026-05-11T08:00:00Z",
      "status": "CHANGED",          // <-- "CHANGED" = series timestamps differ

      "books": [
        {
          "url": "file:///comics/series2/book2.cbz",
          "name": "Book 2",
          "fileLastModified": "2026-05-11T08:00:00Z",
          "fileSize": 987654321,
          "status": "CHANGED",      // <-- fileLastModified different from DB
          "previousFileSize": 987654321,  // helps decide: same size = defer hash
          "hadHash": true            // was hash previously computed?
        },
        {
          "url": "file:///comics/series2/book3_new.cbz",
          "name": "Book 3 New",
          "fileLastModified": "2026-05-11T08:00:00Z",
          "fileSize": 111222333,
          "status": "NEW"           // <-- not in DB at all
        }
      ]
    }
  },

  // Deletions — absolute list of what's gone from filesystem
  "removedSeries": [
    "file:///comics/series3",
    "file:///comics/series4"
  ],
  "removedBooks": [
    "file:///comics/series1/book_old.cbz",
    "file:///comics/series5/book_gone.cbz"
  ],

  // Sidecar changes
  "changedSidecars": [
    {
      "url": "file:///comics/series1/series.json",
      "parentUrl": "file:///comics/series1",
      "lastModifiedTime": "2026-05-11T08:00:00Z",
      "type": "METADATA",
      "source": "SERIES"
    }
  ],
  "removedSidecars": [
    "file:///comics/series2/cover.jpg"
  ],

  // Tool metadata
  "toolInfo": {
    "name": "komga-fs-watcher",
    "version": "1.0.0",
    "filesystemWalkMs": 3500,
    "databaseQueryMs": 1200,
    "diffComputeMs": 800
  }
}
```

---

## 4. What's Eliminated vs What's Still Needed

### 4.1 Completely Eliminated

| Operation | Reason |
|-----------|--------|
| **Full filesystem walk** | Tool did it in parallel |
| **`findAllByLibraryId`** | Tool already queried DB |
| **`findAllBySeriesIds` with ALL books preloaded** | Only need books for changed series now |
| **`findAllNotDeletedByLibraryIdAndUrlNotIn` (series)** | Diff explicitly lists removed series |
| **`findAllNotDeletedByLibraryIdAndUrlNotIn` (books)** | Diff explicitly lists removed books |
| **Series `fileLastModified` comparison loop** | Tool already determined CHANGED vs SAME |
| **Book URL matching per series** | Tool already determined status per book |
| **Sidecar `findAll()` + comparison** | Tool already determined changes |

### 4.2 Still Needed (But Scoped to Diff Only)

| Operation | Why Still Needed |
|-----------|-----------------|
| **`createSeries` + `addBooks`** | Still need to INSERT into DB |
| **`tryRestoreSeries` / `tryRestoreBooks`** | Still need to match with deleted records + restore metadata(read progress, read lists, collections) — AND compute file hash for matching |
| **softDelete** for removed series/books | Still need to mark them deleted |
| **UPDATE for changed books** | Still need to reset media status or defer hash |
| **`sortBooks` + `RefreshSeriesMetadata`** | Still need to re-sort and refresh affected series |
| **Phase 2 fan-out** | Still need to emit `AnalyzeBook`, `HashBook`, etc. |

---

## 5. Performance Impact Analysis

### 5.1 What's the Remaining Bottleneck?

After diff input, `ScanLibrary` becomes a **pure DB reconciliation of only the delta**. The dominant remaining cost shifts from "walking 100K unchanging files" to:

| New Bottleneck | Cost | Mitigation |
|----------------|------|-----------|
| **`tryRestoreSeries` / `tryRestoreBooks`** | Computes file hash (reads entire file) for every new book | Fresh libraries: skip entirely. Use `fileSize + fileName` matching instead of hash. |
| **`createSeries` + `addBooks` DB INSERTs** | 5+ INSERTs per new series/book | Batch inserts (already batched with `chunked(batchSize)`) |
| **Phase 2 fan-out queries** | Query all UNKNOWN/OUTDATED books, all empty hashes | Already runs in every case |
| **No filesystem walk** | ❌ **ELIMINATED** | — |

### 5.2 Worst-Case Scenario Transformed

**Before (100K files on NFS, 5 changed books):**
```
filesystem walk:  3 hours      ← walks 99,995 unchanged files
load existing:    30 seconds   ← loads 100K books into memory
reconcile:        5 seconds    ← iterates 10K series to find 1 changed
fan-out:          2 seconds
─────────────────────────────────
Total:            ~3.5 hours
```

**After diff input (same scenario):**
```
read diff JSON:   0.1 seconds   ← 1 KB payload (5 changed items)
reconcile delta:  5 seconds     ← 1 series updated, 2 books changed
fan-out:          2 seconds
─────────────────────────────────
Total:            ~7 seconds    ← 1,800x faster
```

### 5.3 Payload Size: Critical Advantage

The diff JSON is **proportional to the number of changes, not the library size**.

| Scenario | Raw ScanResult | Diff (with DB knowledge) |
|----------|---------------|--------------------------|
| Full first scan (100K books) | ~33 MB | ~33 MB (everything is NEW) |
| Periodic scan, 0 changes | ~33 MB | **~200 bytes** (empty diff) |
| Periodic scan, 5 changed books | ~33 MB | **~1 KB** |
| Periodic scan, 100 new books | ~33 MB | **~30 KB** |
| New library, first scan | ~33 MB | ~33 MB (everything is NEW) |

**For the 99% case** (periodic scans with few changes), the diff is **10,000x to 100,000x smaller** than the full scan result.

### 5.4 First Scan Remains Expensive (But Why)

On a **first scan** (brand new library, nothing in DB), every single file is "NEW". The diff is identical in size to the raw scan result (~33 MB for 100K books). More importantly:

- `createSeries` + `addBooks` must still execute 100K INSERTs
- `tryRestoreSeries` / `tryRestoreBooks` is actually **faster** because there are no deleted records to match against (queries return empty → skip)
- File hashing still happens only if `tryRestore` matching is attempted

**The first scan bottleneck shifts from filesystem walk to DB write throughput.** This is a significantly easier problem to solve (batch sizes, connection pool, index tuning).

---

## 6. Staleness Problem Solved

The raw `ScanResult` approach has a staleness problem: files can change between the walk and when the task executes. The diff approach **inherently handles this** because:

1. The diff captures the EXACT state at a point in time
2. If a file changes after the diff was generated, the next diff will pick it up
3. The `scanTimestamp` in the diff allows Komga to detect stale diffs and reject them:

```kotlin
is Task.ScanLibraryWithDiff -> {
    if (diff.scanTimestamp < lastCompletedScan[library.id]) {
        logger.warn { "Diff is stale, skipping" }
        return  // Skip — newer scan already happened
    }
    processDiff(library, diff)
}
```

---

## 7. Architectural Comparison

| Aspect | Raw ScanResult as Input | **Diff as Input** |
|--------|------------------------|-------------------|
| **Payload size (typical scan)** | ~33 MB | **~1-30 KB** |
| **Payload size (first scan)** | ~33 MB | ~33 MB (identical) |
| **Komga work eliminated** | Filesystem walk only | Filesystem walk + LOAD existing + DELETE matching + CHANGE detection |
| **Staleness handling** | Potential issue (files change between walk & reconcile) | ✅ Diff timestamp allows staleness detection |
| **Database queries in external tool** | No | Yes (needs to compare with DB state) |
| **Tool complexity** | Low (pure filesystem walk) | **Medium** (walk + DB query + diff computation) |
| **DB credentials needed in tool** | No | Yes |
| **Failover on no diff available** | Komga can still do full scan | Komga can still do full scan (fallback) |

---

## 8. Implementation Path

### Step 1: Define the diff data model

```kotlin
data class ScanDiff(
    val libraryId: String,
    val scanTimestamp: LocalDateTime,
    val scanDeep: Boolean = false,

    // Only CHANGED or NEW series — unchanged series are NOT included
    val changedSeries: Map<String, ChangedSeries>,  // key = URL

    val removedSeries: List<String>,   // URLs of series gone from filesystem
    val removedBooks: List<String>,    // URLs of books gone from filesystem

    val changedSidecars: List<SidecarDiff>,
    val removedSidecars: List<String>,

    val toolInfo: ScanDiffToolInfo? = null,
)

data class ChangedSeries(
    val url: String,
    val name: String,
    val fileLastModified: LocalDateTime,
    val oneshot: Boolean = false,
    val changedBooks: List<ChangedBook>,  // only NEW or CHANGED books
)

data class ChangedBook(
    val url: String,
    val name: String,
    val fileLastModified: LocalDateTime,
    val fileSize: Long,
    val status: DiffStatus,  // NEW or CHANGED
    val previousFileSize: Long? = null,
    val hadHash: Boolean = false,
)

enum class DiffStatus { NEW, CHANGED }
```

### Step 2: New task type

```kotlin
class ScanLibraryWithDiff(
    val libraryId: String,
    val diffJsonPath: String,  // path to JSON file on disk
    priority: Int = DEFAULT_PRIORITY,
) : Task(priority) {
    override val uniqueId = "SCAN_LIBRARY_DIFF_${libraryId}_${diffJsonPath.hashCode()}"
}
```

### Step 3: Modified handler

```kotlin
is Task.ScanLibraryWithDiff -> {
    libraryRepository.findByIdOrNull(task.libraryId)?.let { library ->
        val diff = objectMapper.readValue(Paths.get(task.diffJsonPath).toFile(), ScanDiff::class.java)
        libraryContentLifecycle.processDiff(library, diff)
    }
}
```

### Step 4: New reconciliation method (replaces ~50% of scanRootFolder)

```kotlin
fun processDiff(library: Library, diff: ScanDiff) {
    // 1. Clear unavailable (same as current)
    // 2. Load only affected series from DB (not everything)
    val affectedSeriesIds = diff.changedSeries.keys + diff.removedSeries
    val existingAffectedSeries = seriesRepository.findByUrls(affectedSeriesIds).associateBy { it.url }

    // 3. Process removed series (no query needed — list is in diff)
    diff.removedSeries.mapNotNull { existingAffectedSeries[it] }.let { toDelete ->
        seriesLifecycle.softDeleteMany(toDelete)
    }

    // 4. Process removed books (no query needed)
    diff.removedBooks.chunked(batchSize).forEach { chunk ->
        bookRepository.softDeleteByUrls(chunk)
    }

    // 5. Process changed series
    diff.changedSeries.forEach { (seriesUrl, changedSeries) ->
        val existingSeries = existingAffectedSeries[seriesUrl]
        if (existingSeries == null && changedSeries.changedBooks.isNotEmpty()) {
            // NEW series
            val created = seriesLifecycle.createSeries(seriesFromDiff(changedSeries, library.id))
            seriesLifecycle.addBooks(created, changedSeries.changedBooks.map { bookFromDiff(it) })
            seriesToSortAndRefresh.add(created)
        } else if (existingSeries != null) {
            // CHANGED series — only process the changed books listed in diff
            processChangedBooks(existingSeries, changedSeries.changedBooks)
            if (changedSeries.fileLastModified != existingSeries.fileLastModified) {
                seriesRepository.update(existingSeries.copy(fileLastModified = changedSeries.fileLastModified))
            }
            seriesToSortAndRefresh.add(existingSeries)
        }
    }

    // 6. Sort & refresh only touched series
    // 7. Process sidecar diff
    // 8. Cleanup empty sets
    // 9. Phase 2 fan-out
}
```

---

## 9. Real-World Gains Table

| Library Size | Scan Type | Current (NFS) | Diff Input | Speedup |
|-------------|-----------|---------------|------------|---------|
| 100K books | First scan | ~4 hours | ~25 min (DB writes) | ~10x |
| 100K books | Periodic (0 changes) | ~3 hours | **~1 sec** | ~10,000x |
| 100K books | Periodic (5 changes) | ~3 hours | **~5 sec** | ~2,000x |
| 100K books | Periodic (100 new books) | ~3 hours | **~30 sec** | ~360x |
| 10K books | First scan | ~25 min | ~3 min | ~8x |
| 10K books | Periodic (0 changes) | ~25 min | **~1 sec** | ~1,500x |

---

## 10. Verdict

**The diff approach is dramatically better than the raw ScanResult approach.** Here's why:

| Criteria | Raw ScanResult | **Diff** |
|----------|---------------|----------|
| **Payload transfer** | 33 MB every scan (even if nothing changed) | **~200 bytes when nothing changed** |
| **Komga work eliminated** | Only filesystem walk | Filesystem walk + LOAD ALL + DELETE detection + CHANGE detection **= 50-70% of total phase 1 work** |
| **Phase 1b eliminated** (load existing) | ❌ Still loads ALL series/books in memory | ✅ **Only loads affected series** (tiny) |
| **Phase 1c eliminated** (reconcile) | ❌ Still iterates ALL series comparing URLs | ✅ **Only iterates changed items** |
| **Phase 4 eliminated** (delete missing) | ❌ Still queries for "not in list" | ✅ **Explicit list in diff** |
| **Staleness** | Moderate risk | ✅ **Timestamp-based staleness detection** |
| **External tool complexity** | Low (walk only) | Medium (walk + DB query + compute diff) |
| **First scan cost** | Same for both | Same for both (everything is diff) |
| **Incremental walk support** | Possible (inotify → delta scan result) | **Trivial** (inotify → very small diff) |

### Bottom Line

**If you implement an external tool, the diff approach is the correct design.** The raw scan result approach only eliminates 1 of 10 phases. The diff approach eliminates 5-6 of 10 phases for the common case (periodic scans with few changes), and completely eliminates the O(n) scaling problem where n = library size.

The only scenario where both approaches perform identically is the **first scan** — and even then, the diff approach eliminates the "load existing" and "delete missing" queries (trivial savings), plus the diff is smaller for the handler to deserialize since there's nothing to compare against.
