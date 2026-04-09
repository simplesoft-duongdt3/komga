package org.gotson.komga.infrastructure.datasource

import io.github.oshai.kotlinlogging.KotlinLogging
import org.jooq.Condition
import org.jooq.Field
import org.jooq.impl.DSL
import java.sql.Connection
import java.util.concurrent.atomic.AtomicBoolean

private val log = KotlinLogging.logger {}

class PostgresUdfProvider : DatabaseUdfProvider {
  override val udfStripAccentsName = "UDF_STRIP_ACCENTS"
  override val collationUnicode3Name = "COLLATION_UNICODE_3"

  private val icuCollationAvailable = AtomicBoolean(false) // Default to false for safety
  private var icuCollationChecked = false
  private val lock = Any()

  override fun Field<String>.udfStripAccents(): Field<String> =
    // Use PostgreSQL's unaccent extension
    DSL.function("unaccent", String::class.java, this)

  override fun Field<String>.collateUnicode3(): Field<String> {
    val collationName = if (isIcuCollationAvailable()) "und-u-ks-level2" else "C"
    log.trace { "Using collation: $collationName for Unicode sorting" }
    return this.collate(collationName)
  }

  private fun isIcuCollationAvailable(): Boolean {
    if (icuCollationChecked) {
      return icuCollationAvailable.get()
    }

    synchronized(lock) {
      // If still not checked, it remains false by default until initializeConnection is called.
      // We don't mark it as checked here to allow a future initializeConnection call to perform the real check.
      return icuCollationAvailable.get()
    }
  }

  override fun regexp(field: Field<String>, pattern: String, caseSensitive: Boolean): Condition {
    // PostgreSQL uses ~ for regex matching, ~* for case-insensitive
    return if (caseSensitive) {
      DSL.condition("{0} ~ {1}", field, DSL.inline(pattern))
    } else {
      DSL.condition("{0} ~* {1}", field, DSL.inline(pattern))
    }
  }

  override fun initializeConnection(connection: Any) {
    val pgConnection = connection as Connection
    log.debug { "Initializing PostgreSQL connection with custom functions" }

    // Check ICU collation availability
    checkIcuCollationAvailability(pgConnection)

    // Ensure unaccent extension is available
    try {
      val checkExtensionSQL = "SELECT extname FROM pg_extension WHERE extname = 'unaccent'"
      val rs = pgConnection.createStatement().executeQuery(checkExtensionSQL)
      if (!rs.next()) {
        log.warn { "unaccent extension not found. Attempting to create it..." }
        pgConnection.createStatement().execute("CREATE EXTENSION IF NOT EXISTS unaccent")
        log.info { "Created unaccent extension" }
      } else {
        log.debug { "unaccent extension already exists" }
      }
    } catch (e: Exception) {
      log.error(e) { "Failed to check/create unaccent extension" }
    }

    // Create a wrapper function for UDF_STRIP_ACCENTS that uses unaccent
    val createFunctionSQL =
      """
      CREATE OR REPLACE FUNCTION $udfStripAccentsName(text TEXT)
      RETURNS TEXT AS $$
      BEGIN
          RETURN unaccent(text);
      END;
      $$ LANGUAGE plpgsql IMMUTABLE;
      """.trimIndent()

    try {
      pgConnection.createStatement().execute(createFunctionSQL)
      log.debug { "Created PostgreSQL function $udfStripAccentsName" }
    } catch (e: Exception) {
      log.error(e) { "Failed to create PostgreSQL function $udfStripAccentsName" }
    }
  }

  private fun checkIcuCollationAvailability(connection: Connection) {
    synchronized(lock) {
      if (icuCollationChecked) {
        return
      }

      try {
        // Check if the ICU collation exists in pg_collation
        val checkCollationSQL = """
          SELECT 1 FROM pg_collation 
          WHERE collname = 'und-u-ks-level2' 
          AND collencoding = -1  -- Works for any encoding
        """.trimIndent()
        
        val rs = connection.createStatement().executeQuery(checkCollationSQL)
        val available = rs.next()
        
        icuCollationAvailable.set(available)
        icuCollationChecked = true
        
        if (available) {
          log.info { "ICU collation 'und-u-ks-level2' is available" }
        } else {
          log.warn { 
            "ICU collation 'und-u-ks-level2' is not available. " +
            "Falling back to binary collation 'C'. " +
            "Sorting may be case-sensitive and accent-sensitive."
          }
        }
      } catch (e: Exception) {
        log.error(e) { "Failed to check ICU collation availability" }
        icuCollationAvailable.set(false)
        icuCollationChecked = true
      }
    }
  }
}
