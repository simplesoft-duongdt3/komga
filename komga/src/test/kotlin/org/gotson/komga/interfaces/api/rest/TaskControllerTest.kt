package org.gotson.komga.interfaces.api.rest

import org.gotson.komga.application.tasks.Task
import org.gotson.komga.application.tasks.TaskExecution
import org.gotson.komga.application.tasks.TaskExecutionRepository
import org.gotson.komga.application.tasks.TasksRepository
import org.gotson.komga.infrastructure.jooq.tasks.TaskExecutionDao
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Test
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.boot.test.autoconfigure.web.servlet.AutoConfigureMockMvc
import org.springframework.boot.test.context.SpringBootTest
import org.springframework.security.test.context.support.WithAnonymousUser
import org.springframework.test.web.servlet.MockMvc
import org.springframework.test.web.servlet.get
import java.time.LocalDateTime
import java.time.ZoneId

@SpringBootTest
@AutoConfigureMockMvc(printOnlyOnFailure = false)
class TaskControllerTest(
  @Autowired private val mockMvc: MockMvc,
  @Autowired private val tasksRepository: TasksRepository,
  @Autowired private val taskExecutionRepository: TaskExecutionRepository,
  @Autowired private val taskExecutionDao: TaskExecutionDao,
) {
  @BeforeEach
  @AfterEach
  fun cleanup() {
    tasksRepository.deleteAll()
    taskExecutionDao.deleteAll()
  }

  @Nested
  inner class NonAdminUser {
    @Test
    @WithAnonymousUser
    fun `given anonymous user when retrieving tasks then returns unauthorized`() {
      mockMvc
        .get("/api/v1/tasks")
        .andExpect {
          status { isUnauthorized() }
        }
    }

    @Test
    @WithMockCustomUser
    fun `given restricted user when retrieving tasks then returns forbidden`() {
      mockMvc
        .get("/api/v1/tasks")
        .andExpect {
          status { isForbidden() }
        }
    }
  }

  @Test
  @WithMockCustomUser(roles = ["ADMIN"])
  fun `given admin user when retrieving tasks then paged tasks are returned`() {
    tasksRepository.save(
      listOf(
        Task.HashBookPages("book1", 1),
        Task.AnalyzeBook("book2", 5, "series-1"),
        Task.ScanLibrary("library-1", true, 3),
      ),
    )
    tasksRepository.takeFirst("taskProcessor-1")

    mockMvc
      .get("/api/v1/tasks") {
        param("page", "0")
        param("size", "2")
        param("sort", "priority,desc")
      }.andExpect {
        status { isOk() }
        jsonPath("$.totalElements") { value(3) }
        jsonPath("$.content.length()") { value(2) }
        jsonPath("$.content[0].simpleType") { value("AnalyzeBook") }
        jsonPath("$.content[0].status") { value("RUNNING") }
        jsonPath("$.content[0].owner") { value("taskProcessor-1") }
        jsonPath("$.content[0].durationMillis") { isNumber() }
      }
  }

  @Test
  @WithMockCustomUser(roles = ["ADMIN"])
  fun `given admin user when retrieving tasks without explicit sort then default task ordering is applied`() {
    tasksRepository.save(Task.HashBookPages("book1", 1))
    tasksRepository.save(Task.HashBookPages("book2", 5))

    mockMvc
      .get("/api/v1/tasks")
      .andExpect {
        status { isOk() }
        jsonPath("$.content[0].priority") { value(5) }
      }
  }

  @Test
  @WithMockCustomUser(roles = ["ADMIN"])
  fun `given admin user when filtering tasks then only matching tasks are returned`() {
    tasksRepository.save(
      listOf(
        Task.AnalyzeBook("book1", 5, "series-1"),
        Task.AnalyzeBook("book2", 4, "series-2"),
        Task.HashBookPages("book3", 3),
      ),
    )
    tasksRepository.takeFirst("taskProcessor-1")

    mockMvc
      .get("/api/v1/tasks") {
        param("status", "QUEUED")
        param("simpleType", "AnalyzeBook")
      }.andExpect {
        status { isOk() }
        jsonPath("$.totalElements") { value(1) }
        jsonPath("$.content[0].simpleType") { value("AnalyzeBook") }
        jsonPath("$.content[0].status") { value("QUEUED") }
      }
  }

  @Nested
  inner class ExecutionEndpoints {
    @Test
    @WithAnonymousUser
    fun `given anonymous user when retrieving executions then returns unauthorized`() {
      mockMvc
        .get("/api/v1/tasks/executions")
        .andExpect { status { isUnauthorized() } }
    }

    @Test
    @WithMockCustomUser(roles = ["ADMIN"])
    fun `given admin user when retrieving executions then paged results are returned`() {
      val exec = TaskExecution(
        id = "ex1",
        simpleType = "ScanLibrary",
        taskId = "SCAN_LIBRARY_lib1",
        libraryId = "lib1",
        seriesId = null,
        bookId = null,
        startDate = LocalDateTime.now(ZoneId.of("Z")).minusSeconds(10),
        endDate = LocalDateTime.now(ZoneId.of("Z")),
        success = true,
        errorMessage = null,
        durationMillis = 1500,
      )
      taskExecutionRepository.save(exec)

      mockMvc
        .get("/api/v1/tasks/executions") {
          param("page", "0")
          param("size", "10")
          param("sort", "startDate,desc")
        }.andExpect {
          status { isOk() }
          jsonPath("$.totalElements") { value(1) }
          jsonPath("$.content[0].id") { value("ex1") }
          jsonPath("$.content[0].simpleType") { value("ScanLibrary") }
          jsonPath("$.content[0].libraryId") { value("lib1") }
          jsonPath("$.content[0].success") { value(true) }
          jsonPath("$.content[0].durationMillis") { value(1500) }
        }
    }

    @Test
    @WithMockCustomUser(roles = ["ADMIN"])
    fun `given admin user when retrieving executions with library filter then only matching are returned`() {
      taskExecutionRepository.save(
        TaskExecution("e1", "ScanLibrary", null, "lib-a", null, null, LocalDateTime.now(ZoneId.of("Z")), null, true, null, 100),
      )
      taskExecutionRepository.save(
        TaskExecution("e2", "AnalyzeBook", null, "lib-b", null, null, LocalDateTime.now(ZoneId.of("Z")), null, true, null, 200),
      )

      mockMvc
        .get("/api/v1/tasks/executions") {
          param("libraryId", "lib-a")
        }.andExpect {
          status { isOk() }
          jsonPath("$.totalElements") { value(1) }
          jsonPath("$.content[0].libraryId") { value("lib-a") }
        }
    }

    @Test
    @WithMockCustomUser(roles = ["ADMIN"])
    fun `given admin user when retrieving recent failures then only failures are returned`() {
      taskExecutionRepository.save(
        TaskExecution("f1", "AnalyzeBook", null, null, null, null, LocalDateTime.now(ZoneId.of("Z")), null, false, "error msg", null),
      )
      taskExecutionRepository.save(
        TaskExecution("f2", "ScanLibrary", null, null, null, null, LocalDateTime.now(ZoneId.of("Z")), null, false, "another error", null),
      )

      mockMvc
        .get("/api/v1/tasks/executions/recent-failures") {
          param("limit", "10")
        }.andExpect {
          status { isOk() }
          jsonPath("$.length()") { value(2) }
          jsonPath("$[0].success") { value(false) }
          jsonPath("$[1].success") { value(false) }
        }
    }

    @Test
    @WithMockCustomUser(roles = ["ADMIN"])
    fun `given admin user when retrieving execution summary then stats are returned`() {
      taskExecutionRepository.save(
        TaskExecution("s1", "ScanLibrary", null, "lib1", null, null, LocalDateTime.now(ZoneId.of("Z")), null, true, null, 1000),
      )
      taskExecutionRepository.save(
        TaskExecution("s2", "ScanLibrary", null, "lib1", null, null, LocalDateTime.now(ZoneId.of("Z")), null, false, "err", 500),
      )

      mockMvc
        .get("/api/v1/tasks/executions/summary")
        .andExpect {
          status { isOk() }
          jsonPath("$.length()") { value(1) }
          jsonPath("$[0].simpleType") { value("ScanLibrary") }
          jsonPath("$[0].libraryId") { value("lib1") }
          jsonPath("$[0].totalCount") { value(2) }
          jsonPath("$[0].successCount") { value(1) }
          jsonPath("$[0].failureCount") { value(1) }
          jsonPath("$[0].avgDurationMillis") { isNumber() }
          jsonPath("$[0].lastExecutionDate") { isNotEmpty() }
        }
    }

    @Test
    @WithMockCustomUser(roles = ["ADMIN"])
    fun `given admin user when retrieving execution summary filtered by library then only that library is returned`() {
      taskExecutionRepository.save(
        TaskExecution("x1", "ScanLibrary", null, "lib-a", null, null, LocalDateTime.now(ZoneId.of("Z")), null, true, null, 100),
      )
      taskExecutionRepository.save(
        TaskExecution("x2", "ScanLibrary", null, "lib-b", null, null, LocalDateTime.now(ZoneId.of("Z")), null, true, null, 200),
      )

      mockMvc
        .get("/api/v1/tasks/executions/summary") {
          param("libraryId", "lib-a")
        }.andExpect {
          status { isOk() }
          jsonPath("$.length()") { value(1) }
          jsonPath("$[0].libraryId") { value("lib-a") }
        }
    }
  }
}