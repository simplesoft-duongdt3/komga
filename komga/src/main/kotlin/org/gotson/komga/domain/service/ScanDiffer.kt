package org.gotson.komga.domain.service

import io.github.oshai.kotlinlogging.KotlinLogging
import org.gotson.komga.domain.model.ChangedBookEntry
import org.gotson.komga.domain.model.DbSnapshot
import org.gotson.komga.domain.model.HashCache
import org.gotson.komga.domain.model.ScanDiff
import org.springframework.stereotype.Service

private val logger = KotlinLogging.logger {}

@Service
class ScanDiffer {

  fun diff(
    hashCache: HashCache,
    dbSnapshot: DbSnapshot,
  ): ScanDiff {
    val hashUris = hashCache.entries.keys
    val dbUris = dbSnapshot.books.keys

    val newBookUris = hashUris - dbUris
    val deletedBookUris = dbUris - hashUris
    val commonUris = hashUris.intersect(dbUris)

    val changedBooks = mutableMapOf<String, ChangedBookEntry>()

    for (uri in commonUris) {
      val h = hashCache.entries[uri]!!
      val d = dbSnapshot.books[uri]!!
      if (h.hash != d.fileHash || h.size != d.fileSize) {
        changedBooks[uri] = ChangedBookEntry(
          bookId = d.id,
          seriesId = d.seriesId,
          newHash = h.hash,
          newSize = h.size,
        )
      }
    }

    val newBooksGroupedBySeriesUrl = newBookUris
      .map { uri ->
        val parentUrl = uri.substringBeforeLast("/") + "/"
        uri to parentUrl
      }
      .groupBy({ it.second }, { it.first })
      .mapValues { it.value.toList() }

    logger.info {
      "Diff result: new=${newBookUris.size} deleted=${deletedBookUris.size} " +
        "changed=${changedBooks.size} unchanged=${commonUris.size - changedBooks.size} " +
        "newSeries=${newBooksGroupedBySeriesUrl.size}"
    }

    return ScanDiff(
      newBookUris = newBookUris,
      deletedBookUris = deletedBookUris,
      changedBooks = changedBooks,
      newBooksGroupedBySeriesUrl = newBooksGroupedBySeriesUrl,
    )
  }
}
