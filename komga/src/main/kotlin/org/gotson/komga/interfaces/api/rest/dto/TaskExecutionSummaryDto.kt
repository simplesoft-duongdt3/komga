package org.gotson.komga.interfaces.api.rest.dto

import java.time.LocalDateTime

data class TaskExecutionSummaryDto(
  val simpleType: String,
  val libraryId: String?,
  val totalCount: Long,
  val successCount: Long,
  val failureCount: Long,
  val avgDurationMillis: Double?,
  val minDurationMillis: Long?,
  val maxDurationMillis: Long?,
  val lastExecutionDate: LocalDateTime?,
)
