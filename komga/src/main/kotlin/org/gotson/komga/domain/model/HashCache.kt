package org.gotson.komga.domain.model

import com.fasterxml.jackson.annotation.JsonProperty

data class HashCacheEntry(
  val hash: String,
  val size: Long,
  @JsonProperty("mtime_secs") val mtimeSecs: Long,
)

data class HashCache(
  val type: String? = null,
  val version: Int? = null,
  val root: String? = null,
  val totalFiles: Int? = null,
  val totalBytes: Long? = null,
  val entries: Map<String, HashCacheEntry>,
)

data class DbBookEntry(
  val id: String,
  val seriesId: String,
  val url: String,
  val fileHash: String,
  val fileSize: Long,
  val fileLastModified: String,
  val seriesUrl: String,
)

data class DbSeriesEntry(
  val id: String,
  val name: String,
  val url: String,
  val libraryId: String,
  val fileLastModified: String,
)

data class DbSnapshot(
  val books: Map<String, DbBookEntry>,
  val series: Map<String, DbSeriesEntry>,
)

data class ScanDiff(
  val newBookUris: Set<String>,
  val deletedBookUris: Set<String>,
  val changedBooks: Map<String, ChangedBookEntry>,
  val newBooksGroupedBySeriesUrl: Map<String, List<String>>,
)

data class ChangedBookEntry(
  val bookId: String,
  val seriesId: String,
  val newHash: String,
  val newSize: Long,
)
