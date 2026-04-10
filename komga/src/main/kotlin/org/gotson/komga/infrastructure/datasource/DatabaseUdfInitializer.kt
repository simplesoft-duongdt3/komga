package org.gotson.komga.infrastructure.datasource

import io.github.oshai.kotlinlogging.KotlinLogging
import org.gotson.komga.infrastructure.configuration.KomgaProperties
import org.springframework.beans.factory.annotation.Qualifier
import org.springframework.boot.context.event.ApplicationReadyEvent
import org.springframework.context.ApplicationListener
import org.springframework.stereotype.Component
import javax.sql.DataSource

private val logger = KotlinLogging.logger {}

@Component
class DatabaseUdfInitializer(
  private val databaseUdfProvider: DatabaseUdfProvider,
  @Qualifier("mainDataSourceRW") private val dataSource: DataSource,
  private val komgaProperties: KomgaProperties,
) : ApplicationListener<ApplicationReadyEvent> {

  override fun onApplicationEvent(event: ApplicationReadyEvent) {
    if (komgaProperties.database.type == DatabaseType.POSTGRESQL) {
      logger.info { "Initializing PostgreSQL database with custom functions and checking ICU collation" }
      try {
        dataSource.connection.use {
          databaseUdfProvider.initializeConnection(it)
        }
      } catch (e: Exception) {
        logger.error(e) { "Failed to initialize PostgreSQL database" }
      }
    }
  }
}
