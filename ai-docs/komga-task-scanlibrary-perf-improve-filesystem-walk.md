# Improving ScanLibrary Performance: Filesystem Walk (Phase 1.1)

## Problem Statement

The filesystem walk in `FileSystemScanner.scanRootFolder()` using `Files.walkFileTree()` is the #1 bottleneck for libraries on network filesystems (NFS/SMB). It is:

- **Single-threaded** — a single JVM thread walks the entire tree sequentially
- **I/O-bound** — every file/directory requires a `stat`-equivalent syscall to read `BasicFileAttributes`
- **Amplified by NFS/SMB** — each syscall becomes a network round-trip. On NFS with ~1ms latency, 100,000 files → 100 seconds of pure latency. Real-world NFS is often 5-50ms, yielding 500-5000 seconds.
- **O(n) in total files** — must visit every file and directory, even when only 5 files changed

## Current Implementation

```kotlin
Files.walkFileTree(
  root,
  setOf(FileVisitOption.FOLLOW_LINKS),
  Integer.MAX_VALUE,  // unlimited depth
  object : FileVisitor<Path> {
    override fun preVisitDirectory(dir: Path, attrs: BasicFileAttributes) {
      // Read attrs (syscall already done by walkFileTree)
      // Check directory exclusion rules
      // Create Series(name, url, fileLastModified)
    }
    override fun visitFile(file: Path, attrs: BasicFileAttributes) {
      // Read attrs (syscall already done by walkFileTree)
      // Match extension against [cbz,zip,cbr,rar,pdf,epub]
      // Check file exclusions (leading dot)
      // Create Book(name, url, fileLastModified, fileSize)
      // Match series-level sidecar consumers
      // Match book-level sidecar by regex prefilter
    }
    override fun postVisitDirectory(dir: Path, exc: IOException?) {
      // Assign books to series
      // Handle oneshots directory
      // Match book sidecars with actual book names
    }
  }
)
```

**Key observation**: `walkFileTree` calls `BasicFileAttributes` on every entry (directory + file). This attribute fetch is the I/O operation that's slow on NFS. The `FileVisitor` callbacks themselves are pure in-memory CPU work — they are fast. **The bottleneck is the `stat` equivalent per entry, not the Java code.**

---

## Solution 1: Parallel Directory Walk Using `Files.walk()` + `parallelStream()` (Low Risk, Fast Win)

### Concept

Replace `Files.walkFileTree` (sequential `FileVisitor`) with `Files.walk()` (lazy stream) + `parallelStream()` (ForkJoinPool). The walk itself is still sequential, but attribute reading and object creation are parallelized.

### Code

