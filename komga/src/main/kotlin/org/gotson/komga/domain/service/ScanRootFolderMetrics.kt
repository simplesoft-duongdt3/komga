package org.gotson.komga.domain.service

/**
 * Metrics collected during a single [LibraryContentLifecycle.scanRootFolder] execution.
 *
 * Contains counters for all scanned/created/deleted series and books,
 * plus phase-level timing breakdowns in milliseconds.
 */
data class ScanRootFolderMetrics(
  var scannedSeries: Int = 0,
  var scannedBooks: Int = 0,
  var scannedSidecars: Int = 0,
  var existingSeries: Int = 0,
  var existingScannedSeries: Int = 0,
  var preloadedBooks: Int = 0,
  var deletedSeries: Int = 0,
  var deletedBooks: Int = 0,
  var createdSeries: Int = 0,
  var updatedSeries: Int = 0,
  var addedBooks: Int = 0,
  var deferredHashBooks: Int = 0,
  var outdatedBooks: Int = 0,
  var seriesRefreshQueued: Int = 0,
  var changedSidecars: Int = 0,
  var deletedSidecars: Int = 0,

  // Fan-out counters (Phase 2) — populated by TaskHandler
  var analyzeBookCount: Int = 0,
  var hashBookCount: Int = 0,
  var hashBookKoreaderCount: Int = 0,
  var repairExtensionCount: Int = 0,

  var filesystemScanMs: Long = 0,
  var clearUnavailableMs: Long = 0,
  var loadExistingMs: Long = 0,
  var deleteMissingSeriesMs: Long = 0,
  var deleteMissingBooksMs: Long = 0,
  var reconcileSeriesBooksMs: Long = 0,
  var sortAndRefreshMs: Long = 0,
  var reconcileSidecarsMs: Long = 0,
  var cleanupSidecarsMs: Long = 0,
  var cleanupMs: Long = 0,
  var totalMs: Long = 0,
)
