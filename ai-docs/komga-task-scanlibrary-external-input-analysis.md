# ScanLibrary with External Filesystem State — Feasibility & Impact Analysis

## Question

> What if an external tool generates the current state of all files in a library and sends it as input to the `ScanLibrary` task? Will this improve performance?

This is a common architectural pattern — separating **filesystem scanning** from **database reconciliation**. Instead of `ScanLibrary` doing both (which makes it synchronous and slow), an external tool walks the filesystem and produces a `ScanResult`, which is then passed to `ScanLibrary` for the reconciliation-only step.

---

## 1. Current Architecture

```
ScanLibrary task
  ├── Phase 1a: Filesystem Scan (FileSystemScanner.walkFileTree)
  │     → Walks entire directory tree
  │     → Creates Series + Book + Sidecar objects in memory
  │     → Output: ScanResult(series→books, sidecars)
  │
  ├── Phase 1b: Load Existing DB State
  │     → Queries all series/books for this library
  │
  ├── Phase 1c: Reconcile (diff + update DB)
  │     → Creates new series/books
  │     → Soft-deletes missing ones
  │     → Updates changed ones
  │     → tryRestoreSeries / tryRestoreBooks (file hash computation)
  │     → Sorts books, handles sidecars
  │
  └── Phase 2: Fan-out tasks
        → Emits AnalyzeBook, HashBook, etc.
```

---

## 2. Proposed Architecture

```
External Tool (Go/Rust/Python)
  ├── Walks directory tree (can be parallelized)
  ├── Uses inotify/fanotify/kqueue for incremental
  ├── Output: JSON file or API call with ScanResult
  │
  v
ScanLibrary task (now only Phase 1b + 1c + 2)
  ├── Loads ScanResult from input (no filesystem scan)
  ├── Phase 1b: Load Existing DB State
  ├── Phase 1c: Reconcile (diff + update DB)
  └── Phase 2: Fan-out tasks
```

---

## 3. What Would Be Removed from ScanLibrary

The entire **filesystem scan phase** (`FileSystemScanner.scanRootFolder`) is a synchronous, single-threaded walk of the filesystem using `Files.walkFileTree()`. This is the component that would be replaced.

### 3.1 Current Cost: `filesystemScanMs`

From the logs, `filesystemScanMs` represents 100% of what an external tool would absorb:

- **Local SSD**: 5-30 seconds for 100K files
- **NFS/SMB**: 0.5-3+ hours for 100K files (dominated by network latency + metadata ops)
- **The walk produces only**: `ScanResult(series: Map<Series, List<Book>>, sidecars: List<Sidecar>)` — purely in-memory metadata

### 3.2 What the External Tool Must Produce

```json
{
  "series": {
    "Series(url, fileLastModified, name, oneshot)":
      ["Book(url, fileLastModified, fileSize, name)", ...],
    ...
  },
  "sidecars": [
    "Sidecar(url, parentUrl, lastModifiedTime, type, source)",
    ...
  ]
}
```

**For a library of 10,000 series × 10 books each = 100,000 books ⬇️**

| Data | Objects | Approx JSON Size |
|------|---------|-----------------|
| Series | 10,000 | ~3 MB |
| Books | 100,000 | ~30 MB |
| Sidecars | ~1,000 | ~0.3 MB |
| **Total** | | **~33 MB** |

**The `PAYLOAD` column in `TASK` table is `text` (unbounded).** In the current JSON serialization via Jackson ObjectMapper:

- A `ScanLibrary` task payload today is tiny: `{"libraryId":"xxx","scanDeep":false}` — about 50 bytes
- A `ScanLibrary` task carrying the full `ScanResult` would be **~33 MB**

### 3.3 Database Impact of Large Payload

| Operation | Current | With External Input |
|-----------|---------|-------------------|
| `INSERT INTO TASK` | 50 bytes payload | 33 MB payload |
| `ON DUPLICATE KEY UPDATE` | 50 bytes | 33 MB |
| JSON serialization (write) | Instant | ~100-500ms |
| JSON deserialization (read in `takeFirst`) | Instant | ~100-500ms |
| Storage per scan | Negligible | 33 MB per scan |
| Multiple libraries scanning | Negligible | 33 MB × N libraries |

**The task queue would need to transfer 33 MB through `takeFirst()` → `handleTask()` → `delete()` every scan cycle.** This is plausible but adds latency to the task claiming step.

---

## 4. Performance Impact Analysis

### 4.1 What Gets Faster

| Phase | Current Time | With External Input | Savings |
|-------|-------------|-------------------|---------|
| Filesystem scan (NFS, 100K files) | 3 hours | **0** (external tool) | ~3 hours |
| Load existing DB state | 30 seconds | 30 seconds | 0 |
| Reconcile series & books | Varies | Varies | 0 |
| **Total ScanLibrary runtime** | **3+ hours** | **~minutes** | **Massive** |

