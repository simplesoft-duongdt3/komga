package org.gotson.komga.infrastructure.jooq

import com.github.f4b6a3.tsid.TsidCreator
import org.jooq.DSLContext
import org.jooq.impl.DSL
import java.io.Closeable
import io.github.oshai.kotlinlogging.KotlinLogging
import kotlin.text.lowercase


private val logger = KotlinLogging.logger {}
/**
 * Temporary table with a single STRING column.
 * This is made to store collection of values that are too long to be specified in a query condition,
 * by using a sub-select instead.
 *
 * The table name is automatically generated, and the table is dropped when the object is closed.
 */
class TempTable private constructor(
  private val dslContext: DSLContext,
  val name: String,
) : Closeable {
  constructor(dslContext: DSLContext) : this(dslContext, generateName())

  private var created = false

  fun create() {
    val dialect = dslContext.dialect()
    System.out.println("Creating temporary table " + name + " for dialect " + dialect.name)
    logger.warn { "Creating temporary table " + name + " for dialect " + dialect.name }
    val sql = if (dialect.name.lowercase().contains("postgres")) {
      // For PostgreSQL, create an unlogged table (faster) that will be explicitly dropped
      "CREATE UNLOGGED TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);"
    } else {
      // For SQLite, use temporary tables as before
      "CREATE TEMPORARY TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);"
    }

    System.out.println("Creating temporary table " + name + " for dialect " + dialect.name + " with SQL: " + sql)
    logger.warn { "Creating temporary table " + name + " for dialect " + dialect.name + " with SQL: " + sql }
    dslContext.execute(sql)
    created = true
  }

  fun insertTempStrings(
    batchSize: Int,
    collection: Collection<String>,
  ) {
    if (!created) create()
    if (collection.isNotEmpty()) {
      collection.chunked(batchSize).forEach { chunk ->
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

  fun selectTempStrings() = dslContext.select(DSL.field(DSL.name("STRING"), String::class.java)).from(DSL.table(DSL.name(name)))

  override fun close() {
    if (created) {
      try {
        System.out.println("Dropping temporary table " + name)
        logger.warn { "Dropping temporary table " + name}
        dslContext.dropTableIfExists(name).execute()
      } catch (e: Exception) {
        // Ignore errors when dropping table, especially if transaction is aborted
        // For PostgreSQL, unlogged tables will persist but that's acceptable
        System.out.println("Error dropping temporary table " + name + ": " + e.message)
      }
    }
  }

  companion object {
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
