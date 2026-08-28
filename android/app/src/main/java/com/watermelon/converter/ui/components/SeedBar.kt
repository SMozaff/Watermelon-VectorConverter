// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.watermelon.converter.ui.theme.WatermelonRed
import kotlin.math.roundToInt

/**
 * Watermelon-seed progress bar: a fixed row of [seedCount] dots that "fill"
 * left-to-right as progress advances. Filled seeds use watermelon red (the
 * seeds — a fixed brand color, deliberately not theme-dependent, same as
 * the seed's real-world color wouldn't change) — the remainder use the
 * theme's own onSurfaceVariant, since unlike the "seed" red, "the part not
 * yet filled" has no fixed brand meaning and should adapt between light/
 * dark themes like ordinary muted UI chrome does. Label likewise uses
 * MaterialTheme.colorScheme.primary rather than the fixed FreshTeal
 * constant, which was only ever the LIGHT-theme value for this role — see
 * this same fix already applied to VectorPropertiesPanel.kt/FilesScreen.kt
 * earlier in this pass. A percentage sits to the right.
 */
@Composable
fun SeedBar(
    progress: Float,           // 0f..1f
    label: String,
    modifier: Modifier = Modifier,
    seedCount: Int = 20,
) {
    val p = progress.coerceIn(0f, 1f)
    val filled = (p * seedCount).roundToInt()
    val empty = (seedCount - filled).coerceAtLeast(0)
    val pct = (p * 100).roundToInt()
    val emptySeedColor = MaterialTheme.colorScheme.onSurfaceVariant

    val seeds: AnnotatedString = buildAnnotatedString {
        withStyle(SpanStyle(color = WatermelonRed)) {
            append("\u25CF".repeat(filled))   // filled seed
        }
        withStyle(SpanStyle(color = emptySeedColor)) {
            append("\u00B7".repeat(empty))     // unfilled
        }
    }

    Column(modifier.fillMaxWidth()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                label,
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.weight(1f),
            )
            Text(pct.toString() + "%", style = MaterialTheme.typography.labelLarge)
        }
        Spacer(Modifier.height(4.dp))
        Text(seeds, style = MaterialTheme.typography.titleLarge, maxLines = 1)
    }
}
