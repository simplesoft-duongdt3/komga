package org.gotson.komga.interfaces.api.persistence

import org.gotson.komga.interfaces.api.rest.dto.TaskExecutionDto
import org.springframework.data.domain.Page
import org.springframework.data.domain.Pageable

interface TaskExecutionDtoRepository {
  fun findAll(
    pageable: Pageable,
    simpleType: Collection<String>? = null,
    libraryId: String? = null,
  ): Page<TaskExecutionDto>
}
