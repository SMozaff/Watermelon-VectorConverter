package com.watermelon.converter.viewmodel

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream
import kotlin.io.path.createTempDirectory

class BatchPreflightTest {
    @Test
    fun sanitizeBatchInput_keepsOnlyExpectedExtensionAndReportsExclusions() {
        val source = zipOf(
            "icon.svg" to "<svg/>",
            "notes.txt" to "ignore",
            "folder/second.svg" to "<svg/>",
        )

        val sanitized = sanitizeBatchInput(source, ".svg")

        assertEquals(2, sanitized.eligibleCount)
        assertEquals(listOf("notes.txt"), sanitized.rejected.map { it.name })
        assertEquals(listOf("icon.svg", "second.svg"), namesIn(sanitized.eligibleZip))
    }

    @Test
    fun zipSelectedFiles_deduplicatesSameBasename() {
        val root = createTempDirectory("watermelon-batch-").toFile()
        try {
            val firstDir = File(root, "first").apply { mkdirs() }
            val secondDir = File(root, "second").apply { mkdirs() }
            val first = File(firstDir, "icon.svg").apply { writeText("one") }
            val second = File(secondDir, "icon.svg").apply { writeText("two") }

            val names = namesIn(zipSelectedFiles(listOf(first, second)))

            assertEquals(listOf("icon.svg", "icon_1.svg"), names)
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun countZipEntries_ignoresDirectories() {
        val bytes = ByteArrayOutputStream().use { output ->
            ZipOutputStream(output).use { zip ->
                zip.putNextEntry(ZipEntry("folder/")); zip.closeEntry()
                zip.putNextEntry(ZipEntry("folder/icon.svg")); zip.write(byteArrayOf(1)); zip.closeEntry()
            }
            output.toByteArray()
        }

        assertEquals(1, countZipEntries(bytes))
    }

    private fun zipOf(vararg items: Pair<String, String>): ByteArray = ByteArrayOutputStream().use { output ->
        ZipOutputStream(output).use { zip ->
            items.forEach { (name, body) ->
                zip.putNextEntry(ZipEntry(name))
                zip.write(body.toByteArray())
                zip.closeEntry()
            }
        }
        output.toByteArray()
    }

    private fun namesIn(bytes: ByteArray): List<String> {
        val names = mutableListOf<String>()
        ZipInputStream(ByteArrayInputStream(bytes)).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                if (!entry.isDirectory) names += entry.name
            }
        }
        assertTrue(names.isNotEmpty())
        return names
    }
}
