package org.gotson.komga.interfaces.api.rest.dto

import io.swagger.v3.oas.annotations.media.Schema
import jakarta.validation.constraints.NotBlank
import jakarta.validation.constraints.PositiveOrZero
import java.time.Instant

@Schema(description = "Request to create a new series and optionally register its book files on disk")
data class SeriesCreationDto(
  @get:NotBlank
  @Schema(description = "Library ID this series belongs to", required = true)
  val libraryId: String,

  @get:NotBlank
  @Schema(description = "Series directory name (for display)", required = true)
  val name: String,

  @get:NotBlank
  @Schema(description = "file:// URL to the series directory", required = true)
  val url: String,

  @Schema(description = "Last modified timestamp of the series directory", required = true)
  val fileLastModified: Instant,

  @Schema(description = "Books to register that already exist on disk")
  val books: List<BookInSeriesDto> = emptyList(),
)

@Schema(description = "A book file to register inside the series (file already exists on disk)")
data class BookInSeriesDto(
  @get:NotBlank
  @Schema(description = "Filename without extension", required = true)
  val name: String,

  @get:NotBlank
  @Schema(description = "file:// URL to the book file", required = true)
  val url: String,

  @get:PositiveOrZero
  @Schema(description = "File size in bytes", required = true)
  val fileSize: Long,

  @Schema(description = "Last modified timestamp of the file", required = true)
  val fileLastModified: Instant,

  @Schema(description = "Pre-computed file hash (optional)")
  val fileHash: String? = null,
)
