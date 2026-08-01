// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Tier 3: parse SVG gradients and resolve url(#id) fill references.
//! Android VectorDrawable supports gradients via aapt nested attributes;
//! the emitter renders these. Here we only build the model.

use crate::models::{Gradient, GradientStop};
use crate::utils::{parse_rgb, to_aarrggbb};
use std::collections::BTreeMap;

fn attr_f32(el: &roxmltree::Node, name: &str, default: f32) -> f32 {
    el.attribute(name)
        .and_then(|s| {
            let s = s.trim();
            // support "50%" -> 0.5 for objectBoundingBox-style coords
            if let Some(p) = s.strip_suffix('%') {
                p.trim().parse::<f32>().ok().map(|v| v / 100.0)
            } else {
                s.parse().ok()
            }
        })
        .unwrap_or(default)
}

/// Collect all gradients in the document, keyed by their id.
pub fn collect(doc: &roxmltree::Document) -> BTreeMap<String, Gradient> {
    let mut map = BTreeMap::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let tag = node.tag_name().name();
        if tag != "linearGradient" && tag != "radialGradient" { continue; }
        let id = match node.attribute("id") { Some(i) => i.to_string(), None => continue };
        let stops = parse_stops(&node);
        if stops.is_empty() { continue; }
        let grad = if tag == "linearGradient" {
            Gradient::Linear {
                x1: attr_f32(&node, "x1", 0.0),
                y1: attr_f32(&node, "y1", 0.0),
                x2: attr_f32(&node, "x2", 1.0),
                y2: attr_f32(&node, "y2", 0.0),
                stops,
            }
        } else {
            Gradient::Radial {
                cx: attr_f32(&node, "cx", 0.5),
                cy: attr_f32(&node, "cy", 0.5),
                r: attr_f32(&node, "r", 0.5),
                stops,
            }
        };
        map.insert(id, grad);
    }
    map
}

fn parse_stops(grad: &roxmltree::Node) -> Vec<GradientStop> {
    let mut stops = Vec::new();
    for stop in grad.children().filter(|n| n.has_tag_name("stop")) {
        let offset = attr_f32(&stop, "offset", 0.0).clamp(0.0, 1.0);
        // color + opacity may be in attributes or in a style="" string
        let (color_str, opacity) = stop_color_opacity(&stop);
        if let Some(rgb) = parse_rgb(&color_str) {
            stops.push(GradientStop { offset, color: to_aarrggbb(rgb, opacity) });
        }
    }
    stops
}

fn stop_color_opacity(stop: &roxmltree::Node) -> (String, f32) {
    let mut color = stop.attribute("stop-color").unwrap_or("black").to_string();
    let mut opacity = stop.attribute("stop-opacity").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    if let Some(style) = stop.attribute("style") {
        for decl in style.split(';') {
            let mut kv = decl.splitn(2, ':');
            match (kv.next().map(|s| s.trim()), kv.next().map(|s| s.trim())) {
                (Some("stop-color"), Some(v)) => color = v.to_string(),
                (Some("stop-opacity"), Some(v)) => { if let Ok(o) = v.parse() { opacity = o; } }
                _ => {}
            }
        }
    }
    (color, opacity)
}

/// If a fill value is `url(#id)`, return the id.
pub fn url_ref(fill: &str) -> Option<&str> {
    let s = fill.trim();
    let inner = s.strip_prefix("url(")?.strip_suffix(')')?;
    let inner = inner.trim().trim_start_matches('#');
    // handle url("#id") quoting
    Some(inner.trim_matches(|c| c == '"' || c == '\'' || c == '#'))
}
