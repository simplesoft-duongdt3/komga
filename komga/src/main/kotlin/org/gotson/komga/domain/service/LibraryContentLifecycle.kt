package org.gotson.komga.domain.service

import io.github.oshai.kotlinlogging.KotlinLogging
import org.gotson.komga.application.tasks.TaskEmitter
import org.gotson.komga.domain.model.Book
import org.gotson.komga.domain.model.BookMetadataPatchCapability
import org.gotson.komga.domain.model.DirectoryNotFoundException
import org.gotson.komga.domain.model.DomainEvent
import org.gotson.komga.domain.model.Library
import org.gotson.komga.domain.model.Media
import org.gotson.komga.domain.model.SearchCondition
import org.gotson.komga.domain.model.SearchContext
import org.gotson.komga.domain.model.SearchOperator
import org.gotson.komga.domain.model.Series
import org.gotson.komga.domain.model.Sidecar
import org.gotson.komga.domain.model.SidecarStored
import org.gotson.komga.domain.model.ThumbnailBook
import org.gotson.komga.domain.model.ThumbnailSeries
import org.gotson.komga.domain.persistence.BookMetadataRepository
import org.gotson.komga.domain.persistence.BookRepository
import org.gotson.komga.domain.persistence.LibraryRepository
import org.gotson.komga.domain.persistence.MediaRepository
import org.gotson.komga.domain.persistence.ReadListRepository
import org.gotson.komga.domain.persistence.ReadProgressRepository
import org.gotson.komga.domain.persistence.SeriesCollectionRepository
import org.gotson.komga.domain.persistence.SeriesMetadataRepository
import org.gotson.komga.domain.persistence.SeriesRepository
import org.gotson.komga.domain.persistence.SidecarRepository
import org.gotson.komga.domain.persistence.ThumbnailBookRepository
import org.gotson.komga.domain.persistence.ThumbnailSeriesRepository
import org.gotson.komga.infrastructure.configuration.KomgaSettingsProvider
import org.gotson.komga.infrastructure.hash.Hasher
import org.gotson.komga.language.notEquals
import org.gotson.komga.language.toIndexedMap
import org.springframework.context.ApplicationEventPublisher
import org.springframework.data.domain.Pageable
import org.springframework.stereotype.Service
import org.springframework.transaction.support.TransactionTemplate
import java.net.URL
import java.nio.file.Paths
import java.time.LocalDateTime
import java.util.UUID

private val logger = KotlinLogging.logger {}

