package org.gotson.komga.interfaces.api.rest.dto

import io.swagger.v3.oas.annotations.media.Schema
import jakarta.validation.Valid
import jakarta.validation.constraints.NotBlank
import jakarta.validation.constraints.NotEmpty

@Schema(description = "Add books that already exist on disk to an existing series")
data class AddBooksToSeriesDto(
  @get:NotBlank
  @Schema(description = "Library ID the series belongs to", required = true)
  val libraryId: String,

  @field:Valid
  @get:NotEmpty
  @Schema(description = "Books to register (already exist on disk)", required = true)
  val books: List<BookInSeriesDto>,
)