```kotlin
import kotlin.streams.asSequence
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ConcurrentLinkedQueue

fun scanRootFolderParallel(
  root: Path,
  forceDirectoryModifiedTime: Boolean = false,
  oneshotsDir: String? = null,
  scanCbx: Boolean = true,
  scanPdf: Boolean = true,
  scanEpub: Boolean = true,
  directoryExclusions: Set<String> = emptySet(),
): ScanResult {
  val scanForExtensions = buildList {
    if (scanCbx) addAll(listOf("cbz", "zip", "cbr", "rar"))
    if (scanPdf) add("pdf")
    if (scanEpub) add("epub")
  }

  // Thread-safe collections
  val pathToSeries = ConcurrentHashMap<Path, Series>()
  val pathToBooks = ConcurrentHashMap<Path, ConcurrentLinkedQueue<Book>>()
  val scannedSeries = ConcurrentHashMap<Series, List<Book>>()
  val scannedSidecars = ConcurrentLinkedQueue<Sidecar>()
  val pathToSeriesSidecars = ConcurrentHashMap<Path, ConcurrentLinkedQueue<Sidecar>>()
  val pathToBookSidecars = ConcurrentHashMap<Path, ConcurrentLinkedQueue<TempSidecar>>()

  // Use Files.walk() which returns a lazy Stream<Path>
  // parallel() spreads the work across ForkJoinPool.commonPool()
  Files.walk(root, FileVisitOption.FOLLOW_LINKS)
    .parallel()
    .filter { path ->
      // Quick exclusion check before reading attributes (fast, no I/O for name)
      val excludeDir = directoryExclusions.any { path.pathString.contains(it, true) }
      !excludeDir && !path.name.startsWith(".")
    }
    .forEach { path ->
      val attrs = try {
        Files.readAttributes(path, BasicFileAttributes::class.java)
      } catch (e: Exception) {
        logger.warn { "Could not read attributes for: $path" }
        return@forEach
      }

      if (attrs.isDirectory) {
        pathToSeries[path] = Series(
          name = path.name.ifBlank { path.pathString },
          url = path.toUri().toURL(),
          fileLastModified = attrs.getUpdatedTime(),
        )
      } else if (attrs.isRegularFile && !attrs.isSymbolicLink) {
        val ext = path.extension.lowercase()
        if (scanForExtensions.contains(ext)) {
          val book = Book(
            name = path.nameWithoutExtension,
            url = path.toUri().toURL(),
            fileLastModified = attrs.getUpdatedTime(),
            fileSize = attrs.size(),
          )
          pathToBooks.compute(path.parent) { _, list ->
            (list ?: ConcurrentLinkedQueue()).also { it.add(book) }
          }
        }

        // Sidecar matching (same as original)
        sidecarSeriesConsumers.firstOrNull { consumer ->
          consumer.getSidecarSeriesFilenames().any { path.name.equals(it, ignoreCase = true) }
        }?.let {
          val sidecar = Sidecar(path.toUri().toURL(), path.parent.toUri().toURL(),
            attrs.getUpdatedTime(), it.getSidecarSeriesType(), Sidecar.Source.SERIES)
          pathToSeriesSidecars.compute(path.parent) { _, list ->
            (list ?: ConcurrentLinkedQueue()).also { it.add(sidecar) }
          }
        }

        if (sidecarBookPrefilter.any { it.matches(path.name) }) {
          val tempSidecar = TempSidecar(path.name, path.toUri().toURL(), attrs.getUpdatedTime())
          pathToBookSidecars.compute(path.parent) { _, list ->
            (list ?: ConcurrentLinkedQueue()).also { it.add(tempSidecar) }
          }
        }
      }
    }

  // Post-processing (same as original postVisitDirectory logic, but on the collected maps)
  // This must be single-threaded because it matches book sidecars to books
  pathToBooks.forEach { (dir, books) ->
    val tempSeries = pathToSeries[dir] ?: return@forEach
    if (books.isEmpty()) return@forEach

    val seriesList = if (!oneshotsDir.isNullOrBlank() && dir.pathString.contains(oneshotsDir, true)) {
      books.map { book ->
        Series(name = book.name, url = book.url, fileLastModified = book.fileLastModified, oneshot = true) to listOf(book.copy(oneshot = true))
      }
    } else {
      val series = if (forceDirectoryModifiedTime)
        tempSeries.copy(fileLastModified = maxOf(tempSeries.fileLastModified, books.maxOf { it.fileLastModified }))
      else tempSeries

      pathToSeriesSidecars[dir]?.forEach { scannedSidecars.add(it) }
      listOf(series to books.toList())
    }

    // Match book sidecars (same logic as original)
    books.forEach { book ->
      pathToBookSidecars[dir]?.forEach { tempSidecar ->
        sidecarBookConsumers.firstOrNull { it.isSidecarBookMatch(book.name, tempSidecar.name) }
          ?.let { scannedSidecars.add(Sidecar(tempSidecar.url, book.url, tempSidecar.lastModifiedTime, it.getSidecarBookType(), Sidecar.Source.BOOK)) }
      }
    }

    seriesList.forEach { (series, bookList) -> scannedSeries[series] = bookList }
  }

  return ScanResult(scannedSeries, scannedSidecars.toList())
}
```

### Performance Analysis

| Factor | `walkFileTree` (sequential) | `walk().parallelStream()` |
|--------|---------------------------|--------------------------|
| Threads | 1 | `Runtime.getRuntime().availableProcessors()` (e.g., 8-16) |
| Attribute reads | 1 at a time, blocking | Up to N in parallel |
| NFS latency wall | 100% latency (1 thread × 100ms × 100K files) | Latency divided by core count (8 threads × 100ms × 12.5K files each) |
| CPU work | N/A (I/O bound) | Minimal overhead from thread coordination |
| Memory | Low (accumulates in maps) | Slightly higher (ConcurrentHashMap overhead) |

