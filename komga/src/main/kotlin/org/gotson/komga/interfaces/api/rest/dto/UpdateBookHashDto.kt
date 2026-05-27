package org.gotson.komga.interfaces.api.rest.dto

import io.swagger.v3.oas.annotations.media.Schema

@Schema(description = "Request to update a book's file hash. If fileHash is null/blank, Komga computes it from the file.")
data class UpdateBookHashDto(
  @Schema(description = "Pre-computed file hash (XXH3_128). If omitted, Komga reads the file and computes it.")
  val fileHash: String? = null,
)
