// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Tier 1: convert SVG basic shapes to VectorDrawable path data (Contract C-1).
//! VectorDrawable only supports <path>, so rect/circle/ellipse/line/poly* are
//! converted to equivalent absolute path-data strings.

use crate::utils::fmt_num;

fn attr(el: &roxmltree::Node, name: &str) -> Option<f32> {
    el.attribute(name).and_then(|s| s.trim().trim_end_matches("px").trim().parse().ok())
}

/// <rect x y width height [rx ry]> -> path. Supports rounded corners.
pub fn rect_to_path(el: &roxmltree::Node) -> Option<String> {
    let x = attr(el, "x").unwrap_or(0.0);
    let y = attr(el, "y").unwrap_or(0.0);
    let w = attr(el, "width")?;
    let h = attr(el, "height")?;
    if w <= 0.0 || h <= 0.0 { return None; }

    // rx/ry: if only one given, mirror it; clamp to half-extent.
    let mut rx = attr(el, "rx");
    let mut ry = attr(el, "ry");
    if rx.is_none() && ry.is_some() { rx = ry; }
    if ry.is_none() && rx.is_some() { ry = rx; }
    let rx = rx.unwrap_or(0.0).min(w / 2.0).max(0.0);
    let ry = ry.unwrap_or(0.0).min(h / 2.0).max(0.0);

    let n = |v: f32| fmt_num(v);
    if rx == 0.0 || ry == 0.0 {
        // sharp rectangle
        Some(format!(
            "M{},{} L{},{} L{},{} L{},{} Z",
            n(x), n(y), n(x + w), n(y), n(x + w), n(y + h), n(x), n(y + h)
        ))
    } else {
        // rounded: use arcs (A) — emitted as path data; the arc handler in the
        // normalizer will convert A to cubics. Standard SVG rounded-rect path.
        Some(format!(
            "M{},{} L{},{} A{},{} 0 0 1 {},{} L{},{} A{},{} 0 0 1 {},{} \
             L{},{} A{},{} 0 0 1 {},{} L{},{} A{},{} 0 0 1 {},{} Z",
            n(x + rx), n(y),
            n(x + w - rx), n(y),
            n(rx), n(ry), n(x + w), n(y + ry),
            n(x + w), n(y + h - ry),
            n(rx), n(ry), n(x + w - rx), n(y + h),
            n(x + rx), n(y + h),
            n(rx), n(ry), n(x), n(y + h - ry),
            n(x), n(y + ry),
            n(rx), n(ry), n(x + rx), n(y)
        ))
    }
}

/// <circle cx cy r> -> two-arc path.
pub fn circle_to_path(el: &roxmltree::Node) -> Option<String> {
    let cx = attr(el, "cx").unwrap_or(0.0);
    let cy = attr(el, "cy").unwrap_or(0.0);
    let r = attr(el, "r")?;
    if r <= 0.0 { return None; }
    Some(ellipse_path(cx, cy, r, r))
}

/// <ellipse cx cy rx ry> -> two-arc path.
pub fn ellipse_to_path(el: &roxmltree::Node) -> Option<String> {
    let cx = attr(el, "cx").unwrap_or(0.0);
    let cy = attr(el, "cy").unwrap_or(0.0);
    let rx = attr(el, "rx")?;
    let ry = attr(el, "ry")?;
    if rx <= 0.0 || ry <= 0.0 { return None; }
    Some(ellipse_path(cx, cy, rx, ry))
}

fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32) -> String {
    let n = |v: f32| fmt_num(v);
    // Start at right edge, two half arcs around. A handler converts to cubics.
    format!(
        "M{},{} A{},{} 0 1 1 {},{} A{},{} 0 1 1 {},{} Z",
        n(cx + rx), n(cy),
        n(rx), n(ry), n(cx - rx), n(cy),
        n(rx), n(ry), n(cx + rx), n(cy)
    )
}

/// <line x1 y1 x2 y2> -> path (only meaningful with a stroke).
pub fn line_to_path(el: &roxmltree::Node) -> Option<String> {
    let x1 = attr(el, "x1").unwrap_or(0.0);
    let y1 = attr(el, "y1").unwrap_or(0.0);
    let x2 = attr(el, "x2").unwrap_or(0.0);
    let y2 = attr(el, "y2").unwrap_or(0.0);
    Some(format!("M{},{} L{},{}", fmt_num(x1), fmt_num(y1), fmt_num(x2), fmt_num(y2)))
}

/// <polyline points="..."> -> open path.
pub fn polyline_to_path(el: &roxmltree::Node) -> Option<String> {
    points_to_path(el, false)
}

/// <polygon points="..."> -> closed path.
pub fn polygon_to_path(el: &roxmltree::Node) -> Option<String> {
    points_to_path(el, true)
}

fn points_to_path(el: &roxmltree::Node, close: bool) -> Option<String> {
    let raw = el.attribute("points")?;
    let nums: Vec<f32> = raw
        .split([',', ' ', '\n', '\t', '\r'])
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 4 || nums.len() % 2 != 0 { return None; }
    let mut d = String::new();
    let mut i = 0;
    while i < nums.len() {
        let cmd = if i == 0 { 'M' } else { 'L' };
        d.push_str(&format!("{}{},{} ", cmd, fmt_num(nums[i]), fmt_num(nums[i + 1])));
        i += 2;
    }
    if close { d.push('Z'); }
    Some(d.trim().to_string())
}