**Expected improvement on NFS**: 3-8x (limited by NFS server throughput, not client threads).

**Expected improvement on local SSD**: ~1x (already fast — `walkFileTree` is CPU-bound by nothing; adding parallelism doesn't help I/O that completes in microseconds).

### Risks

1. **`ConcurrentHashMap` memory overhead** — slightly higher than `HashMap`
2. **ForkJoinPool saturation** — `commonPool()` is shared with other parallel streams in the JVM. Use a dedicated pool via `ForkJoinPool(nThreads) { ... }` to isolate.
3. **Thread safety of sidecar consumer lambdas** — `isSidecarBookMatch()` must be stateless (currently it is).

---

## Solution 2: Dedicated `ForkJoinPool` for Scan (Low Risk, Best Practice)

Wraps solution 1 with an isolated thread pool:

```kotlin
class FileSystemScanner(
  private val sidecarBookConsumers: List<SidecarBookConsumer>,
  private val sidecarSeriesConsumers: List<SidecarSeriesConsumer>,
  private val komgaProperties: KomgaProperties,  // NEW: inject config
) {
  // Dedicated pool for filesystem scanning — isolates from other parallel streams
  private val scanPool: ForkJoinPool by lazy {
    ForkJoinPool(komgaProperties.filesystemScannerParallelism)
  }

  fun scanRootFolder(/*...*/): ScanResult {
    // ...
    scanPool.submit {
      Files.walk(root, FileVisitOption.FOLLOW_LINKS)
        .parallel()
        .filter { /* exclusions */ }
        .forEach { path ->
          // ... rest of solution 1 logic
        }
    }.get() // block until completed
    // ... post-processing
  }
}
```

**Configuration:**

```kotlin
// KomgaProperties.kt
class KomgaProperties {
  // ...
  var filesystemScannerParallelism: Int = Runtime.getRuntime().availableProcessors()
    get() = if (field < 2) 2 else field  // minimum 2
}
```

---

## Solution 3: Use `java.nio.file.DirectoryStream` with `readAttributes` Batched (Medium Risk, Complex)

### Concept

`Files.walkFileTree` reads attributes one-at-a-time. On NFS, each read is a separate RPC. Some filesystems (NFSv4, SMB3) support batched metadata reads — but Java's `FileVisitor` doesn't use them.

Instead, use `DirectoryStream` at each directory level and batch attribute reads:

```kotlin
// Pseudo-code — would need JNI/JNR for true batched stat
fun scanDirectoryConcurrently(root: Path): ScanResult {
  // Phase 1: BFS to collect all paths (sequential, fast)
  val allPaths = Files.walk(root).toList()

  // Phase 2: Batch attribute reads in parallel (the bottleneck)
  val attrsMap = ConcurrentHashMap<Path, BasicFileAttributes>()
  allPaths.parMap { path ->  // hypothetical parallel map
    attrsMap[path] = Files.readAttributes(path, BasicFileAttributes::class.java)
  }

  // Phase 3: Process in-memory (fast)
  // ... build ScanResult from attrsMap
}
```

**However**: Java does not expose a batched `stat` syscall API. `Files.readAttributes` is always one-at-a-time. True batched stat requires:

1. **JNI/JNR** calling `getattr()` batching on NFS
2. **Native tool** (Go/Rust) that calls `statx()`, `getdents()` + batch stat, or uses `inotify`

This approach overlaps with the external tool proposal.

---

## Solution 4: `find` Command via `ProcessBuilder` (High Risk, Non-portable)

### Concept

Shell out to the OS `find` command, which is highly optimized C code with parallel directory traversal support (GNU `find` uses `fts`, which is faster than Java's `walkFileTree`).

```kotlin
fun scanWithFind(root: Path): ScanResult {
  val process = ProcessBuilder(
    "find", root.toString(),
    "-type", "f",
    "(", "-name", "*.cbz", "-o", "-name", "*.zip", "-o", "-name", "*.cbr",
    "-o", "-name", "*.rar", "-o", "-name", "*.pdf", "-o", "-name", "*.epub", ")",
    "-printf", "%h|%f|%s|%T@\\n"  // dir|filename|size|mtime
  ).start()

  val output = process.inputStream.bufferedReader().readText()
  // Parse output into ScanResult...
}
```

### Pros/Cons

| Factor | Rating |
|--------|--------|
| Speed | Very fast (`find` is highly optimized C) |
| Portability | ❌ Linux-only (`-printf` flag; macOS `find` uses different syntax) |
| Security | ⚠️ Path injection risk (library root path parsed into command) |
| Sidecars | ⚠️ Must implement sidecar matching separately |
| Error handling | ⚠️ Exit codes, stderr parsing needed |
| Windows | ❌ No native `find` equivalent |

**Not recommended** for a production Java application.

---

## Solution 5: Per-Series Parallel Walk (Medium Risk, Architectural Change)

### Concept

Instead of walking the entire library tree in one task, **split the walk into per-series walks** that run as individual tasks in the thread pool:

```kotlin
// Step 1: Discover series directories only (fast — 1 stat per directory)
// Uses DirectoryStream at root level only
class DiscoverSeries(
  val libraryId: String,
  val root: URL,
) : Task(HIGHEST_PRIORITY) {
  override val uniqueId = "DISCOVER_SERIES_$libraryId"
}

// Handler for DiscoverSeries
is Task.DiscoverSeries -> {
  val root = Paths.get(task.root.toURI())
  val seriesDirs = Files.list(root).filter { Files.isDirectory(it) }.toList()
  seriesDirs.forEach { dir ->
    taskEmitter.scanSingleSeries(libraryId, dir)
  }
}

// Step 2: Walk each series directory in parallel via the task thread pool
class ScanSingleSeries(
  val libraryId: String,
  val seriesDir: String,
) : Task {
  override val uniqueId = "SCAN_SERIES_${libraryId}_$seriesDir"
}
```

### Performance

If `taskPoolSize = 8`:
- 10,000 series → 10,000 `ScanSingleSeries` tasks in queue
- 8 run concurrently (each walks 1 series directory)
- Sequential walk of 1 series directory = ~1ms on SSD, ~50ms on NFS
- 10,000 series × 50ms / 8 threads = **~62 seconds** vs 3 hours

### Pros/Cons

| Factor | Rating |
|--------|--------|
| Speed | Excellent — leverages existing thread pool for parallelism |
| Implementation | ⚠️ Requires splitting `scanRootFolder` into 2 phases |
| Sidecars | ⚠️ Per-series sidecar matching is straightforward |
| Restoration | ⚠️ `tryRestoreSeries` becomes concurrent — needs locking (already has `Semaphore` cache) |
| Series creation lock | ✅ Already exists (`seriesCreationLockCache`) — covers concurrent creation |
| DB isolation | ⚠️ Transaction boundaries need careful review |

---

## Solution 6: Incremental Walk via Cached Directory Listings (Low Risk, Large Win for Repeat Scans)

### Concept

Current scans don't reuse any state from previous scans. They walk everything every time. Most filesystems don't change between scans — cache the directory listing and only re-read files whose parent directory's `fileLastModified` changed.

### The Most Important Insight

**For periodic scans** (`scanDeep = false`), the ONLY thing needed is to detect whether anything changed. The current code still walks every file to discover this. But `Files.walkFileTree` already reads directory `fileLastModified` in `preVisitDirectory` — we can use that timestamp to decide whether to skip entire subtrees.

### Code

```kotlin
// Cached directory timestamps (persisted in DB or a local file)
data class DirectoryMetadata(
  val path: String,
  val lastModified: LocalDateTime,
  val fileCount: Int,
)

class FileSystemScanner {
  // In-memory cache of previous directory metadata
  // Populated from DB or JSON file on startup
  private val previousDirectoryMetadata: Map<String, DirectoryMetadata> = loadFromStorage()

  fun scanRootFolder(/*...*/): ScanResult {
    // Only walk directories whose modified time changed
    Files.walkFileTree(root, /*...*/, object : FileVisitor<Path> {
      override fun preVisitDirectory(dir: Path, attrs: BasicFileAttributes): FileVisitResult {
        val dirPath = dir.toString()
        val prev = previousDirectoryMetadata[dirPath]
        val currentModified = attrs.getUpdatedTime()

        if (prev != null &&
          prev.lastModified == currentModified &&
          !forceDirectoryModifiedTime) {
          // Directory unchanged — skip subtree entirely!
          // But we still need the count of files to verify...
          // Actually, we can't skip because files inside might have changed
          // while directory mtime stayed the same (rare but possible)
          // Safe approach: skip ONLY if scanDeep=false AND mtime matches AND file count matches
          return FileVisitResult.SKIP_SUBTREE
        }
        return FileVisitResult.CONTINUE
      }
      // ... rest unchanged
    })

    // Update cache with new metadata
    saveToStorage(newDirectoryMetadata)
  }
}
```

**Why this works in practice**: Most filesystem operations (add, delete, rename a file) update the parent directory's `mtime`/`ctime`. On NFS, this is reliably propagated. The exception is in-place file content changes (same file, same name, new bytes) — but these are rare for comic libraries where CBZ/RAR/PDF files are typically added, replaced, or deleted, not modified in-place.

**Quantified impact for a typical periodic scan:**
- 10,000 directories total
- 5 directories changed since last scan
- Walk 5 directories (~50 files) instead of 10,000 directories (~100,000 files)
- **Filesystem walk time: 3 hours → ~1 second**

### Storage for Directory Metadata

The cache can be stored as:

**Option A: JSON file** (simplest)
```json
{
  "/comics/series1": {"lastModified": "2026-05-10T10:00:00", "fileCount": 12},
  "/comics/series2": {"lastModified": "2026-05-09T08:00:00", "fileCount": 8}
}
```
Stored at `data/scans/library-{id}-directory-cache.json`.

**Option B: Database table** (permanent)
```sql
CREATE TABLE SCAN_DIRECTORY_CACHE (
  library_id   varchar NOT NULL,
  path         varchar NOT NULL,
  last_modified timestamp NOT NULL,
  file_count   int NOT NULL,
  PRIMARY KEY (library_id, path)
);
```

---

## Solution 7: External Native Tool + Diff Input (Highest Impact, Highest Effort)

This is the previously discussed approach. Summarized here for completeness:

1. A Go/Rust daemon watches the library root using `inotify` (Linux) or `kqueue` (macOS)
2. On first run: walks the entire tree (fast in native code) and produces a JSON diff
3. On subsequent runs: receives inotify events and produces incremental diffs
4. Komga `ScanLibrary` task receives the diff and only reconciles changed items

**Expected outcome**: Scans become near-instant for any library size, with <1 second detection-to-reconciliation for changes.

---

## Comparison Table

| Solution | Effort | Risk | On NFS Speedup | On SSD Speedup | Incremental? | Notes |
|----------|--------|------|---------------|---------------|-------------|-------|
| **1. `walk().parallelStream()`** | Low | Low | 3-8x | 0-1.2x | No | Simplest win. Change `walkFileTree` → `walk().parallel()`. |
| **2. Dedicated ForkJoinPool** | Low | Low | Same as #1 | Same | No | Wraps #1 with isolated pool. Best practice. |
| **3. Batched stat (JNI/JNR)** | Very High | High | 10-50x* | 1-2x* | No | Theoretical. No Java API exists. |
| **4. `find` command** | Medium | High | 3-5x | 1-2x | No | Non-portable, security risk. Not recommended. |
| **5. Per-series tasks** | High | Medium | 10-50x | 3-5x | No | Most architectural change. Leverages existing thread pool. |
| **6. Directory mtime cache** | Medium | Medium | 100-10000x† | 10-100x† | ✅ Yes | Huge win for periodic scans. Requires cache storage + staleness handling. |
| **7. External native tool** | Very High | Medium | 100-10000x† | 10-100x† | ✅ Yes | Ultimate solution. Requires separate daemon + diff protocol. |

\* Theoretical — not implementable in pure Java without native code  
† For periodic scans with few changes. First scan is comparable to current.

---

## Recommendation

### Immediate (1-2 days engineering effort)

**Implement Solution 1 + 2**: `walk().parallel()` with dedicated `ForkJoinPool`.

```kotlin
// In KomgaProperties:
var filesystemScannerParallelism: Int = 4
```

```kotlin
// In FileSystemScanner:
private val scanPool = lazy { ForkJoinPool(komgaProperties.filesystemScannerParallelism) }
```

**Implementation plan:**
1. Add `filesystemScannerParallelism` to `KomgaProperties` (default: min(4, availableProcessors))
2. Create `scanPool: ForkJoinPool` in `FileSystemScanner` constructor
3. Replace `Files.walkFileTree` with `scanPool.submit { Files.walk(...).parallel().forEach { ... } }.get()`
4. Replace `HashMap` / `MutableList` with `ConcurrentHashMap` / `ConcurrentLinkedQueue` equivalents
5. Keep `postVisitDirectory` logic as a single-threaded post-processing step (sidecar matching is not trivially parallelizable at the directory level)

**Expected gain on NFS**: 3-4x reduction in `filesystemScanMs`.

### Short-term (1 week engineering effort)

**Implement Solution 6**: Directory mtime cache.

This is the most impactful single change for periodic scans. The logic is:

```
if (scanDeep == false && directory.cached_mtime == directory.current_mtime) {
    SKIP_SUBTREE  // entire directory tree unchanged
}
```

**Implementation plan:**
1. Define `ScanDirectoryCache` entity/table or JSON file
2. In `scanRootFolder`, on entry to `preVisitDirectory`: check cache
3. On exit from `scanRootFolder`: update cache with new directory metadata
4. Handle first scan (no cache → walk everything)
5. Handle cache staleness (provide CLI command or API to invalidate)

**Expected gain on NFS**: 100-1000x reduction in `filesystemScanMs` for periodic scans (3 hours → 10-60 seconds).

### Long-term (2-4 weeks engineering effort)

**Implement Solution 7**: External native tool + diff-based scanning.

This is the architectural solution that eliminates the filesystem scanning bottleneck entirely. Design the diff format as specified in `komga-task-scanlibrary-diff-input-analysis.md`.

---

## Appendix: Profiling the Current Walk

Before implementing any solution, confirm the filesystem walk is the bottleneck:

### Via existing logs

```
scanRootFolder phase=filesystem_scan status=ok scanId=xxx libraryId=yyy durationMs=3000000 series=5000 books=50000
```
Check `durationMs` for `phase=filesystem_scan`. If it's >50% of total, the walk is the bottleneck.

### Via Java Flight Recorder (JFR)

```bash
# Add to JVM options:
-XX:StartFlightRecording=filename=scan.jfr,delay=10s,duration=300s
```

In JFR, look for:
- `java.nio.file.Files.walkFileTree` — wall clock time
- `jdk.FileRead` / `jdk.FileWrite` events — I/O time
- Thread state: `SLEEPING` (network I/O) vs `RUNNABLE` (CPU)

### Via strace (Linux)

```bash
# Record syscalls during scan:
strace -c -p <komga-pid> -e trace=newfstatat,stat,openat,getdents64
```

High `newfstatat` count + high elapsed time confirms the syscall-per-file bottleneck.

---

## Summary

| Priority | Solution | Effort | Gain (NFS periodic) | Key Implementation |
|----------|----------|--------|-------------------|-------------------|
| P0 | **#6 Directory mtime cache** | 1 week | 100-10,000x | Cache directory timestamps; skip unchanged subtrees |
| P1 | **#1 + #2 Parallel walk** | 2 days | 3-8x | Replace `walkFileTree` with `walk().parallel()` + `ForkJoinPool` |
| P2 | **#7 External native tool** | 2-4 weeks | 100-10,000x | Go/Rust daemon + diff protocol |

For a library scanning in 30 hours on NFS:

- **P0 alone**: 30 hours → 10-60 seconds for most periodic scans, ~1-2 hours for first scan
- **P0 + P1**: 30 hours → 5-30 seconds for periodic scans, ~30 min for first scan
- **P0 + P1 + P7**: 30 hours → <1 second for incremental changes
