// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter.viewmodel

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream

/** A rejected input shown before conversion; no native work has started yet. */
data class BatchInputRejection(
    val name: String,
    val reason: String,
)

/**
 * User-visible contract for a batch before native conversion starts. It is built
 * from the actual ZIP bytes so both document-picker and loose-file entry paths
 * show the same eligible count, exclusions, destination, and output name.
 */
data class BatchPreflight(
    val directionLabel: String,
    val inputLabel: String,
    val eligibleCount: Int,
    val rejected: List<BatchInputRejection>,
    val inputBytes: Long,
    val outputName: String,
    val destinationLabel: String,
) {
    val totalCount: Int get() = eligibleCount + rejected.size
}

internal data class SanitizedBatchInput(
    val eligibleZip: ByteArray,
    val eligibleCount: Int,
    val rejected: List<BatchInputRejection>,
)

/**
 * Rebuild a ZIP with only the direction-compatible entries. Preflight counts
 * describe this exact byte stream, preventing excluded files from reaching FFI.
 */
internal fun sanitizeBatchInput(zipBytes: ByteArray, expectedExtension: String): SanitizedBatchInput {
    val output = ByteArrayOutputStream()
    val rejected = mutableListOf<BatchInputRejection>()
    var eligible = 0
    ZipInputStream(ByteArrayInputStream(zipBytes)).use { zis ->
        ZipOutputStream(output).use { zos ->
            while (true) {
                val entry = zis.nextEntry ?: break
                if (entry.isDirectory) continue
                val name = entry.name.substringAfterLast('/')
                if (name.isBlank()) continue
                if (name.endsWith(expectedExtension, ignoreCase = true)) {
                    zos.putNextEntry(ZipEntry(name))
                    zis.copyTo(zos)
                    zos.closeEntry()
                    eligible++
                } else {
                    // Drain rejected entry before moving to the next ZIP entry.
                    zis.readBytes()
                    rejected += BatchInputRejection(name, "Expected $expectedExtension input")
                }
            }
        }
    }
    return SanitizedBatchInput(output.toByteArray(), eligible, rejected)
}

internal fun batchPreflight(
    sourceBytes: Long,
    eligibleCount: Int,
    rejected: List<BatchInputRejection>,
    directionLabel: String,
    inputLabel: String,
    outputName: String,
    destinationLabel: String,
): BatchPreflight = BatchPreflight(
    directionLabel = directionLabel,
    inputLabel = inputLabel,
    eligibleCount = eligibleCount,
    rejected = rejected,
    inputBytes = sourceBytes,
    outputName = outputName,
    destinationLabel = destinationLabel,
)

/** Build an in-memory ZIP from selected loose files while de-duplicating names. */
internal fun zipSelectedFiles(files: List<File>): ByteArray {
    val out = ByteArrayOutputStream()
    val usedNames = hashSetOf<String>()
    ZipOutputStream(out).use { zos ->
        files.filter { it.isFile && it.exists() }.forEach { file ->
            val base = file.name.substringBeforeLast('.', file.name)
            val extension = file.name.substringAfterLast('.', "")
            var entryName = file.name
            var suffix = 1
            while (!usedNames.add(entryName)) {
                entryName = if (extension.isBlank()) "${base}_$suffix" else "${base}_$suffix.$extension"
                suffix++
            }
            zos.putNextEntry(ZipEntry(entryName))
            file.inputStream().use { it.copyTo(zos) }
            zos.closeEntry()
        }
    }
    return out.toByteArray()
}

/** Count entries in an already-sanitized retry input. */
internal fun countZipEntries(zipBytes: ByteArray): Int {
    var count = 0
    ZipInputStream(ByteArrayInputStream(zipBytes)).use { zis ->
        while (true) {
            val entry = zis.nextEntry ?: break
            if (!entry.isDirectory) count++
        }
    }
    return count
}
