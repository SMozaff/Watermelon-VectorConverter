// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Dual approximate preview rendering via resvg (Contract C-2).
//! Renders the ORIGINAL SVG and the GENERATED VectorDrawable's pathData to PNG.
//! Both are explicitly approximate: resvg is not Android's Skia pipeline.

use crate::error::ConversionError;

const PX_MIN: u32 = 16;
const PX_MAX: u32 = 2048;

/// C-2: render original SVG bytes to a square px*px PNG (sRGB, transparent bg).
pub fn render_svg_preview(svg_bytes: &[u8], px: u32) -> Result<Vec<u8>, ConversionError> {
    check_px(px)?;
    let svg = std::str::from_utf8(svg_bytes)
        .map_err(|e| ConversionError::RenderError(format!("not UTF-8: {e}")))?;
    render_svg_string(svg, px)
}

/// C-2: render the GENERATED VectorDrawable by reconstructing an SVG from its
/// pathData and rendering THAT (keeps the preview honest about emitted XML).
pub fn render_vd_preview(vd_xml: &str, px: u32) -> Result<Vec<u8>, ConversionError> {
    check_px(px)?;
    let svg = vd_to_svg(vd_xml)?;
    render_svg_string(&svg, px)
}

fn check_px(px: u32) -> Result<(), ConversionError> {
    if !(PX_MIN..=PX_MAX).contains(&px) {
        return Err(ConversionError::RenderError(format!(
            "px {px} out of range {PX_MIN}..={PX_MAX}"
        )));
    }
    Ok(())
}

fn render_svg_string(svg: &str, px: u32) -> Result<Vec<u8>, ConversionError> {
    // usvg requires the SVG namespace. Real files have it; inject if absent
    // so a namespace-less snippet still renders for preview purposes.
    let svg_owned;
    let svg = if svg.contains("xmlns") {
        svg
    } else {
        svg_owned = svg.replacen("<svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"", 1);
        &svg_owned
    };

    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt)
        .map_err(|e| ConversionError::RenderError(e.to_string()))?;

    let mut pixmap = tiny_skia::Pixmap::new(px, px)
        .ok_or_else(|| ConversionError::RenderError("pixmap alloc failed".into()))?;

    // Fit the tree's viewbox into px*px.
    let size = tree.size();
    let scale = (px as f32 / size.width()).min(px as f32 / size.height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| ConversionError::RenderError(e.to_string()))
}

/// VectorDrawable -> SVG, for preview rendering. Delegates to the same
/// vd_parser + svg_emit pipeline that powers the real convert_vd conversion
/// (Contract C-4), rather than maintaining a second, weaker VD->SVG parser
/// here. Previously this function had its own minimal reimplementation
/// (pathData + fillColor + group transforms only, no gradients/clip-path/
/// stroke) which meant the preview could silently diverge from what
/// convert_vd actually produces. Using the real pipeline means the preview
/// is now an honest picture of the real reverse-conversion output.
fn vd_to_svg(vd_xml: &str) -> Result<String, ConversionError> {
    let doc = crate::vd_parser::parse(vd_xml.as_bytes())?;
    Ok(crate::svg_emit::emit(&doc))
}
