package org.gotson.komga.interfaces.api.rest

import org.gotson.komga.application.tasks.Task
import org.gotson.komga.application.tasks.TasksRepository
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Test
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.boot.test.autoconfigure.web.servlet.AutoConfigureMockMvc
import org.springframework.boot.test.context.SpringBootTest
import org.springframework.security.test.context.support.WithAnonymousUser
import org.springframework.test.web.servlet.MockMvc
import org.springframework.test.web.servlet.get

@SpringBootTest
@AutoConfigureMockMvc(printOnlyOnFailure = false)
class TaskControllerTest(
  @Autowired private val mockMvc: MockMvc,
  @Autowired private val tasksRepository: TasksRepository,
) {
  @AfterEach
  fun cleanup() {
    tasksRepository.deleteAll()
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
}