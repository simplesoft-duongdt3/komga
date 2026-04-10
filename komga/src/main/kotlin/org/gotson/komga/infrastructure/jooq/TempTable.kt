package org.gotson.komga.infrastructure.jooq

import com.github.f4b6a3.tsid.TsidCreator
import org.jooq.DSLContext
import org.jooq.SQLDialect
import org.jooq.impl.DSL
import java.io.Closeable
import io.github.oshai.kotlinlogging.KotlinLogging

private val logger = KotlinLogging.logger {}

/**
 * Temporary table with a single STRING column.
 * This is made to store collection of values that are too long to be specified in a query condition,
 * by using a sub-select instead.
 *
 * For SQLite: creates a temporary table per instance, dropped on close.
 * For PostgreSQL: uses the permanent TEMP_INDEXING table with INDEX_NAME as discriminator,
 * rows are deleted on close.
 */
class TempTable private constructor(
  private val dslContext: DSLContext,
  val name: String,
) : Closeable {
  constructor(dslContext: DSLContext) : this(dslContext, generateName())

  private var created = false

  fun create() {
    val dialect = dslContext.dialect()
    if (isPostgres(dialect)) {
      // PostgreSQL uses the permanent TEMP_INDEXING table — no DDL needed
      logger.debug { "Using TEMP_INDEXING table with INDEX_NAME=$name" }
      created = true
      return
    }
    // SQLite: create temporary table as before
    val sql = "CREATE TEMPORARY TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);"
    logger.debug { "Creating temporary table $name for dialect ${dialect.name}" }
    dslContext.execute(sql)
    created = true
  }

  fun insertTempStrings(
    batchSize: Int,
    collection: Collection<String>,
  ) {
    if (!created) create()
    if (collection.isNotEmpty()) {
      val dialect = dslContext.dialect()

      collection.chunked(batchSize).forEach { chunk ->
        if (isPostgres(dialect)) {
          // Insert into permanent TEMP_INDEXING table with INDEX_NAME discriminator
          dslContext
            .batch(
              dslContext.insertInto(
                DSL.table(DSL.name("TEMP_INDEXING")),
                DSL.field(DSL.name("STRING"), String::class.java),
                DSL.field(DSL.name("INDEX_NAME"), String::class.java),
              ).values(null as String?, null as String?),
            ).also { step ->
              chunk.forEach { step.bind(it, name) }
            }.execute()
        } else {
          // SQLite: insert into dynamic temp table
          dslContext
            .batch(
              dslContext.insertInto(DSL.table(DSL.name(name)), DSL.field(DSL.name("STRING"), String::class.java)).values(null as String?),
            ).also { step ->
              chunk.forEach {
                step.bind(it)
              }
            }.execute()
        }
      }
    }
  }

  fun selectTempStrings() =
    if (isPostgres(dslContext.dialect())) {
      // Select from permanent table, filtered by this instance's INDEX_NAME
      dslContext
        .select(DSL.field(DSL.name("STRING"), String::class.java))
        .from(DSL.table(DSL.name("TEMP_INDEXING")))
        .where(DSL.field(DSL.name("INDEX_NAME"), String::class.java).eq(name))
    } else {
      // SQLite: select from dynamic temp table
      dslContext
        .select(DSL.field(DSL.name("STRING"), String::class.java))
        .from(DSL.table(DSL.name(name)))
    }

  override fun close() {
    if (!created) return
    val dialect = dslContext.dialect()
    try {
      if (isPostgres(dialect)) {
        // Delete rows for this INDEX_NAME from the shared table
        dslContext
          .deleteFrom(DSL.table(DSL.name("TEMP_INDEXING")))
          .where(DSL.field(DSL.name("INDEX_NAME"), String::class.java).eq(name))
          .execute()
        logger.debug { "Cleaned up TEMP_INDEXING rows for INDEX_NAME=$name" }
      } else {
        // SQLite: drop the temporary table
        dslContext.dropTableIfExists(name).execute()
        logger.debug { "Dropped temporary table $name" }
      }
    } catch (e: Exception) {
      logger.warn { "Error cleaning up temp data for $name: ${e.message}" }
    }
  }

  companion object {
    private fun isPostgres(dialect: SQLDialect): Boolean =
      dialect.name.lowercase().contains("postgres")

    private fun generateName(): String {
      val tsid = TsidCreator.getTsid256().toString()
      return "temp_v2_$tsid"
    }

    fun DSLContext.withTempTable(
      batchSize: Int,
      collection: Collection<String>,
    ) = TempTable(this, generateName())
      .also {
        it.insertTempStrings(batchSize, collection)
      }
  }
}
