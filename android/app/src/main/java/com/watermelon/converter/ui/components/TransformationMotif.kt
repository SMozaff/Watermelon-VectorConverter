// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * The shared Source -> Result transformation motif: `[SOURCE badge] ->
 * [RESULT badge]`. Deliberately just the two badges plus a plain arrow
 * glyph — no morphing, no animation, no gradient, no particles, per the
 * redesign prompt's explicit "keep it restrained, static, and functional"
 * instruction. Mirrors desktop's `transformation_motif()` in converter.rs
 * — same conceptual grammar (SVG -> VD XML / VD XML -> SVG), same
 * two-badges-plus-arrow shape, different platform's idiomatic layout
 * primitives.
 *
 * `isForward` matches this app's existing direction convention (see
 * ConversionViewModel/ReverseConversionViewModel and PreviewScreen.kt's
 * own `isForward: Boolean` parameter) rather than introducing a new
 * direction type — true = SVG -> VectorDrawable XML, false = the reverse.
 *
 * The whole row is exposed to accessibility services as one grouped
 * announcement ("Converting from SVG to Android VectorDrawable XML")
 * rather than three separate nodes (badge, arrow glyph, badge) that
 * TalkBack would otherwise announce as three disconnected stops. Uses
 * `clearAndSetSemantics` deliberately (not the additive `semantics(
 * mergeDescendants = true) {}` form): verified that mergeDescendants
 * still reads BOTH the custom group description AND each child's own
 * merged description back-to-back, which would double-announce the two
 * badges' individual "SVG format"/"...XML format" descriptions right
 * after the group description — the opposite of the single, clean
 * announcement this motif is meant to produce. clearAndSetSemantics
 * replaces all of it with just the one group description, which is what's
 * actually wanted here.
 */
@Composable
fun TransformationMotif(isForward: Boolean, modifier: Modifier = Modifier) {
    val sourceFormat = if (isForward) VectorFormat.SVG else VectorFormat.VD_XML
    val resultFormat = if (isForward) VectorFormat.VD_XML else VectorFormat.SVG
    val groupDescription = if (isForward) {
        "Converting from SVG to Android VectorDrawable XML"
    } else {
        "Converting from Android VectorDrawable XML to SVG"
    }

    Row(
        modifier = modifier.clearAndSetSemantics { contentDescription = groupDescription },
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        FormatBadge(sourceFormat)
        Text(
            text = "→",
            fontSize = 16.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        FormatBadge(resultFormat)
    }
}
