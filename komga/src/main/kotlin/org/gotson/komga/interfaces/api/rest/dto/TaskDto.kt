package org.gotson.komga.interfaces.api.rest.dto

import java.time.LocalDateTime

data class TaskDto(
  val id: String,
  val simpleType: String,
  val status: TaskStatusDto,
  val owner: String?,
  val priority: Int,
  val groupId: String?,
  val createdDate: LocalDateTime,
  val lastModifiedDate: LocalDateTime,
  val durationMillis: Long,
)

enum class TaskStatusDto {
  QUEUED,
  RUNNING,
}