### 4.2 What Does NOT Get Faster

The following phases run IDENTICALLY regardless of input source:

- **Load existing DB state** (`findAllByLibraryId`, `findAllBySeriesIds`) — still reads all series/books
- **Reconciliation loop** — still iterates every scanned series and book
- **`tryRestoreSeries` / `tryRestoreBooks`** — still computes file hashes (reads entire files from disk!)
- **Sort & refresh** — still sorts books and emits `RefreshSeriesMetadata`
- **Sidecar reconciliation** — still compares timestamps
- **Phase 2 fan-out** — still queries DB and emits all sub-tasks

### 4.3 What Gets Worse

| Aspect | Impact |
|--------|--------|
| **Jackson serialization of payload** | 33 MB JSON produces ~100-500ms extra per save + read |
| **Task queue table bloat** | `PAYLOAD` column stores full scan state as text |
| **Memory in TaskProcessor** | Deserialized `ScanResult` with 100K+ objects sits in heap until handled (~500 MB) |
| **Duplicate handling** | On restart, `disown()` re-queues tasks — 33 MB payload gets processed again |

### 4.4 The Bigger Bottleneck: Reconciliation

After the filesystem scan, **the reconciliation phase in `LibraryContentLifecycle` is NOT I/O bound on the filesystem** — it's DB-bound and CPU-bound:

```kotlin
// Inside reconcile loop — for each new series:
seriesLifecycle.createSeries(newSeries)        // 3 INSERTs (series, metadata, aggregation)
seriesLifecycle.addBooks(createdSeries, newBooks) // 3 INSERTs per book (book, media, metadata)
tryRestoreSeries(createdSeries, newBooks)       // file hash computation + more INSERTs
tryRestoreBooks(newBooks)                       // file hash computation + more INSERTs
```

**Each new book triggers up to 5+ INSERTs.** For 10,000 new books, that's 50,000+ DB writes. These don't go away with external input.

**`tryRestoreSeries` and `tryRestoreBooks` compute file hashes by reading every file on disk.** This is a heavy I/O operation that happens regardless of whether the filesystem walk was external:

```kotlin
// In tryRestoreSeries:
val newBooksWithHash = newBooks.map { book ->
    bookRepository.findByIdOrNull(book.id)!!.copy(
        fileHash = hasher.computeHash(book.path)  // Reads ENTIRE file!
    )
}
```

---

## 5. Cost-Benefit Summary

| Scenario | Original | External Tool | Improvement |
|----------|----------|---------------|-------------|
| **First scan, local SSD, 100K books** | ~30 min total | ~25 min total | ~5 min (filesystem walk) |
| **First scan, NFS, 100K books** | ~4 hours total | ~1 hour total | ~3 hours (filesystem walk) |
| **Incremental scan, NFS, few changes** | ~3 hours total | ~few min total | **Massive** (walk dominates) |
| **Startup scan (scanOnStartup=true)** | Blocks startup | Doesn't block startup (external tool generates state ahead of time) | Huge UX improvement |

### 5.1 Where External Input Helps MOST

1. **NFS/SMB libraries** — filesystem walk is the #1 bottleneck due to metadata latency
2. **Periodic scans with few changes** — walking the entire tree just to find 5 changed files is wasteful
3. **Startup scans** — a pre-generated state can be consumed quickly without blocking application startup
4. **Very large libraries** (500K+ files) — the walk time dominates

### 5.2 Where External Input Helps LEAST

1. **Small libraries on local SSD** — walk takes seconds, overhead of external tool may be negative
2. **First-time scans** — reconciliation is still the bottleneck (file hashing, DB inserts)
3. **Libraries with `hashFiles=true`** — most of the time is in hash computation, not the walk

### 5.3 What External Input Does NOT Fix

| Problem | Root Cause | Needs Different Fix |
|---------|-----------|-------------------|
| **Slow reconciliation with many new books** | Series creation + book insertion + restoration loops | Batch inserts, skip tryRestore for fresh imports |
| **File hashing in tryRestore** | Reads entire file content | Skip restoration entirely for fresh libraries |
| **Slow phase 2 fan-out queries** | `findAll` with JOINs on media | Add indexes, reduce query scope |
| **Too many fan-out tasks (HashBook × 100K)** | `hashFiles=true` on first scan | Disable hashing, or lazy-hash on demand |
| **Single-threaded reconciliation** | Kotlin `forEach` loop | Parallelize reconciliation per series |

---

## 6. Architectural Considerations

### 6.1 Option A: External Tool → API → TaskQueue (Your Proposal)

```
External Tool ──HTTP/JSON──► Komga API ──► TaskQueue ──► ScanLibrary handler
```

**Pros:**
- Clean separation of concerns
- External tool can be in any language (Go for parallel walk, Rust for efficiency)
- Can use OS-level notifications (inotify/kqueue) to avoid full walks
- Can parallelize the directory walk

