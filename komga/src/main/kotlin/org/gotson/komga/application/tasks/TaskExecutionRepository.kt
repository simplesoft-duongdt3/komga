package org.gotson.komga.application.tasks

import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable
import java.time.LocalDateTime

interface TaskExecutionRepository {
  fun save(execution: TaskExecution)

  fun findAll(
    pageable: Pageable,
    simpleType: Collection<String>? = null,
    libraryId: String? = null,
    success: Boolean? = null,
  ): Page<TaskExecutionDto>

  fun findRecentFailures(limit: Int): List<TaskExecutionDto>

  fun summaryByLibrary(libraryId: String? = null): List<TaskExecutionSummaryDto>
}

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
