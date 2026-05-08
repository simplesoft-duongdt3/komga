package org.gotson.komga.interfaces.api.rest.dto

import java.time.LocalDateTime

data class TaskExecutionDto(
  val id: String,
  val simpleType: String,
  val taskId: String?,
  val libraryId: String?,
  val seriesId: String?,
  val bookId: String?,
  val startDate: LocalDateTime,
  val endDate: LocalDateTime?,
  val success: Boolean,
  val errorMessage: String?,
  val durationMillis: Long?,
)
