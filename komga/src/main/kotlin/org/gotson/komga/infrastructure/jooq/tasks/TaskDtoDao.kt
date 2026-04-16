package org.gotson.komga.infrastructure.jooq.tasks

import org.gotson.komga.infrastructure.jooq.SplitDslDaoBase
import org.gotson.komga.infrastructure.jooq.toOrderBy
import org.gotson.komga.interfaces.api.persistence.TaskDtoRepository
import org.gotson.komga.interfaces.api.rest.dto.TaskDto
import org.gotson.komga.interfaces.api.rest.dto.TaskStatusDto
import org.gotson.komga.jooq.tasks.Tables
import org.jooq.DSLContext
import org.jooq.Condition
import org.jooq.Field
import org.jooq.impl.DSL
import org.springframework.beans.factory.annotation.Qualifier
import org.springframework.context.annotation.DependsOn
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageImpl
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.stereotype.Component
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.temporal.ChronoUnit

@Component
@DependsOn("flywaySecondaryMigrationInitializer")
class TaskDtoDao(
  @Qualifier("tasksDslContextRW") dslRW: DSLContext,
  @Qualifier("tasksDslContextRO") dslRO: DSLContext,
) : SplitDslDaoBase(dslRW, dslRO),
  TaskDtoRepository {
  private val t = Tables.TASK

  private val sorts: Map<String, Field<out Any>> =
    mapOf(
      "id" to t.ID,
      "simpleType" to t.SIMPLE_TYPE,
      "owner" to t.OWNER,
      "priority" to t.PRIORITY,
      "groupId" to t.GROUP_ID,
      "createdDate" to t.CREATED_DATE,
      "lastModifiedDate" to t.LAST_MODIFIED_DATE,
    )

  override fun findAll(
    pageable: Pageable,
    status: Set<TaskStatusDto>?,
    simpleType: Collection<String>?,
  ): Page<TaskDto> {
    val condition = buildCondition(status, simpleType)
    val count = dslRO.fetchCount(t, condition)
    val orderBy = pageable.sort.toOrderBy(sorts)
    val now = LocalDateTime.now(ZoneId.of("Z"))

    val items =
      dslRO
        .select(
          t.ID,
          t.SIMPLE_TYPE,
          t.OWNER,
          t.PRIORITY,
          t.GROUP_ID,
          t.CREATED_DATE,
          t.LAST_MODIFIED_DATE,
        ).from(t)
        .where(condition)
        .orderBy(orderBy)
        .apply { if (pageable.isPaged) limit(pageable.pageSize).offset(pageable.offset) }
        .fetch { record ->
          val createdDate = record[t.CREATED_DATE]!!
          val owner = record[t.OWNER]

          TaskDto(
            id = record[t.ID]!!,
            simpleType = record[t.SIMPLE_TYPE]!!,
            status = if (owner == null) TaskStatusDto.QUEUED else TaskStatusDto.RUNNING,
            owner = owner,
            priority = record[t.PRIORITY]!!,
            groupId = record[t.GROUP_ID],
            createdDate = createdDate,
            lastModifiedDate = record[t.LAST_MODIFIED_DATE]!!,
            durationMillis = maxOf(0L, ChronoUnit.MILLIS.between(createdDate, now)),
          )
        }

    val pageSort = if (orderBy.isNotEmpty()) pageable.sort else Sort.unsorted()
    return PageImpl(
      items,
      if (pageable.isPaged)
        PageRequest.of(pageable.pageNumber, pageable.pageSize, pageSort)
      else
        PageRequest.of(0, maxOf(count, 20), pageSort),
      count.toLong(),
    )
  }

  private fun buildCondition(
    status: Set<TaskStatusDto>?,
    simpleType: Collection<String>?,
  ): Condition {
    val statusCondition =
      when {
        status.isNullOrEmpty() || status.containsAll(TaskStatusDto.entries) -> DSL.noCondition()
        status == setOf(TaskStatusDto.QUEUED) -> t.OWNER.isNull
        status == setOf(TaskStatusDto.RUNNING) -> t.OWNER.isNotNull
        else -> DSL.noCondition()
      }

    val simpleTypeCondition =
      if (simpleType.isNullOrEmpty())
        DSL.noCondition()
      else
        t.SIMPLE_TYPE.`in`(simpleType)

    return statusCondition.and(simpleTypeCondition)
  }
}