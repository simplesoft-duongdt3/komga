package org.gotson.komga.interfaces.api.rest

import io.swagger.v3.oas.annotations.Operation
import io.swagger.v3.oas.annotations.Parameter
import io.swagger.v3.oas.annotations.tags.Tag
import org.gotson.komga.application.tasks.TasksRepository
import org.gotson.komga.infrastructure.openapi.OpenApiConfiguration
import org.gotson.komga.infrastructure.openapi.PageableAsQueryParam
import org.gotson.komga.interfaces.api.persistence.TaskDtoRepository
import org.gotson.komga.interfaces.api.rest.dto.TaskDto
import org.gotson.komga.interfaces.api.rest.dto.TaskStatusDto
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.http.HttpStatus
import org.springframework.http.MediaType
import org.springframework.security.access.prepost.PreAuthorize
import org.springframework.web.bind.annotation.DeleteMapping
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.ResponseStatus
import org.springframework.web.bind.annotation.RestController

@RestController
@RequestMapping(produces = [MediaType.APPLICATION_JSON_VALUE])
@Tag(name = OpenApiConfiguration.TagNames.TASKS)
class TaskController(
  private val tasksRepository: TasksRepository,
  private val taskDtoRepository: TaskDtoRepository,
) {
  @GetMapping("api/v1/tasks")
  @PageableAsQueryParam
  @PreAuthorize("hasRole('ADMIN')")
  @Operation(summary = "List tasks")
  fun getTasks(
    @Parameter(hidden = true) page: Pageable,
    @RequestParam(required = false) status: Set<TaskStatusDto>?,
    @RequestParam(required = false) simpleType: Set<String>?,
  ): Page<TaskDto> {
    val sort =
      if (page.sort.isSorted)
        page.sort
      else
        Sort.by(Sort.Order.desc("priority"), Sort.Order.asc("lastModifiedDate"))

    val pageRequest =
      PageRequest.of(
        page.pageNumber,
        page.pageSize,
        sort,
      )

    return taskDtoRepository.findAll(pageRequest, status, simpleType)
  }

  @DeleteMapping("api/v1/tasks")
  @ResponseStatus(HttpStatus.OK)
  @PreAuthorize("hasRole('ADMIN')")
  @Operation(summary = "Clear task queue", description = "Cancel all tasks queued")
  fun emptyTaskQueue(): Int = tasksRepository.deleteAllWithoutOwner()
}
