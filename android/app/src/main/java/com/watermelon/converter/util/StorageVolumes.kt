// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.util

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.storage.StorageManager
import android.os.storage.StorageVolume

/** One mounted storage volume, as surfaced to the file manager UI. */
data class WvgcVolume(
    /** Stable key used to look up/store the SAF grant for this volume. On API 30+
     *  this is StorageVolume.uuid (or "primary" for the primary volume); below that,
     *  we fall back to the volume's description since uuid isn't exposed pre-30. */
    val id: String,
    val label: String,
    val isPrimary: Boolean,
    val isRemovable: Boolean,
    private val volume: StorageVolume,
) {
    /** Intent to open the SAF folder picker scoped to this volume where supported. */
    fun createOpenDocumentTreeIntent(): Intent =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            volume.createOpenDocumentTreeIntent()
        } else {
            // Android 8–9 support the SAF tree picker but cannot scope its initial
            // location to a StorageVolume. The user can still select a folder.
            Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
        }
}

/**
 * Enumerates mounted storage volumes (internal shared storage + any SD card /
 * USB-OTG storage). Read-only listing — does not touch permissions.
 */
object StorageVolumes {

    fun list(ctx: Context): List<WvgcVolume> {
        val sm = ctx.getSystemService(Context.STORAGE_SERVICE) as? StorageManager
            ?: return emptyList()
        return sm.storageVolumes.mapNotNull { vol -> toWvgcVolume(vol) }
    }

    private fun toWvgcVolume(vol: StorageVolume): WvgcVolume? {
        // Skip volumes that aren't in a usable state.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            if (vol.state != android.os.Environment.MEDIA_MOUNTED) return null
        }
        val id = volumeId(vol)
        val label = vol.getDescription(null) ?: if (vol.isPrimary) "Internal Storage" else "SD Card"
        return WvgcVolume(
            id = id,
            label = label,
            isPrimary = vol.isPrimary,
            isRemovable = vol.isRemovable,
            volume = vol,
        )
    }

    private fun volumeId(vol: StorageVolume): String {
        if (vol.isPrimary) return "primary"
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            vol.uuid ?: vol.hashCode().toString()
        } else {
            // uuid not available pre-API 30; description + removable flag is the
            // best stable-ish key we have. Good enough to distinguish volumes
            // within a single device session.
            "legacy_${vol.getDescription(null)}_${vol.isRemovable}"
        }
    }
}