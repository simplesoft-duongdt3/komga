package org.gotson.komga.application.tasks

import io.github.oshai.kotlinlogging.KotlinLogging
import org.gotson.komga.infrastructure.configuration.KomgaSettingsProvider
import org.gotson.komga.infrastructure.configuration.SettingChangedEvent
import org.springframework.beans.factory.InitializingBean
import org.springframework.boot.context.event.ApplicationReadyEvent
import org.springframework.boot.task.ThreadPoolTaskExecutorBuilder
import org.springframework.context.event.EventListener
import org.springframework.scheduling.concurrent.ThreadPoolTaskExecutor
import org.springframework.stereotype.Service

private val logger = KotlinLogging.logger {}

@Service
class TaskProcessor(
  private val tasksRepository: TasksRepository,
  private val taskHandler: TaskHandler,
  private val settingsProvider: KomgaSettingsProvider,
  taskExecutorBuilder: ThreadPoolTaskExecutorBuilder,
) : InitializingBean {
  val executor: ThreadPoolTaskExecutor =
    taskExecutorBuilder
      .threadNamePrefix("taskProcessor-")
      .corePoolSize(settingsProvider.taskPoolSize)
      .build()
      .apply { initialize() }

  var processTasks = false

  override fun afterPropertiesSet() {
    val disowned = tasksRepository.disown()
    if (disowned > 0)
      logger.info { "Reset $disowned tasks that were not finished" }
    processTasks = true
  }

  @EventListener(SettingChangedEvent.TaskPoolSize::class)
  fun taskPoolSizeChanged() {
    executor.corePoolSize = settingsProvider.taskPoolSize
  }

  @EventListener(TaskAddedEvent::class, ApplicationReadyEvent::class)
  fun processAvailableTask() {
    if (processTasks) {
      val currentActive = executor.activeCount
      logger.debug { "Active count: $currentActive, Core Pool Size: ${executor.corePoolSize}, Pool Size: ${executor.poolSize}" }

      // fill up to corePoolSize worker threads
      val slotsFree = executor.corePoolSize - currentActive
      if (slotsFree > 0) {
        repeat(slotsFree) {
          executor.execute { takeAndProcess() }
        }
      }
    } else {
      logger.debug { "Not processing tasks" }
    }
  }

  private fun takeAndProcess() {
    logger.debug { "Try to process first available task" }
    var task = tasksRepository.takeFirst()
    while (task != null) {
      logger.debug { "Found task to process: $task" }
      taskHandler.handleTask(task)
      logger.debug { "Task processed, remove it from the queue: $task" }
      tasksRepository.delete(task.uniqueId)
      task = tasksRepository.takeFirst()
    }
    logger.debug { "No available task found, going idle" }
  }
}