**Cons:**
- Large payload: ~33 MB serialized scan result in the task queue DB
- Serialization/deserialization overhead
- Reconciled data may be stale by the time the task runs (files changed between walk and reconciliation)
- Need to version the `ScanResult` format

### 6.2 Option B: External Tool writes directly to a "filesystem snapshot" table

```
External Tool ──► "fs_snapshot" table ──► ScanLibrary reads snapshot
```

- Intermediate table avoids large task payload
- Can compare snapshots at the DB level (SQL diff)
- More queryable/filterable than JSON blob

### 6.3 Option C: External Tool produces a file on disk

```
External Tool ──► /tmp/scan_{libraryId}_{timestamp}.json
                  ScanLibrary reads file path
```

- No DB payload bloat
- The task carries only a `scanResultPath: String` parameter
- Needs file cleanup/discovery mechanism

---

## 7. Incremental Scanning: The Better Alternative

Rather than a full external walk every time, consider **incremental change detection**:

### 7.1 inotify/fanotify/kqueue (OS-level file watching)

The external tool registers watches on the library root. When files change, it sends only the **delta**:

```json
{
  "added": [{"url": "...", "fileSize": 12345, "fileLastModified": "..."}],
  "removed": [{"url": "..."}],
  "modified": [{"url": "...", "fileSize": 67890, "fileLastModified": "..."}]
}
```

**Advantage**: Near-instant detection, minimal data transfer, no full walk.

**Challenge**: Initial state still needs a full walk. Re-watching after restart also needs a full walk.

### 7.2 Debounced batching

Group changes into batches (e.g., every 5 seconds, or every 100 changes), then submit a single `ScanLibrary` task with the batch delta.

---

## 8. Recommendation

### Decision Matrix

| Factor | Current | External Input |
|--------|---------|---------------|
| Complexity | Low | High (new tool, serialization, staleness) |
| NFS performance | Awful | **Excellent** |
| Local SSD performance | Good | Slightly worse (overhead) |
| Startup time | Blocks | **Does not block** |
| Maintenance | None | External tool must be maintained |
| Payload size | ~50 bytes | ~33 MB |
| Staleness risk | None | **Real** (files change between walk & reconcile) |
| Incremental support | None | **Possible** (inotify → delta) |

### Verdict

**For NFS/SMB libraries ≥ 50K files**: ✅ **Yes, significant benefit.** The filesystem walk dominates the runtime. An external tool that walks once and then uses inotify delta would reduce scan time from hours to seconds.

**For local SSD libraries < 50K files**: ❌ **Not worth it.** The walk is fast, and the overhead of serialization, increased payload, and complexity outweigh the gains.

**For libraries where `hashFiles=true` and many books need hashing**: ⚠️ **Limited benefit.** File hashing during reconciliation (`tryRestoreSeries`/`tryRestoreBooks`) still reads every file — this is often the true bottleneck, not the walk.

### Starting Point for Implementation

If you proceed, the simplest approach is **Option C** (file path):

```kotlin
// New task type
class ScanLibrary(
    val libraryId: String,
    val scanDeep: Boolean,
    val scanResultPath: String? = null,  // null = do the walk ourselves
    priority: Int = DEFAULT_PRIORITY,
) : Task(priority) {
    override val uniqueId = "SCAN_LIBRARY_${libraryId}_DEEP_${scanDeep}_PATH_${scanResultPath != null}"
}
```

```kotlin
// Modified handler
is Task.ScanLibrary -> {
    libraryRepository.findByIdOrNull(task.libraryId)?.let { library ->
        val scanResult = task.scanResultPath?.let { path ->
            objectMapper.readValue(Paths.get(path).toFile(), ScanResult::class.java)
        }
        libraryContentLifecycle.scanRootFolder(library, task.scanDeep, scanResult)
    }
}
```

```kotlin
// Modified LibraryContentLifecycle
fun scanRootFolder(library: Library, scanDeep: Boolean, externalResult: ScanResult? = null) {
    val scanResult = externalResult ?: fileSystemScanner.scanRootFolder(...)
    // ... rest of reconciliation unchanged
}
```

### Highest ROI Fixes (before building external tool)

1. **Increase `taskPoolSize`** → allows fan-out tasks to parallelize
2. **Disable `hashFiles`, `hashPages`, `hashKoreader`** if not needed → eliminates 100K+ HashBook tasks
3. **Disable `repairExtensions`, `convertToCbz`** if not needed → eliminates conversion queries
4. **Use `scanDeep = false`** for periodic scans → only checks `fileLastModified`
5. **Set `scanInterval = DISABLED` + trigger scans manually** → eliminates periodic scans entirely

These changes address the **reconciliation + fan-out** side of the equation, which is often the bigger bottleneck than the filesystem walk alone.
