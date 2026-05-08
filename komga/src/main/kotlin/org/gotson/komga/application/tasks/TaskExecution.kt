package org.gotson.komga.application.tasks

import java.time.LocalDateTime

data class TaskExecution(
  val id: String,
  val simpleType: String,
  val taskId: String?,
  val libraryId: String?,
  val seriesId: String?,
  val bookId: String?,
  val startDate: LocalDateTime,
  val endDate: LocalDateTime? = null,
  val success: Boolean = false,
  val errorMessage: String? = null,
  val durationMillis: Long? = null,
)
