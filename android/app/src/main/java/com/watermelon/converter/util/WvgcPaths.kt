// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.util

import android.content.Context
import java.io.File

/**
 * Unified output locations within the app's own external files directory.
 * No special storage permission is required — the app always has full access
 * to its own scoped storage at Android/data/<package>/files/.
 *
 * Users can browse this directory via Files app or ADB.
 * Files are removed when the app is uninstalled (expected for a converter).
 */
object WvgcPaths {

    /** Root: Android/data/com.watermelon.converter/files/VectorConverter/ */
    fun root(ctx: Context): File =
        File(ctx.getExternalFilesDir(null), "VectorConverter")

    fun batchFilesDir(ctx: Context): File =
        File(root(ctx), "Batch_files").apply { mkdirs() }

    fun isolatedFilesDir(ctx: Context): File =
        File(root(ctx), "Isolated_files").apply { mkdirs() }

    /** Ensure both output directories exist. Call once on app start. */
    fun ensureDirs(ctx: Context) {
        batchFilesDir(ctx)
        isolatedFilesDir(ctx)
    }
}
