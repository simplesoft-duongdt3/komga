package org.gotson.komga.infrastructure.jooq.tasks

import org.gotson.komga.application.tasks.TaskExecution
import org.gotson.komga.application.tasks.TaskExecutionDto
import org.gotson.komga.application.tasks.TaskExecutionRepository
import org.gotson.komga.application.tasks.TaskExecutionSummaryDto
import org.gotson.komga.infrastructure.datasource.DatabaseType
import org.gotson.komga.infrastructure.jooq.SplitDslDaoBase
import org.gotson.komga.infrastructure.jooq.toOrderBy
import org.gotson.komga.interfaces.api.persistence.TaskExecutionDtoRepository
import org.jooq.DSLContext
import org.jooq.Field
import org.jooq.impl.DSL
import org.springframework.beans.factory.annotation.Qualifier
import org.springframework.beans.factory.annotation.Value
import org.springframework.context.annotation.DependsOn
import org.springframework.data.domain.Page
import org.springframework.data.domain.PageImpl
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Pageable
import org.springframework.data.domain.Sort
import org.springframework.stereotype.Component
import java.time.LocalDateTime
import java.time.ZoneId

@Component
@DependsOn("flywaySecondaryMigrationInitializer")
class TaskExecutionDao(
  @Qualifier("tasksDslContextRW") dslRW: DSLContext,
  @Qualifier("tasksDslContextRO") dslRO: DSLContext,
  @param:Value("#{@komgaProperties.tasksDb.type}") private val databaseType: DatabaseType,
) : SplitDslDaoBase(dslRW, dslRO),
  TaskExecutionRepository,
  TaskExecutionDtoRepository {
  private val tableName = if (databaseType == DatabaseType.POSTGRESQL) "\"TASK_EXECUTION\"" else "TASK_EXECUTION"
  private val t = DSL.table(tableName)
  private val cId = col("ID", String::class.java)
  private val cSimpleType = col("SIMPLE_TYPE", String::class.java)
  private val cTaskId = col("TASK_ID", String::class.java)
  private val cLibraryId = col("LIBRARY_ID", String::class.java)
  private val cSeriesId = col("SERIES_ID", String::class.java)
  private val cBookId = col("BOOK_ID", String::class.java)
  private val cStartDate = col("START_DATE", LocalDateTime::class.java)
  private val cEndDate = col("END_DATE", LocalDateTime::class.java)
  private val cSuccess = col("SUCCESS", Any::class.java)
  private val cErrorMessage = col("ERROR_MESSAGE", String::class.java)
  private val cDurationMillis = col("DURATION_MILLIS", Long::class.java)

  private val allFields = arrayOf(cId, cSimpleType, cTaskId, cLibraryId, cSeriesId, cBookId, cStartDate, cEndDate, cSuccess, cErrorMessage, cDurationMillis)

  private val sorts: Map<String, Field<out Any>> =
    mapOf(
      "simpleType" to cSimpleType,
      "libraryId" to cLibraryId,
      "seriesId" to cSeriesId,
      "bookId" to cBookId,
      "startDate" to cStartDate,
      "endDate" to cEndDate,
      "success" to cSuccess,
      "durationMillis" to cDurationMillis,
    )

  private inline fun <reified T> col(name: String, type: Class<T>) =
    if (databaseType == DatabaseType.POSTGRESQL) DSL.field("\"$name\"", type) else DSL.field(name, type)

  private val trueValue: Any = if (databaseType == DatabaseType.POSTGRESQL) true else 1
  private val falseValue: Any = if (databaseType == DatabaseType.POSTGRESQL) false else 0

  private fun successEq(value: Boolean): org.jooq.Condition =
    if (databaseType == DatabaseType.POSTGRESQL)
      cSuccess.eq(DSL.`val`(value) as org.jooq.Field<Any>)
    else
      cSuccess.eq(DSL.`val`(if (value) 1 else 0) as org.jooq.Field<Any>)

  private fun readSuccess(record: org.jooq.Record): Boolean {
    val v = record[cSuccess, Any::class.java]!!
    return if (databaseType == DatabaseType.POSTGRESQL) v as Boolean else (v as Int) != 0
  }

  private fun toExecutionDto(record: org.jooq.Record): TaskExecutionDto =
    TaskExecutionDto(
      id = record[cId]!!,
      simpleType = record[cSimpleType]!!,
      taskId = record[cTaskId],
      libraryId = record[cLibraryId],
      seriesId = record[cSeriesId],
      bookId = record[cBookId],
      startDate = record[cStartDate]!!,
      endDate = record[cEndDate],
      success = readSuccess(record),
      errorMessage = record[cErrorMessage],
      durationMillis = record[cDurationMillis],
    )

  override fun save(execution: TaskExecution) {
    dslRW
      .insertInto(t, *allFields)
      .values(
        execution.id,
        execution.simpleType,
        execution.taskId,
        execution.libraryId,
        execution.seriesId,
        execution.bookId,
        execution.startDate,
        execution.endDate,
        if (databaseType == DatabaseType.SQLITE) (if (execution.success) 1 else 0) else execution.success,
        execution.errorMessage,
        execution.durationMillis,
      ).execute()
  }

  override fun findAll(
    pageable: Pageable,
    simpleType: Collection<String>?,
    libraryId: String?,
    success: Boolean?,
  ): Page<TaskExecutionDto> {
    val condition =
      DSL.noCondition()
        .and(if (simpleType.isNullOrEmpty()) DSL.noCondition() else cSimpleType.`in`(simpleType))
        .and(if (libraryId == null) DSL.noCondition() else cLibraryId.eq(libraryId))
        .and(
          when (success) {
            null -> DSL.noCondition()
            else -> successEq(success)
          },
        )

    val count = dslRO.select(DSL.count()).from(t).where(condition).fetchOne(0, Long::class.java) ?: 0

    val orderBy = pageable.sort.toOrderBy(sorts)

    val items =
      dslRO
        .select(*allFields)
        .from(t)
        .where(condition)
        .orderBy(orderBy)
        .apply { if (pageable.isPaged) limit(pageable.pageSize).offset(pageable.offset) }
        .fetch { toExecutionDto(it) }

    val pageSort = if (orderBy.isNotEmpty()) pageable.sort else Sort.unsorted()
    return PageImpl(
      items,
      if (pageable.isPaged)
        PageRequest.of(pageable.pageNumber, pageable.pageSize, pageSort)
      else
        PageRequest.of(0, maxOf(count, 20).toInt(), pageSort),
      count,
    )
  }

  override fun findAll(
    pageable: Pageable,
    simpleType: Collection<String>?,
    libraryId: String?,
  ): org.springframework.data.domain.Page<org.gotson.komga.interfaces.api.rest.dto.TaskExecutionDto> {
    val domainPage = findAll(pageable, simpleType, libraryId, null)
    val dtos =
      domainPage.map { dto ->
        org.gotson.komga.interfaces.api.rest.dto.TaskExecutionDto(
          id = dto.id,
          simpleType = dto.simpleType,
          taskId = dto.taskId,
          libraryId = dto.libraryId,
          seriesId = dto.seriesId,
          bookId = dto.bookId,
          startDate = dto.startDate,
          endDate = dto.endDate,
          success = dto.success,
          errorMessage = dto.errorMessage,
          durationMillis = dto.durationMillis,
        )
      }
    return dtos
  }

  override fun findRecentFailures(limit: Int): List<TaskExecutionDto> {
    val condition = successEq(false)
    return dslRO
      .select(*allFields)
      .from(t)
      .where(condition)
      .orderBy(cStartDate.desc())
      .limit(limit)
      .fetch { toExecutionDto(it) }
  }

  fun deleteAll() {
    dslRW.deleteFrom(t).execute()
  }

  override fun summaryByLibrary(libraryId: String?): List<TaskExecutionSummaryDto> {
    val libraryCondition = if (libraryId == null) DSL.noCondition() else cLibraryId.eq(libraryId)

    return dslRO
      .select(
        cSimpleType,
        cLibraryId,
        DSL.count().`as`("total_count"),
        DSL.sum(DSL.`when`(successEq(true), DSL.inline(1)).otherwise(DSL.inline(0))).`as`("success_count"),
        DSL.sum(DSL.`when`(successEq(false), DSL.inline(1)).otherwise(DSL.inline(0))).`as`("failure_count"),
        DSL.avg(cDurationMillis).`as`("avg_duration"),
        DSL.min(cDurationMillis).`as`("min_duration"),
        DSL.max(cDurationMillis).`as`("max_duration"),
        DSL.max(cStartDate).`as`("last_execution"),
      ).from(t)
      .where(libraryCondition)
      .groupBy(cSimpleType, cLibraryId)
      .orderBy(cSimpleType)
      .fetch { record ->
        TaskExecutionSummaryDto(
          simpleType = record[cSimpleType]!!,
          libraryId = record[cLibraryId],
          totalCount = record["total_count", Long::class.java] ?: 0,
          successCount = record["success_count", Long::class.java] ?: 0,
          failureCount = record["failure_count", Long::class.java] ?: 0,
          avgDurationMillis = record["avg_duration", Double::class.java],
          minDurationMillis = record["min_duration", Long::class.java],
          maxDurationMillis = record["max_duration", Long::class.java],
          lastExecutionDate = record["last_execution", LocalDateTime::class.java],
        )
      }
  }
}
