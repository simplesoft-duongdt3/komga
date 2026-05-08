package org.gotson.komga.infrastructure.jooq.tasks

import org.assertj.core.api.Assertions.assertThat
import org.assertj.core.api.Assertions.within
import org.gotson.komga.application.tasks.TaskExecution
import org.gotson.komga.application.tasks.TaskExecutionRepository
import org.gotson.komga.interfaces.api.persistence.TaskExecutionDtoRepository
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.boot.test.context.SpringBootTest
import org.springframework.data.domain.PageRequest
import org.springframework.data.domain.Sort
import java.time.LocalDateTime
import java.time.ZoneId

@SpringBootTest
class TaskExecutionDaoTest(
  @Autowired private val taskExecutionDao: TaskExecutionDao,
  @Autowired private val taskExecutionRepository: TaskExecutionRepository,
  @Autowired private val taskExecutionDtoRepository: TaskExecutionDtoRepository,
) {
  @BeforeEach
  @AfterEach
  fun cleanup() {
    taskExecutionDao.deleteAll()
  }

  private fun makeExecution(
    id: String,
    simpleType: String = "ScanLibrary",
    libraryId: String? = "library-1",
    success: Boolean = true,
    durationMillis: Long? = 1000,
  ) = TaskExecution(
    id = id,
    simpleType = simpleType,
    taskId = "SCAN_LIBRARY_library-1",
    libraryId = libraryId,
    seriesId = null,
    bookId = null,
    startDate = LocalDateTime.now(ZoneId.of("Z")).minusSeconds(10),
    endDate = if (success) LocalDateTime.now(ZoneId.of("Z")) else null,
    success = success,
    errorMessage = if (success) null else "Something went wrong",
    durationMillis = durationMillis,
  )

  @Test
  fun `given execution saved when retrieving by id then it is returned`() {
    val exec = makeExecution("exec1")
    taskExecutionRepository.save(exec)

    val page = taskExecutionRepository.findAll(PageRequest.of(0, 10))
    assertThat(page.totalElements).isEqualTo(1)
    assertThat(page.content.first())
      .extracting("id", "simpleType", "libraryId", "success", "errorMessage", "durationMillis")
      .containsExactly("exec1", "ScanLibrary", "library-1", true, null, 1000L)
  }

  @Test
  fun `given multiple executions when finding paged then results are paginated`() {
    (1..5).forEach { taskExecutionRepository.save(makeExecution("exec$it")) }

    val page = taskExecutionRepository.findAll(PageRequest.of(0, 2, Sort.by(Sort.Order.desc("startDate"))))
    assertThat(page.totalElements).isEqualTo(5)
    assertThat(page.content).hasSize(2)
  }

  @Test
  fun `given executions when filtering by simpleType then only matching ones are returned`() {
    taskExecutionRepository.save(makeExecution("exec1", "ScanLibrary"))
    taskExecutionRepository.save(makeExecution("exec2", "AnalyzeBook"))
    taskExecutionRepository.save(makeExecution("exec3", "ScanLibrary"))

    val page = taskExecutionRepository.findAll(PageRequest.of(0, 10), listOf("ScanLibrary"))
    assertThat(page.totalElements).isEqualTo(2)
    assertThat(page.content.all { it.simpleType == "ScanLibrary" }).isTrue()
  }

  @Test
  fun `given executions when filtering by libraryId then only matching ones are returned`() {
    taskExecutionRepository.save(makeExecution("exec1", libraryId = "lib-a"))
    taskExecutionRepository.save(makeExecution("exec2", libraryId = "lib-b"))
    taskExecutionRepository.save(makeExecution("exec3", libraryId = "lib-a"))

    val page = taskExecutionRepository.findAll(PageRequest.of(0, 10), libraryId = "lib-a")
    assertThat(page.totalElements).isEqualTo(2)
    assertThat(page.content.all { it.libraryId == "lib-a" }).isTrue()
  }

  @Test
  fun `given executions with failures when finding recent failures then only failures are returned`() {
    taskExecutionRepository.save(makeExecution("exec1", success = true))
    taskExecutionRepository.save(makeExecution("exec2", success = false))
    taskExecutionRepository.save(makeExecution("exec3", success = false))

    val failures = taskExecutionRepository.findRecentFailures(10)
    assertThat(failures).hasSize(2)
    assertThat(failures.all { !it.success }).isTrue()
  }

  @Test
  fun `given executions with failures when finding recent failures with limit then only limit items are returned`() {
    (1..5).forEach { taskExecutionRepository.save(makeExecution("fail$it", success = false)) }

    val failures = taskExecutionRepository.findRecentFailures(3)
    assertThat(failures).hasSize(3)
  }

  @Test
  fun `given executions when getting summary then grouped stats are returned`() {
    taskExecutionRepository.save(makeExecution("e1", "ScanLibrary", "lib-a", true, 1000))
    taskExecutionRepository.save(makeExecution("e2", "ScanLibrary", "lib-a", true, 2000))
    taskExecutionRepository.save(makeExecution("e3", "ScanLibrary", "lib-a", false, 500))
    taskExecutionRepository.save(makeExecution("e4", "AnalyzeBook", "lib-b", true, 300))

    val summary = taskExecutionRepository.summaryByLibrary()

    assertThat(summary).hasSize(2)

    val scanLibA = summary.find { it.simpleType == "ScanLibrary" && it.libraryId == "lib-a" }!!
    assertThat(scanLibA.totalCount).isEqualTo(3)
    assertThat(scanLibA.successCount).isEqualTo(2)
    assertThat(scanLibA.failureCount).isEqualTo(1)
    assertThat(scanLibA.avgDurationMillis).isCloseTo(1166.67, within(1.0))
    assertThat(scanLibA.minDurationMillis).isEqualTo(500)
    assertThat(scanLibA.maxDurationMillis).isEqualTo(2000)
    assertThat(scanLibA.lastExecutionDate).isNotNull()
  }

  @Test
  fun `given executions when getting summary filtered by library then only that library is returned`() {
    taskExecutionRepository.save(makeExecution("e1", "ScanLibrary", "lib-a", true, 1000))
    taskExecutionRepository.save(makeExecution("e2", "ScanLibrary", "lib-b", true, 2000))

    val summary = taskExecutionRepository.summaryByLibrary("lib-a")
    assertThat(summary).hasSize(1)
    assertThat(summary.first().libraryId).isEqualTo("lib-a")
  }

  @Test
  fun `given executions saved when finding via DTO repository then dtos are mapped correctly`() {
    taskExecutionRepository.save(makeExecution("exec_dto_1"))

    val page = taskExecutionDtoRepository.findAll(PageRequest.of(0, 10))
    assertThat(page.totalElements).isEqualTo(1)
  }
}
