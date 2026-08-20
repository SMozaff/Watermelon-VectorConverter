// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter.util

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import java.io.ByteArrayOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/** Small SAF-only access layer used by ordinary conversion flows. */
object SafAccess {
    fun persistReadGrant(context: Context, uri: Uri) {
        runCatching {
            context.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
    }

    fun persistReadWriteGrant(context: Context, uri: Uri) {
        runCatching {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
        }
    }

    fun folderDisplayName(context: Context, uri: Uri): String =
        DocumentFile.fromTreeUri(context, uri)?.name ?: "Selected folder"

    /** Packages supported SVG/XML descendants into a ZIP without broad storage access. */
    fun zipSupportedFolder(context: Context, treeUri: Uri): ByteArray {
        val root = DocumentFile.fromTreeUri(context, treeUri)
            ?: error("The selected folder is no longer available")
        val resolver = context.contentResolver
        val output = ByteArrayOutputStream()
        val names = HashSet<String>()
        ZipOutputStream(output).use { zip ->
            fun walk(folder: DocumentFile, prefix: String) {
                folder.listFiles().forEach { child ->
                    val name = child.name ?: return@forEach
                    if (name.startsWith(".")) return@forEach
                    if (child.isDirectory) {
                        walk(child, if (prefix.isEmpty()) name else "$prefix/$name")
                    } else if (child.isFile && (name.endsWith(".svg", true) || name.endsWith(".xml", true))) {
                        val base = if (prefix.isEmpty()) name else "$prefix/$name"
                        var entryName = base
                        var suffix = 1
                        while (!names.add(entryName)) {
                            val stem = base.substringBeforeLast('.', base)
                            val ext = base.substringAfterLast('.', "")
                            entryName = if (ext.isEmpty()) "${stem}_$suffix" else "${stem}_$suffix.$ext"
                            suffix++
                        }
                        zip.putNextEntry(ZipEntry(entryName))
                        resolver.openInputStream(child.uri)?.use { it.copyTo(zip) }
                        zip.closeEntry()
                    }
                }
            }
            walk(root, "")
        }
        return output.toByteArray()
    }
}
