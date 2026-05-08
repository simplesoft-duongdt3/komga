package org.gotson.komga.interfaces.api.rest

import io.swagger.v3.oas.annotations.Operation
import io.swagger.v3.oas.annotations.Parameter
import io.swagger.v3.oas.annotations.tags.Tag
import org.gotson.komga.application.tasks.TaskExecutionDto
import org.gotson.komga.application.tasks.TaskExecutionRepository
import org.gotson.komga.application.tasks.TaskExecutionSummaryDto
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
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.ResponseStatus
import org.springframework.web.bind.annotation.RestController

@RestController
@RequestMapping(produces = [MediaType.APPLICATION_JSON_VALUE])
@Tag(name = OpenApiConfiguration.TagNames.TASKS)
class TaskController(
  private val tasksRepository: TasksRepository,
  private val taskDtoRepository: TaskDtoRepository,
  private val taskExecutionRepository: TaskExecutionRepository,
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

  @GetMapping("api/v1/tasks/executions")
  @PageableAsQueryParam
  @PreAuthorize("hasRole('ADMIN')")
  @Operation(summary = "List task execution history")
  fun getTaskExecutions(
    @Parameter(hidden = true) page: Pageable,
    @RequestParam(required = false) simpleType: Set<String>?,
    @RequestParam(required = false) libraryId: String?,
    @RequestParam(required = false) success: Boolean?,
  ): Page<TaskExecutionDto> {
    val sort =
      if (page.sort.isSorted)
        page.sort
      else
        Sort.by(Sort.Order.desc("startDate"))

    val pageRequest =
      PageRequest.of(
        page.pageNumber,
        page.pageSize,
        sort,
      )

    return taskExecutionRepository.findAll(pageRequest, simpleType, libraryId, success)
  }

  @GetMapping("api/v1/tasks/executions/recent-failures")
  @PreAuthorize("hasRole('ADMIN')")
  @Operation(summary = "List recent failed task executions")
  fun getRecentTaskFailures(
    @RequestParam(defaultValue = "20") limit: Int,
  ): List<TaskExecutionDto> = taskExecutionRepository.findRecentFailures(limit)

  @GetMapping("api/v1/tasks/executions/summary")
  @PreAuthorize("hasRole('ADMIN')")
  @Operation(summary = "Task execution summary grouped by type and library")
  fun getTaskExecutionSummary(
    @RequestParam(required = false) libraryId: String?,
  ): List<TaskExecutionSummaryDto> = taskExecutionRepository.summaryByLibrary(libraryId)
}
