// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! VectorDrawable XML -> RevDoc (reverse-direction parsing, Contract C-4).
//! Handles <group> transforms, <clip-path>, solid/gradient <path> fills
//! (including <aapt:attr> nested gradients), and strokes.

use crate::error::ConversionError;
use crate::vd_models::*;

const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";

pub fn parse(vd_xml: &[u8]) -> Result<RevDoc, ConversionError> {
    let text = std::str::from_utf8(vd_xml)
        .map_err(|e| ConversionError::InvalidSvg(format!("not UTF-8: {e}")))?;
    let doc = roxmltree::Document::parse(text)
        .map_err(|e| ConversionError::InvalidSvg(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "vector" {
        return Err(ConversionError::InvalidSvg("root is not <vector>".into()));
    }

    let vw = android_f32(&root, "viewportWidth").unwrap_or(24.0);
    let vh = android_f32(&root, "viewportHeight").unwrap_or(24.0);
    let width = parse_dp(&root, "width").unwrap_or(vw);
    let height = parse_dp(&root, "height").unwrap_or(vh);
    let alpha = android_f32(&root, "alpha").unwrap_or(1.0);

    let mut nodes = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        if let Some(n) = parse_node(&child)? {
            nodes.push(n);
        }
    }

    Ok(RevDoc { viewport_w: vw, viewport_h: vh, width, height, alpha, nodes })
}

fn parse_node(el: &roxmltree::Node) -> Result<Option<RevNode>, ConversionError> {
    match el.tag_name().name() {
        "path" => Ok(Some(RevNode::Path(parse_path(el)?))),
        "group" => Ok(Some(RevNode::Group(parse_group(el)?))),
        // <clip-path> is consumed by parse_group when it scans its own
        // children (it isn't a node in its own right in the reverse tree).
        "clip-path" => Ok(None),
        _ => Ok(None),
    }
}

fn parse_group(el: &roxmltree::Node) -> Result<RevGroup, ConversionError> {
    let mut g = RevGroup {
        translate_x: android_f32(el, "translateX").unwrap_or(0.0),
        translate_y: android_f32(el, "translateY").unwrap_or(0.0),
        scale_x: android_f32(el, "scaleX").unwrap_or(1.0),
        scale_y: android_f32(el, "scaleY").unwrap_or(1.0),
        rotation: android_f32(el, "rotation").unwrap_or(0.0),
        pivot_x: android_f32(el, "pivotX").unwrap_or(0.0),
        pivot_y: android_f32(el, "pivotY").unwrap_or(0.0),
        clip_path: None,
        children: Vec::new(),
    };
    for child in el.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "clip-path" {
            g.clip_path = android_attr(&child, "pathData").map(|s| s.to_string());
            continue;
        }
        if let Some(n) = parse_node(&child)? {
            g.children.push(n);
        }
    }
    Ok(g)
}

fn parse_path(el: &roxmltree::Node) -> Result<RevPath, ConversionError> {
    let path_data = android_attr(el, "pathData")
        .ok_or_else(|| ConversionError::InvalidSvg("<path> missing android:pathData".into()))?
        .to_string();

    let stroke_color = android_attr(el, "strokeColor").map(|s| s.to_string());
    let stroke_width = android_f32(el, "strokeWidth").unwrap_or(0.0);
    let fill_type_evenodd = android_attr(el, "fillType")
        .map(|s| s.eq_ignore_ascii_case("evenOdd"))
        .unwrap_or(false);

    // Fill is either a plain android:fillColor attribute, or an
    // <aapt:attr name="android:fillColor"><gradient>...</gradient></aapt:attr>
    // nested child (Contract C-1's own gradient emission format, mirrored back).
    let mut fill = match android_attr(el, "fillColor") {
        Some(c) => RevFill::Solid(c.to_string()),
        None => RevFill::None,
    };
    for child in el.children().filter(|n| n.is_element() && n.tag_name().name() == "attr") {
        if child.attribute("name") != Some("android:fillColor") { continue; }
        if let Some(grad_el) = child.children().find(|n| n.has_tag_name("gradient")) {
            fill = RevFill::Gradient(parse_gradient(&grad_el)?);
        }
    }

    Ok(RevPath { path_data, fill, stroke_color, stroke_width, fill_type_evenodd })
}

fn parse_gradient(el: &roxmltree::Node) -> Result<RevGradient, ConversionError> {
    let stops: Vec<RevGradientStop> = el.children()
        .filter(|n| n.has_tag_name("item"))
        .filter_map(|item| {
            let offset = android_f32(&item, "offset")?;
            let color = android_attr(&item, "color")?.to_string();
            Some(RevGradientStop { offset, color })
        })
        .collect();

    let kind = android_attr(el, "type").unwrap_or("linear");
    if kind == "radial" {
        Ok(RevGradient::Radial {
            cx: android_f32(el, "centerX").unwrap_or(0.5),
            cy: android_f32(el, "centerY").unwrap_or(0.5),
            r: android_f32(el, "gradientRadius").unwrap_or(0.5),
            stops,
        })
    } else {
        Ok(RevGradient::Linear {
            x1: android_f32(el, "startX").unwrap_or(0.0),
            y1: android_f32(el, "startY").unwrap_or(0.0),
            x2: android_f32(el, "endX").unwrap_or(1.0),
            y2: android_f32(el, "endY").unwrap_or(0.0),
            stops,
        })
    }
}

/// Read an android: namespaced attribute, falling back to a bare (unprefixed)
/// attribute — some hand-authored or third-party VD files omit the prefix
/// binding while still meaning the android: attribute.
fn android_attr<'a>(el: &'a roxmltree::Node, name: &str) -> Option<&'a str> {
    el.attribute((ANDROID_NS, name)).or_else(|| el.attribute(name))
}

fn android_f32(el: &roxmltree::Node, name: &str) -> Option<f32> {
    android_attr(el, name).and_then(|s| s.trim().parse().ok())
}

/// width/height carry a "dp" suffix (e.g. "24dp"); strip it before parsing.
fn parse_dp(el: &roxmltree::Node, name: &str) -> Option<f32> {
    android_attr(el, name).and_then(|s| s.trim_end_matches("dp").trim().parse().ok())
}
