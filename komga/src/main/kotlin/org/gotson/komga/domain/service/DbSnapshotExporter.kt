package org.gotson.komga.domain.service

import io.github.oshai.kotlinlogging.KotlinLogging
import org.gotson.komga.domain.model.DbBookEntry
import org.gotson.komga.domain.model.DbSeriesEntry
import org.gotson.komga.domain.model.DbSnapshot
import org.gotson.komga.domain.persistence.BookRepository
import org.gotson.komga.domain.persistence.SeriesRepository
import org.springframework.stereotype.Component

private val logger = KotlinLogging.logger {}

@Component
class DbSnapshotExporter(
  private val seriesRepository: SeriesRepository,
  private val bookRepository: BookRepository,
) {
  fun export(libraryId: String): DbSnapshot {
    val allSeries = seriesRepository.findAllByLibraryId(libraryId)
      .filter { it.deletedDate == null }

    val seriesByUrl = allSeries.associateBy { it.url.toString() }

    val seriesById = allSeries.associateBy { it.id }

    val books =
      if (seriesById.isEmpty()) emptyList()
      else bookRepository.findAllBySeriesIds(seriesById.keys)
        .filter { it.deletedDate == null }

    val bookEntries = books.associate { book ->
      val s = seriesById[book.seriesId]!!
      book.url.toString() to DbBookEntry(
        id = book.id,
        seriesId = book.seriesId,
        url = book.url.toString(),
        fileHash = book.fileHash,
        fileSize = book.fileSize,
        fileLastModified = book.fileLastModified.toString(),
        seriesUrl = s.url.toString(),
      )
    }

    val seriesEntries = seriesByUrl.mapValues { (_, s) ->
      DbSeriesEntry(
        id = s.id,
        name = s.name,
        url = s.url.toString(),
        libraryId = s.libraryId,
        fileLastModified = s.fileLastModified.toString(),
      )
    }

    logger.info { "DB snapshot for library $libraryId: ${seriesEntries.size} series, ${bookEntries.size} books" }
    return DbSnapshot(books = bookEntries, series = seriesEntries)
  }
}
