// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.components

/**
 * The shared Source -> Result format-badge vocabulary. Same three variants
 * as the desktop counterpart (see desktop/src/app/converter.rs's
 * `VectorFormat` enum) — this is the "same conceptual grammar on both
 * platforms" the redesign prompt asks for. The two are independent
 * Kotlin/Rust types (there's no shared code between platforms) but are
 * kept deliberately in lock-step: same variant names, same short labels,
 * same semantic-token mapping rationale documented in FormatBadge.kt.
 *
 * [BATCH_ZIP] is not wired into any screen yet — it exists now so a future
 * batch/zip badge follows the same component without a breaking change,
 * per the redesign prompt's "optional future support" instruction.
 */
enum class VectorFormat(val shortLabel: String) {
    SVG("SVG"),
    VD_XML("XML"),
    BATCH_ZIP("ZIP"),
}