@Service
class LibraryContentLifecycle(
  private val fileSystemScanner: FileSystemScanner,
  private val seriesRepository: SeriesRepository,
  private val bookRepository: BookRepository,
  private val libraryRepository: LibraryRepository,
  private val bookLifecycle: BookLifecycle,
  private val mediaRepository: MediaRepository,
  private val seriesLifecycle: SeriesLifecycle,
  private val collectionLifecycle: SeriesCollectionLifecycle,
  private val readListLifecycle: ReadListLifecycle,
  private val sidecarRepository: SidecarRepository,
  private val komgaSettingsProvider: KomgaSettingsProvider,
  private val taskEmitter: TaskEmitter,
  private val transactionTemplate: TransactionTemplate,
  private val hasher: Hasher,
  private val bookMetadataRepository: BookMetadataRepository,
  private val seriesMetadataRepository: SeriesMetadataRepository,
  private val readListRepository: ReadListRepository,
  private val readProgressRepository: ReadProgressRepository,
  private val collectionRepository: SeriesCollectionRepository,
  private val thumbnailBookRepository: ThumbnailBookRepository,
  private val eventPublisher: ApplicationEventPublisher,
  private val thumbnailSeriesRepository: ThumbnailSeriesRepository,
) {
  fun scanRootFolder(
    library: Library,
    scanDeep: Boolean = false,
  ) {
    val scanId = UUID.randomUUID().toString().substring(0, 8)
    val metrics = ScanRootFolderMetrics()
    val totalStartNanos = System.nanoTime()

    logger.info { "scanRootFolder started scanId=$scanId libraryId=${library.id} scanDeep=$scanDeep root=${library.root}" }

    try {
      val (scanResult, filesystemScanMs) =
        logScanPhase(scanId, library, "filesystem_scan", details = { result ->
          val scannedBooks = result.series.values.sumOf { it.size }
          "series=${result.series.size} books=$scannedBooks sidecars=${result.sidecars.size}"
        }) {
          try {
            fileSystemScanner.scanRootFolder(
              Paths.get(library.root.toURI()),
              library.scanForceModifiedTime,
              library.oneshotsDirectory,
              library.scanCbx,
              library.scanPdf,
              library.scanEpub,
              library.scanDirectoryExclusions,
            )
          } catch (e: DirectoryNotFoundException) {
            library.copy(unavailableDate = LocalDateTime.now()).let {
              libraryRepository.update(it)
              eventPublisher.publishEvent(DomainEvent.LibraryUpdated(it))
            }
            throw e
          }
        }
      metrics.filesystemScanMs = filesystemScanMs
      metrics.scannedSeries = scanResult.series.size
      metrics.scannedBooks = scanResult.series.values.sumOf { it.size }
      metrics.scannedSidecars = scanResult.sidecars.size

      val (_, clearUnavailableMs) =
        logScanPhase(scanId, library, "clear_unavailable", details = { cleared ->
          "clearedUnavailable=$cleared"
        }) {
          if (library.unavailableDate != null) {
            library.copy(unavailableDate = null).let {
              libraryRepository.update(it)
              eventPublisher.publishEvent(DomainEvent.LibraryUpdated(it))
            }
            true
          } else {
            false
          }
        }
      metrics.clearUnavailableMs = clearUnavailableMs

      val scannedSeries =
        scanResult
          .series
          .map { (series, books) ->
            series.copy(libraryId = library.id) to books.map { it.copy(libraryId = library.id) }
          }.toMap()

      val (existingState, loadExistingMs) =
        logScanPhase(scanId, library, "load_existing", details = { state ->
          "existingSeries=${state.existingSeriesById.size} existingScannedSeries=${state.existingScannedSeriesIds.size} preloadedBooks=${state.existingBooksCount}"
        }) {
          val existingSeriesById =
            seriesRepository
              .findAllByLibraryId(library.id)
              .associateBy { it.id }
          val existingActiveSeriesByUrl =
            existingSeriesById.values
              .filter { it.deletedDate == null }
              .sortedByDescending { it.lastModifiedDate }
              .associateBy { it.url }
          val existingScannedSeriesIds =
            scannedSeries.keys
              .mapNotNull { existingActiveSeriesByUrl[it.url]?.id }
              .distinct()
          val existingBooksBySeriesId =
            if (existingScannedSeriesIds.isEmpty()) {
              emptyMap()
            } else {
              bookRepository.findAllBySeriesIds(existingScannedSeriesIds).groupBy { it.seriesId }
            }

          ExistingScanState(
            existingSeriesById = existingSeriesById,
            existingActiveSeriesByUrl = existingActiveSeriesByUrl,
            existingScannedSeriesIds = existingScannedSeriesIds,
            existingBooksBySeriesId = existingBooksBySeriesId,
          )
        }
      metrics.loadExistingMs = loadExistingMs
      metrics.existingSeries = existingState.existingSeriesById.size
      metrics.existingScannedSeries = existingState.existingScannedSeriesIds.size
      metrics.preloadedBooks = existingState.existingBooksCount

      val existingSeriesById = existingState.existingSeriesById
      val existingActiveSeriesByUrl = existingState.existingActiveSeriesByUrl
      val existingBooksBySeriesId = existingState.existingBooksBySeriesId

      val (deletedSeriesCount, deleteMissingSeriesMs) =
        logScanPhase(scanId, library, "delete_missing_series", details = { deletedCount ->
          "deletedSeries=$deletedCount"
        }) {
          if (scannedSeries.isEmpty()) {
            logger.info { "Scan returned no series, soft deleting all existing series" }
            val series = existingSeriesById.values.toList()
            seriesLifecycle.softDeleteMany(series)
            series.size
          } else {
            scannedSeries.keys.map { it.url }.let { urls ->
              val series = seriesRepository.findAllNotDeletedByLibraryIdAndUrlNotIn(library.id, urls)
              if (series.isNotEmpty()) {
                logger.info { "Soft deleting series not on disk anymore: $series" }
                seriesLifecycle.softDeleteMany(series)
              }
              series.size
            }
          }
        }
      metrics.deleteMissingSeriesMs = deleteMissingSeriesMs
      metrics.deletedSeries = deletedSeriesCount

      // delete books that don't exist anymore. We need to do this now, so trash bin can work
      val (seriesToSortAndRefresh, deleteMissingBooksMs) =
        logScanPhase(scanId, library, "delete_missing_books", details = { deletedState ->
          "deletedBooks=${deletedState.deletedBooks} affectedSeries=${deletedState.seriesToSortAndRefresh.size}"
        }) {
          scannedSeries.values.flatten().map { it.url }.let { urls ->
            val books = bookRepository.findAllNotDeletedByLibraryIdAndUrlNotIn(library.id, urls)
            val seriesToRefresh =
              if (books.isNotEmpty()) {
                logger.info { "Soft deleting books not on disk anymore: $books" }
                bookLifecycle.softDeleteMany(books)
                books
                  .map { it.seriesId }
                  .distinct()
                  .mapNotNull { existingSeriesById[it] }
                  .toMutableList()
              } else {
                mutableListOf()
              }
            DeletedBooksState(seriesToRefresh, books.size)
          }
        }
      metrics.deleteMissingBooksMs = deleteMissingBooksMs
      metrics.deletedBooks = seriesToSortAndRefresh.deletedBooks
      val seriesToSortAndRefreshList = seriesToSortAndRefresh.seriesToSortAndRefresh

      // we store the url of all the series that had deleted books
      // this can be used to detect changed series even if their file modified date did not change, for example because of NFS/SMB cache
      val seriesUrlWithDeletedBooks = seriesToSortAndRefreshList.map { it.url }.toSet()
      val reconciledSeriesByUrl = mutableMapOf<URL, Series>()

      val (_, reconcileSeriesBooksMs) =
        logScanPhase(scanId, library, "reconcile_series_books", details = {
          "createdSeries=${metrics.createdSeries} updatedSeries=${metrics.updatedSeries} addedBooks=${metrics.addedBooks} deferredHashBooks=${metrics.deferredHashBooks} outdatedBooks=${metrics.outdatedBooks} seriesToRefresh=${seriesToSortAndRefreshList.size}"
        }) {
          scannedSeries.forEach { (newSeries, newBooks) ->
            val existingSeries = existingActiveSeriesByUrl[newSeries.url]

            if (existingSeries == null) {
              logger.info { "Adding new series: $newSeries" }
              val createdSeries = seriesLifecycle.createSeries(newSeries)
              seriesLifecycle.addBooks(createdSeries, newBooks)
              tryRestoreSeries(createdSeries, newBooks)
              tryRestoreBooks(newBooks)
              metrics.createdSeries += 1
              metrics.addedBooks += newBooks.size
              seriesToSortAndRefreshList.add(createdSeries)
              reconciledSeriesByUrl[newSeries.url] = createdSeries
            } else {
              logger.debug { "Scanned series already exists. Scanned: $newSeries, Existing: $existingSeries" }
              val seriesChanged = newSeries.fileLastModified.notEquals(existingSeries.fileLastModified) || existingSeries.deletedDate != null || seriesUrlWithDeletedBooks.contains(newSeries.url)
              val reconciledSeries =
                if (seriesChanged) {
                  existingSeries.copy(fileLastModified = newSeries.fileLastModified, deletedDate = null)
                } else {
                  existingSeries
                }
              if (seriesChanged) {
                logger.info { "Series changed on disk, updating: $existingSeries" }
                seriesRepository.update(reconciledSeries)
                metrics.updatedSeries += 1
              }
              if (scanDeep || seriesChanged) {
                val existingBooks = existingBooksBySeriesId[existingSeries.id].orEmpty()
                val existingActiveBooksByUrl = existingBooks.filter { it.deletedDate == null }.associateBy { it.url }
                logger.debug { "Existing books: $existingBooks" }

                newBooks.forEach { newBook ->
                  logger.debug { "Trying to match scanned book by url: $newBook" }
                  existingActiveBooksByUrl[newBook.url]?.let { existingBook ->
                    logger.debug { "Matched existing book: $existingBook" }
                    if (newBook.fileLastModified.notEquals(existingBook.fileLastModified)) {
                      if (existingBook.fileSize == newBook.fileSize && existingBook.fileHash.isNotBlank()) {
                        logger.info { "Book changed on disk with same file size, defer hash verification: $existingBook" }
                        val updatedBook =
                          existingBook.copy(
                            fileLastModified = newBook.fileLastModified,
                            fileSize = newBook.fileSize,
                          )
                        bookRepository.update(updatedBook)
                        taskEmitter.verifyBookHash(updatedBook)
                        metrics.deferredHashBooks += 1
                      } else {
                        logger.info { "Book changed on disk, update and reset media status: $existingBook" }
                        val updatedBook =
                          existingBook.copy(
                            fileLastModified = newBook.fileLastModified,
                            fileSize = newBook.fileSize,
                            fileHash = "",
                          )
                        transactionTemplate.executeWithoutResult {
                          mediaRepository.findById(existingBook.id).let {
                            mediaRepository.update(it.copy(status = Media.Status.OUTDATED))
                          }
                          bookRepository.update(updatedBook)
                        }
                        metrics.outdatedBooks += 1
                      }
                    }
                  }
                }

                val booksToAdd = newBooks.filterNot { newBook -> existingActiveBooksByUrl.containsKey(newBook.url) }
                logger.info { "Adding new books: $booksToAdd" }
                seriesLifecycle.addBooks(existingSeries, booksToAdd)
                tryRestoreBooks(booksToAdd)
                metrics.addedBooks += booksToAdd.size
                seriesToSortAndRefreshList.add(existingSeries)
              }
              reconciledSeriesByUrl[newSeries.url] = reconciledSeries
            }
          }
        }
      metrics.reconcileSeriesBooksMs = reconcileSeriesBooksMs

      val (_, sortAndRefreshMs) =
        logScanPhase(scanId, library, "sort_and_refresh_series", details = { queuedCount ->
          "seriesRefreshQueued=$queuedCount"
        }) {
          val distinctSeriesToRefresh = seriesToSortAndRefreshList.distinctBy { it.id }
          distinctSeriesToRefresh.forEach {
            seriesLifecycle.sortBooks(it)
            taskEmitter.refreshSeriesMetadata(it.id)
          }
          distinctSeriesToRefresh.size
        }
      metrics.sortAndRefreshMs = sortAndRefreshMs
      metrics.seriesRefreshQueued = seriesToSortAndRefreshList.distinctBy { it.id }.size

      val (existingSidecarsState, reconcileSidecarsMs) =
        logScanPhase(scanId, library, "reconcile_sidecars", details = { sidecarState ->
          "existingSidecars=${sidecarState.existingSidecars.size} changedSidecars=${metrics.changedSidecars}"
        }) {
          val existingSidecars = sidecarRepository.findAll().filter { it.libraryId == library.id }
          val existingSidecarsByUrl = existingSidecars.associateBy { it.url }
          val reconciledBooksByUrl =
            if (reconciledSeriesByUrl.isEmpty()) {
              emptyMap()
            } else {
              bookRepository
                .findAllBySeriesIds(reconciledSeriesByUrl.values.map { it.id }.distinct())
                .filter { it.deletedDate == null }
                .associateBy { it.url }
            }
          scanResult.sidecars.forEach { newSidecar ->
            val existingSidecar = existingSidecarsByUrl[newSidecar.url]
            if (existingSidecar == null || existingSidecar.lastModifiedTime.notEquals(newSidecar.lastModifiedTime)) {
              when (newSidecar.source) {
                Sidecar.Source.SERIES ->
                  reconciledSeriesByUrl[newSidecar.parentUrl]?.let { series ->
                    logger.info { "Sidecar changed on disk (${newSidecar.url}, refresh Series for ${newSidecar.type}: $series" }
                    when (newSidecar.type) {
                      Sidecar.Type.ARTWORK -> taskEmitter.refreshSeriesLocalArtwork(series.id)
                      Sidecar.Type.METADATA -> taskEmitter.refreshSeriesMetadata(series.id)
                    }
                  }

                Sidecar.Source.BOOK ->
                  reconciledBooksByUrl[newSidecar.parentUrl]?.let { book ->
                    logger.info { "Sidecar changed on disk (${newSidecar.url}, refresh Book for ${newSidecar.type}: $book" }
                    when (newSidecar.type) {
                      Sidecar.Type.ARTWORK -> taskEmitter.refreshBookLocalArtwork(book)
                      Sidecar.Type.METADATA -> taskEmitter.refreshBookMetadata(book)
                    }
                  }
              }
              sidecarRepository.save(library.id, newSidecar)
              metrics.changedSidecars += 1
            }
          }
          ExistingSidecarsState(existingSidecars)
        }
      metrics.reconcileSidecarsMs = reconcileSidecarsMs

      val (deletedSidecarsCount, cleanupSidecarsMs) =
        logScanPhase(scanId, library, "cleanup_sidecars", details = { deletedSidecars ->
          "deletedSidecars=$deletedSidecars"
        }) {
          scanResult.sidecars.map { it.url }.let { newSidecarsUrls ->
            existingSidecarsState.existingSidecars
              .filterNot { existing -> newSidecarsUrls.contains(existing.url) }
              .let { sidecars ->
                sidecarRepository.deleteByLibraryIdAndUrls(library.id, sidecars.map { it.url })
                sidecars.size
              }
          }
        }
      metrics.cleanupSidecarsMs = cleanupSidecarsMs
      metrics.deletedSidecars = deletedSidecarsCount

      val (_, cleanupMs) =
        logScanPhase(scanId, library, if (library.emptyTrashAfterScan) "empty_trash" else "cleanup_empty_sets", details = {
          "emptyTrashAfterScan=${library.emptyTrashAfterScan}"
        }) {
          if (library.emptyTrashAfterScan)
            emptyTrash(library)
          else
            cleanupEmptySets()
        }
      metrics.cleanupMs = cleanupMs

      val totalMs = (System.nanoTime() - totalStartNanos) / 1_000_000
      logger.info {
        "scanRootFolder completed status=ok scanId=$scanId libraryId=${library.id} scanDeep=$scanDeep totalMs=$totalMs scannedSeries=${metrics.scannedSeries} scannedBooks=${metrics.scannedBooks} scannedSidecars=${metrics.scannedSidecars} existingSeries=${metrics.existingSeries} existingScannedSeries=${metrics.existingScannedSeries} preloadedBooks=${metrics.preloadedBooks} deletedSeries=${metrics.deletedSeries} deletedBooks=${metrics.deletedBooks} createdSeries=${metrics.createdSeries} updatedSeries=${metrics.updatedSeries} addedBooks=${metrics.addedBooks} deferredHashBooks=${metrics.deferredHashBooks} outdatedBooks=${metrics.outdatedBooks} seriesRefreshQueued=${metrics.seriesRefreshQueued} changedSidecars=${metrics.changedSidecars} deletedSidecars=${metrics.deletedSidecars} filesystemScanMs=${metrics.filesystemScanMs} clearUnavailableMs=${metrics.clearUnavailableMs} loadExistingMs=${metrics.loadExistingMs} deleteMissingSeriesMs=${metrics.deleteMissingSeriesMs} deleteMissingBooksMs=${metrics.deleteMissingBooksMs} reconcileSeriesBooksMs=${metrics.reconcileSeriesBooksMs} sortAndRefreshMs=${metrics.sortAndRefreshMs} reconcileSidecarsMs=${metrics.reconcileSidecarsMs} cleanupSidecarsMs=${metrics.cleanupSidecarsMs} cleanupMs=${metrics.cleanupMs}"
      }
    } catch (e: Exception) {
      val totalMs = (System.nanoTime() - totalStartNanos) / 1_000_000
      logger.warn(e) { "scanRootFolder completed status=failed scanId=$scanId libraryId=${library.id} scanDeep=$scanDeep totalMs=$totalMs" }
      throw e
    }

    eventPublisher.publishEvent(DomainEvent.LibraryScanned(library))
  }

  private inline fun <T> logScanPhase(
    scanId: String,
    library: Library,
    phase: String,
    details: (T) -> String = { "" },
    block: () -> T,
  ): Pair<T, Long> {
    val startNanos = System.nanoTime()

    try {
      val result = block()
      val durationMs = (System.nanoTime() - startNanos) / 1_000_000
      val detailMessage = details(result).takeIf { it.isNotBlank() }?.let { " $it" } ?: ""
      logger.info { "scanRootFolder phase=$phase status=ok scanId=$scanId libraryId=${library.id} durationMs=$durationMs$detailMessage" }
      return result to durationMs
    } catch (e: Exception) {
      val durationMs = (System.nanoTime() - startNanos) / 1_000_000
      logger.warn(e) { "scanRootFolder phase=$phase status=failed scanId=$scanId libraryId=${library.id} durationMs=$durationMs" }
      throw e
    }
  }

  private data class ExistingScanState(
    val existingSeriesById: Map<String, Series>,
    val existingActiveSeriesByUrl: Map<URL, Series>,
    val existingScannedSeriesIds: List<String>,
    val existingBooksBySeriesId: Map<String, List<Book>>,
  ) {
    val existingBooksCount: Int = existingBooksBySeriesId.values.sumOf { it.size }
  }

  private data class DeletedBooksState(
    val seriesToSortAndRefresh: MutableList<Series>,
    val deletedBooks: Int,
  )

  private data class ExistingSidecarsState(
    val existingSidecars: List<SidecarStored>,
  )

  private data class ScanRootFolderMetrics(
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
  )

  /**
   * This will try to match newSeries with a deleted series.
   * Series are matched if:
   * - they have the same number of books
   * - all the books are matched by file size and file hash
   *
   * If a series is matched, the following will be restored from the deleted series to the new series:
   * - Collections
   * - Metadata. The metadata title will only be copied if locked. If not locked, the folder name is used.
   * - all books, via #tryRestoreBooks
   */
  private fun tryRestoreSeries(
    newSeries: Series,
    newBooks: List<Book>,
  ) {
    logger.info { "Try to restore series: $newSeries" }
    val bookSizes = newBooks.map { it.fileSize }

    val deletedCandidates =
      seriesRepository
        .findAll(SearchCondition.Deleted(SearchOperator.IsTrue), SearchContext.empty(), Pageable.unpaged())
        .content
        .mapNotNull { deletedCandidate ->
          val deletedBooks = bookRepository.findAllBySeriesId(deletedCandidate.id)
          val deletedBooksSizes = deletedBooks.map { it.fileSize }
          if (newBooks.size == deletedBooks.size && bookSizes.containsAll(deletedBooksSizes) && deletedBooksSizes.containsAll(bookSizes) && deletedBooks.all { it.fileHash.isNotBlank() }) {
            deletedCandidate to deletedBooks
          } else {
            null
          }
        }
    logger.debug { "Deleted series candidates: $deletedCandidates" }

    if (deletedCandidates.isNotEmpty()) {
      val newBooksWithHash = newBooks.map { book -> bookRepository.findByIdOrNull(book.id)!!.copy(fileHash = hasher.computeHash(book.path)) }
      bookRepository.update(newBooksWithHash)

      val match =
        deletedCandidates.find { (_, books) ->
          books.map { it.fileHash }.containsAll(newBooksWithHash.map { it.fileHash }) && newBooksWithHash.map { it.fileHash }.containsAll(books.map { it.fileHash })
        }

      if (match != null) {
        // restore series
        logger.info { "Match found, restore $match into $newSeries" }
        transactionTemplate.executeWithoutResult {
          // copy metadata
          seriesMetadataRepository.findById(match.first.id).let { deleted ->
            val newlyAdded = seriesMetadataRepository.findById(newSeries.id)
            seriesMetadataRepository.update(
              deleted.copy(
                seriesId = newSeries.id,
                title = if (deleted.titleLock) deleted.title else newlyAdded.title,
                titleSort = if (deleted.titleSortLock) deleted.titleSort else newlyAdded.titleSort,
              ),
            )
          }

          // copy user uploaded thumbnails
          thumbnailSeriesRepository.findAllBySeriesIdIdAndType(match.first.id, ThumbnailSeries.Type.USER_UPLOADED).forEach { deleted ->
            thumbnailSeriesRepository.update(deleted.copy(seriesId = newSeries.id))
          }

          // replace deleted series by new series in collections
          collectionRepository
            .findAllContainingSeriesId(match.first.id, filterOnLibraryIds = null)
            .forEach { col ->
              collectionRepository.update(
                col.copy(
                  seriesIds = col.seriesIds.map { if (it == match.first.id) newSeries.id else it },
                ),
              )
            }

          tryRestoreBooks(newBooksWithHash)

          // delete upgraded series
          seriesLifecycle.deleteMany(listOf(match.first))
        }
      }
    }
  }

  /**
   * This will try to match each book in newBooks with a deleted book.
   * Books are matched by file size, then by file hash.
   *
   * If a book is matched, the following will be restored from the deleted book to the new book:
   * - Media
   * - Read Progress
   * - Read Lists
   * - Metadata. The metadata title will only be copied if locked. If not locked, the filename is used, but a refresh for Title will be requested.
   */
  private fun tryRestoreBooks(newBooks: List<Book>) {
    logger.info { "Try to restore books: $newBooks" }
    newBooks.forEach { bookToAdd ->
      // try to find a deleted book that matches the file size
      val deletedCandidates = bookRepository.findAllDeletedByFileSize(bookToAdd.fileSize).filter { it.fileHash.isNotBlank() }
      logger.debug { "Deleted candidates: $deletedCandidates" }

      if (deletedCandidates.isNotEmpty()) {
        // if the book has no hash, compute the hash and store it
        val bookWithHash =
          if (bookToAdd.fileHash.isNotBlank())
            bookToAdd
          else
            bookRepository.findByIdOrNull(bookToAdd.id)!!.copy(fileHash = hasher.computeHash(bookToAdd.path)).also { bookRepository.update(it) }

        val match = deletedCandidates.find { it.fileHash == bookWithHash.fileHash }

        if (match != null) {
          // restore book
          logger.info { "Match found, restore $match into $bookToAdd" }
          transactionTemplate.executeWithoutResult {
            // copy media
            mediaRepository.copy(match.id, bookToAdd.id)

            // copy generated and user uploaded thumbnails
            thumbnailBookRepository.findAllByBookIdAndType(match.id, setOf(ThumbnailBook.Type.GENERATED, ThumbnailBook.Type.USER_UPLOADED)).forEach { deleted ->
              thumbnailBookRepository.update(deleted.copy(bookId = bookToAdd.id))
            }

            // copy metadata
            bookMetadataRepository.findById(match.id).let { deleted ->
              val newlyAdded = bookMetadataRepository.findById(bookToAdd.id)
              bookMetadataRepository.update(
                deleted.copy(
                  bookId = bookToAdd.id,
                  title = if (deleted.titleLock) deleted.title else newlyAdded.title,
                ),
              )
              if (!deleted.titleLock) taskEmitter.refreshBookMetadata(bookToAdd, setOf(BookMetadataPatchCapability.TITLE))
            }

            // copy read progress
            readProgressRepository
              .findAllByBookId(match.id)
              .map { it.copy(bookId = bookToAdd.id) }
              .forEach { readProgressRepository.save(it) }

            // replace deleted book by new book in read lists
            readListRepository
              .findAllContainingBookId(match.id, filterOnLibraryIds = null)
              .forEach { rl ->
                readListRepository.update(
                  rl.copy(
                    bookIds =
                      rl.bookIds.values
                        .map { if (it == match.id) bookToAdd.id else it }
                        .toIndexedMap(),
                  ),
                )
              }

            // delete soft-deleted book
            bookLifecycle.deleteOne(match)
          }
        }
      }
    }
  }

  fun emptyTrash(library: Library) {
    logger.info { "Empty trash for library: $library" }

    val seriesToDelete =
      seriesRepository
        .findAll(
          SearchCondition.AllOfSeries(
            SearchCondition.LibraryId(SearchOperator.Is(library.id)),
            SearchCondition.Deleted(SearchOperator.IsTrue),
          ),
          SearchContext.empty(),
          Pageable.unpaged(),
        ).content
    seriesLifecycle.deleteMany(seriesToDelete)

    val booksToDelete =
      bookRepository
        .findAll(
          SearchCondition.AllOfBook(
            SearchCondition.LibraryId(SearchOperator.Is(library.id)),
            SearchCondition.Deleted(SearchOperator.IsTrue),
          ),
          SearchContext.empty(),
          Pageable.unpaged(),
        ).content
    bookLifecycle.deleteMany(booksToDelete)
    booksToDelete.map { it.seriesId }.distinct().forEach { seriesId ->
      seriesRepository.findByIdOrNull(seriesId)?.let { seriesLifecycle.sortBooks(it) }
    }

    cleanupEmptySets()
  }

  private fun cleanupEmptySets() {
    if (komgaSettingsProvider.deleteEmptyCollections) {
      collectionLifecycle.deleteEmptyCollections()
    }

    if (komgaSettingsProvider.deleteEmptyReadLists) {
      readListLifecycle.deleteEmptyReadLists()
    }
  }
}
