package org.gotson.komga.application.tasks

import java.time.LocalDateTime

data class LibraryScanExecution(
  val id: String,
  val taskExecutionId: String,
  val libraryId: String,
  val scanDeep: Boolean = false,
  val startDate: LocalDateTime,
  val endDate: LocalDateTime? = null,

  // Counters
  val scannedSeries: Int = 0,
  val scannedBooks: Int = 0,
  val scannedSidecars: Int = 0,
  val existingSeries: Int = 0,
  val existingScannedSeries: Int = 0,
  val preloadedBooks: Int = 0,
  val deletedSeries: Int = 0,
  val deletedBooks: Int = 0,
  val createdSeries: Int = 0,
  val updatedSeries: Int = 0,
  val addedBooks: Int = 0,
  val deferredHashBooks: Int = 0,
  val outdatedBooks: Int = 0,
  val seriesRefreshQueued: Int = 0,
  val changedSidecars: Int = 0,
  val deletedSidecars: Int = 0,

  // Fan-out counters (Phase 2)
  val analyzeBookCount: Int = 0,
  val hashBookCount: Int = 0,
  val hashBookKoreaderCount: Int = 0,
  val repairExtensionCount: Int = 0,

  // Phase timings (ms)
  val totalMs: Long = 0,
  val filesystemScanMs: Long = 0,
  val clearUnavailableMs: Long = 0,
  val loadExistingMs: Long = 0,
  val deleteMissingSeriesMs: Long = 0,
  val deleteMissingBooksMs: Long = 0,
  val reconcileSeriesBooksMs: Long = 0,
  val sortAndRefreshMs: Long = 0,
  val reconcileSidecarsMs: Long = 0,
  val cleanupSidecarsMs: Long = 0,
  val cleanupMs: Long = 0,

  // Error tracking
  val success: Boolean = true,
  val errorMessage: String? = null,
)
