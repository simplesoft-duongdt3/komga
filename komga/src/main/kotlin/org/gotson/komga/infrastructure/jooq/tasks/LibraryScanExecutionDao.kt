package org.gotson.komga.infrastructure.jooq.tasks

import org.gotson.komga.application.tasks.LibraryScanExecution
import org.gotson.komga.application.tasks.LibraryScanExecutionRepository
import org.gotson.komga.infrastructure.datasource.DatabaseType
import org.gotson.komga.infrastructure.jooq.SplitDslDaoBase
import org.jooq.DSLContext
import org.jooq.Field
import org.jooq.impl.DSL
import org.springframework.beans.factory.annotation.Qualifier
import org.springframework.beans.factory.annotation.Value
import org.springframework.context.annotation.DependsOn
import org.springframework.stereotype.Component
import java.time.LocalDateTime

@Component
@DependsOn("flywaySecondaryMigrationInitializer")
class LibraryScanExecutionDao(
  @Qualifier("tasksDslContextRW") dslRW: DSLContext,
  @Qualifier("tasksDslContextRO") dslRO: DSLContext,
  @param:Value("#{@komgaProperties.tasksDb.type}") private val databaseType: DatabaseType,
) : SplitDslDaoBase(dslRW, dslRO),
  LibraryScanExecutionRepository {

  private val tableName = if (databaseType == DatabaseType.POSTGRESQL) "\"LIBRARY_SCAN_EXECUTION\"" else "LIBRARY_SCAN_EXECUTION"
  private val t = DSL.table(tableName)

  private val cId = col("ID", String::class.java)
  private val cTaskExecutionId = col("TASK_EXECUTION_ID", String::class.java)
  private val cLibraryId = col("LIBRARY_ID", String::class.java)
  private val cScanDeep = col("SCAN_DEEP", Any::class.java)
  private val cStartDate = col("START_DATE", LocalDateTime::class.java)
  private val cEndDate = col("END_DATE", LocalDateTime::class.java)

  // Counters
  private val cScannedSeries = col("SCANNED_SERIES", Int::class.java)
  private val cScannedBooks = col("SCANNED_BOOKS", Int::class.java)
  private val cScannedSidecars = col("SCANNED_SIDECARS", Int::class.java)
  private val cExistingSeries = col("EXISTING_SERIES", Int::class.java)
  private val cExistingScannedSeries = col("EXISTING_SCANNED_SERIES", Int::class.java)
  private val cPreloadedBooks = col("PRELOADED_BOOKS", Int::class.java)
  private val cDeletedSeries = col("DELETED_SERIES", Int::class.java)
  private val cDeletedBooks = col("DELETED_BOOKS", Int::class.java)
  private val cCreatedSeries = col("CREATED_SERIES", Int::class.java)
  private val cUpdatedSeries = col("UPDATED_SERIES", Int::class.java)
  private val cAddedBooks = col("ADDED_BOOKS", Int::class.java)
  private val cDeferredHashBooks = col("DEFERRED_HASH_BOOKS", Int::class.java)
  private val cOutdatedBooks = col("OUTDATED_BOOKS", Int::class.java)
  private val cSeriesRefreshQueued = col("SERIES_REFRESH_QUEUED", Int::class.java)
  private val cChangedSidecars = col("CHANGED_SIDECARS", Int::class.java)
  private val cDeletedSidecars = col("DELETED_SIDECARS", Int::class.java)

  // Fan-out counters
  private val cAnalyzeBookCount = col("ANALYZE_BOOK_COUNT", Int::class.java)
  private val cHashBookCount = col("HASH_BOOK_COUNT", Int::class.java)
  private val cHashBookKoreaderCount = col("HASH_BOOK_KOREADER_COUNT", Int::class.java)
  private val cRepairExtensionCount = col("REPAIR_EXTENSION_COUNT", Int::class.java)

  // Phase timings
  private val cTotalMs = col("TOTAL_MS", Long::class.java)
  private val cFilesystemScanMs = col("FILESYSTEM_SCAN_MS", Long::class.java)
  private val cClearUnavailableMs = col("CLEAR_UNAVAILABLE_MS", Long::class.java)
  private val cLoadExistingMs = col("LOAD_EXISTING_MS", Long::class.java)
  private val cDeleteMissingSeriesMs = col("DELETE_MISSING_SERIES_MS", Long::class.java)
  private val cDeleteMissingBooksMs = col("DELETE_MISSING_BOOKS_MS", Long::class.java)
  private val cReconcileSeriesBooksMs = col("RECONCILE_SERIES_BOOKS_MS", Long::class.java)
  private val cSortAndRefreshMs = col("SORT_AND_REFRESH_MS", Long::class.java)
  private val cReconcileSidecarsMs = col("RECONCILE_SIDECARS_MS", Long::class.java)
  private val cCleanupSidecarsMs = col("CLEANUP_SIDECARS_MS", Long::class.java)
  private val cCleanupMs = col("CLEANUP_MS", Long::class.java)

  // Error tracking
  private val cSuccess = col("SUCCESS", Any::class.java)
  private val cErrorMessage = col("ERROR_MESSAGE", String::class.java)

  private val allFields = arrayOf(
    cId, cTaskExecutionId, cLibraryId, cScanDeep, cStartDate, cEndDate,
    cScannedSeries, cScannedBooks, cScannedSidecars,
    cExistingSeries, cExistingScannedSeries, cPreloadedBooks,
    cDeletedSeries, cDeletedBooks,
    cCreatedSeries, cUpdatedSeries, cAddedBooks,
    cDeferredHashBooks, cOutdatedBooks, cSeriesRefreshQueued,
    cChangedSidecars, cDeletedSidecars,
    cAnalyzeBookCount, cHashBookCount, cHashBookKoreaderCount, cRepairExtensionCount,
    cTotalMs,
    cFilesystemScanMs, cClearUnavailableMs, cLoadExistingMs,
    cDeleteMissingSeriesMs, cDeleteMissingBooksMs,
    cReconcileSeriesBooksMs, cSortAndRefreshMs,
    cReconcileSidecarsMs, cCleanupSidecarsMs, cCleanupMs,
    cSuccess, cErrorMessage,
  )

  private inline fun <reified T> col(name: String, type: Class<T>) =
    if (databaseType == DatabaseType.POSTGRESQL) DSL.field("\"$name\"", type) else DSL.field(name, type)

  private val trueValue: Any = if (databaseType == DatabaseType.POSTGRESQL) true else 1
  private val falseValue: Any = if (databaseType == DatabaseType.POSTGRESQL) false else 0

  override fun save(execution: LibraryScanExecution) {
    dslRW
      .insertInto(t, *allFields)
      .values(
        execution.id,
        execution.taskExecutionId,
        execution.libraryId,
        if (databaseType == DatabaseType.SQLITE) (if (execution.scanDeep) 1 else 0) else execution.scanDeep,
        execution.startDate,
        execution.endDate,
        execution.scannedSeries,
        execution.scannedBooks,
        execution.scannedSidecars,
        execution.existingSeries,
        execution.existingScannedSeries,
        execution.preloadedBooks,
        execution.deletedSeries,
        execution.deletedBooks,
        execution.createdSeries,
        execution.updatedSeries,
        execution.addedBooks,
        execution.deferredHashBooks,
        execution.outdatedBooks,
        execution.seriesRefreshQueued,
        execution.changedSidecars,
        execution.deletedSidecars,
        execution.analyzeBookCount,
        execution.hashBookCount,
        execution.hashBookKoreaderCount,
        execution.repairExtensionCount,
        execution.totalMs,
        execution.filesystemScanMs,
        execution.clearUnavailableMs,
        execution.loadExistingMs,
        execution.deleteMissingSeriesMs,
        execution.deleteMissingBooksMs,
        execution.reconcileSeriesBooksMs,
        execution.sortAndRefreshMs,
        execution.reconcileSidecarsMs,
        execution.cleanupSidecarsMs,
        execution.cleanupMs,
        if (databaseType == DatabaseType.SQLITE) (if (execution.success) 1 else 0) else execution.success,
        execution.errorMessage,
      ).execute()
  }
}
