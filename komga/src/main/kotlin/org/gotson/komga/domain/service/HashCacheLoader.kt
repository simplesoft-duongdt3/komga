package org.gotson.komga.domain.service

import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import com.fasterxml.jackson.module.kotlin.readValue
import io.github.oshai.kotlinlogging.KotlinLogging
import org.gotson.komga.domain.model.HashCache
import org.gotson.komga.infrastructure.configuration.KomgaProperties
import org.springframework.stereotype.Component
import java.nio.file.Paths

private val logger = KotlinLogging.logger {}

@Component
class HashCacheLoader(
  private val komgaProperties: KomgaProperties,
) {
  private val mapper = jacksonObjectMapper()

  fun load(libraryId: String): HashCache? {
    val configDir = komgaProperties.configDir ?: return null
    val cacheFile = Paths.get(configDir, "hash-cache", "hashes-$libraryId.json")
    if (!cacheFile.toFile().exists()) return null
    return try {
      mapper.readValue(cacheFile.toFile())
    } catch (e: Exception) {
      logger.warn(e) { "Failed to load hash cache for library $libraryId" }
      null
    }
  }
}
