package org.gotson.komga.infrastructure.datasource

import org.flywaydb.core.Flyway
import org.springframework.beans.factory.InitializingBean
import org.springframework.beans.factory.annotation.Qualifier
import org.springframework.stereotype.Component
import javax.sql.DataSource
import org.gotson.komga.infrastructure.configuration.KomgaProperties

@Component
class FlywaySecondaryMigrationInitializer(
  @Qualifier("tasksDataSourceRW")
  private val tasksDataSource: DataSource,
  private val komgaProperties: KomgaProperties,
) : InitializingBean {
  // by default Spring Boot will perform migration only on the @Primary datasource
  override fun afterPropertiesSet() {
    val location = "classpath:tasks/migration/${komgaProperties.tasksDb.type.name.lowercase()}"
    Flyway
      .configure()
      .locations(location)
      .dataSource(tasksDataSource)
      .table("tasks_flyway_schema_history")
      .baselineOnMigrate(true)
      .load()
      .apply {
        migrate()
      }
  }
}
