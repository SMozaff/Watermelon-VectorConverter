// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.components

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * A compact, informational format label — SVG / XML / (future) ZIP. Not a
 * button: deliberately built on the non-clickable `Surface(modifier, shape,
 * color, contentColor, ...)` overload rather than `AssistChip`/
 * `SuggestionChip`, both of which REQUIRE an `onClick` and — per Material3's
 * own documented behavior — report themselves to accessibility services as
 * a disabled interactive control when given no real action, which is worse
 * than a genuine static label, not equivalent to one. Verified this via
 * Context7 before choosing Surface over either chip component.
 *
 * Background derives from `MaterialTheme.colorScheme.primary` via
 * `.copy(alpha = ...)` (never a new hardcoded color): SVG/XML both
 * represent "a real, converted vector format," which reads as the same
 * positive/primary semantic this app already uses for success elsewhere —
 * so the badge intentionally borrows that meaning rather than introducing
 * a third, competing color idea. [VectorFormat.BATCH_ZIP] is deliberately
 * excluded from this green association: a batch/zip container isn't a
 * converted vector format, so it uses a neutral surfaceVariant/
 * onSurfaceVariant pairing instead, so it can't be mistaken for a third
 * real conversion format. Mirrors desktop's `format_badge()` in
 * converter.rs — same rationale, same shape, different platform's
 * idiomatic non-interactive container.
 *
 * No visible text label accompanies this badge at most call sites (it IS
 * the label), so its content description is set explicitly via
 * `Modifier.semantics { contentDescription = ... }` to a spelled-out form
 * ("SVG format", "Android VectorDrawable XML format") rather than the bare
 * 3-letter abbreviation a screen reader would otherwise read character-by-
 * character. Uses the additive `semantics {}` modifier, not
 * `clearAndSetSemantics {}` — the latter is meant for collapsing several
 * descendant nodes into one logical unit (per Android's own accessibility
 * docs: "reserve its use for elements in a collection... use it
 * sparingly"), which doesn't apply here since this is a single Text leaf
 * with no meaningful descendant semantics to clear.
 */
@Composable
fun FormatBadge(format: VectorFormat, modifier: Modifier = Modifier) {
    val (background, contentColor) = when (format) {
        VectorFormat.SVG, VectorFormat.VD_XML ->
            MaterialTheme.colorScheme.primary.copy(alpha = 0.16f) to MaterialTheme.colorScheme.primary
        VectorFormat.BATCH_ZIP ->
            MaterialTheme.colorScheme.surfaceVariant to MaterialTheme.colorScheme.onSurfaceVariant
    }
    val description = when (format) {
        VectorFormat.SVG -> "SVG format"
        VectorFormat.VD_XML -> "Android VectorDrawable XML format"
        VectorFormat.BATCH_ZIP -> "Batch ZIP archive"
    }

    Surface(
        modifier = modifier.semantics { contentDescription = description },
        shape = RoundedCornerShape(percent = 50),
        color = background,
        contentColor = contentColor,
    ) {
        Row(modifier = Modifier.padding(PaddingValues(horizontal = 10.dp, vertical = 4.dp))) {
            Text(
                text = format.shortLabel,
                fontSize = 12.sp,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}
