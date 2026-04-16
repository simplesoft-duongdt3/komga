package org.gotson.komga.interfaces.api.persistence

import org.gotson.komga.interfaces.api.rest.dto.TaskDto
import org.gotson.komga.interfaces.api.rest.dto.TaskStatusDto
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable

interface TaskDtoRepository {
  fun findAll(
    pageable: Pageable,
    status: Set<TaskStatusDto>? = null,
    simpleType: Collection<String>? = null,
  ): Page<TaskDto>
}