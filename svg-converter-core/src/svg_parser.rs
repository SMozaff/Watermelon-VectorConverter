// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! SVG parsing & normalization (Contract C-1).
//! Rejects out-of-scope features with UnsupportedFeature(1002).

use crate::error::ConversionError;
use crate::models::*;
use crate::utils::{fmt_num, parse_rgb, to_aarrggbb};
use crate::shapes;
use crate::gradients;
use crate::models::Fill;
use std::collections::BTreeMap;
use crate::models::Gradient;

const UNSUPPORTED: &[&str] = &[
    "filter", "text", "tspan", "pattern", "mask", "image", "use",
];

/// Inheritable SVG presentation properties, cascaded from ancestor <g>
/// elements down to leaf shapes — mirrors the subset of CSS inheritance SVG
/// actually needs for fill/stroke. Each field is the *resolved string value*
/// as it would appear in an attribute (e.g. "none", "#ff0000", "url(#g1)"),
/// so downstream parsing (parse_rgb, gradients::url_ref) is unchanged.
#[derive(Debug, Clone)]
struct StyleContext {
    fill: Option<String>,
    stroke: Option<String>,
    fill_opacity: Option<f32>,
    stroke_opacity: Option<f32>,
    opacity: Option<f32>,
    stroke_width: Option<f32>,
    fill_rule: Option<String>,
}

impl StyleContext {
    fn root() -> Self {
        StyleContext {
            fill: None, stroke: None,
            fill_opacity: None, stroke_opacity: None, opacity: None,
            stroke_width: None, fill_rule: None,
        }
    }

    /// Merge this element's own attributes/inline style over the inherited
    /// context, producing the context to use for this element AND to pass
    /// down to its children (SVG presentation properties inherit by default
    /// unless the child re-specifies them).
    fn cascade(&self, el: &roxmltree::Node) -> StyleContext {
        let inline = parse_inline_style(el.attribute("style").unwrap_or(""));
        let get = |name: &str| -> Option<String> {
            inline.get(name).cloned().or_else(|| el.attribute(name).map(|s| s.to_string()))
        };
        StyleContext {
            fill: get("fill").or_else(|| self.fill.clone()),
            stroke: get("stroke").or_else(|| self.stroke.clone()),
            fill_opacity: get("fill-opacity").and_then(|s| s.parse().ok()).or(self.fill_opacity),
            stroke_opacity: get("stroke-opacity").and_then(|s| s.parse().ok()).or(self.stroke_opacity),
            opacity: get("opacity").and_then(|s| s.parse().ok()).or(self.opacity),
            stroke_width: get("stroke-width").and_then(|s| s.parse().ok()).or(self.stroke_width),
            fill_rule: get("fill-rule").or_else(|| self.fill_rule.clone()),
        }
    }
}

/// Parse `style="fill:#ff0000; stroke: none; fill-opacity:0.5"` into a map.
/// Deliberately minimal: no CSS specificity, no selectors, no !important —
/// just the inline-style declaration list SVG exporters commonly emit.
fn parse_inline_style(style: &str) -> std::collections::HashMap<String, String> {
    style
        .split(';')
        .filter_map(|decl| {
            let mut parts = decl.splitn(2, ':');
            let prop = parts.next()?.trim();
            let val = parts.next()?.trim();
            if prop.is_empty() || val.is_empty() { return None; }
            Some((prop.to_string(), val.to_string()))
        })
        .collect()
}
fn collect_clip_paths(doc: &roxmltree::Document) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("clipPath")) {
        let id = match node.attribute("id") { Some(i) => i.to_string(), None => continue };
        for child in node.children().filter(|n| n.is_element()) {
            let tag = child.tag_name().name();
            let raw = match tag {
                "path" => child.attribute("d").map(|s| s.to_string()),
                "rect" => shapes::rect_to_path(&child),
                "circle" => shapes::circle_to_path(&child),
                "ellipse" => shapes::ellipse_to_path(&child),
                "polygon" => shapes::polygon_to_path(&child),
                _ => None,
            };
            if let Some(d) = raw {
                if let Ok(normalized) = normalize_path_data(&d) {
                    map.insert(id, normalized);
                    break;
                }
            }
        }
    }
    map
}

/// Resolve a clip-path="url(#id)" attribute against the collected defs.
fn resolve_clip_path(el: &roxmltree::Node, clips: &BTreeMap<String, String>) -> Option<String> {
    let raw = el.attribute("clip-path")?;
    let id = gradients::url_ref(raw)?;
    clips.get(id).cloned()
}

pub fn parse(svg_bytes: &[u8]) -> Result<NormalizedSvg, ConversionError> {
    let text = std::str::from_utf8(svg_bytes)
        .map_err(|e| ConversionError::InvalidSvg(format!("not UTF-8: {e}")))?;
    let doc = roxmltree::Document::parse(text)
        .map_err(|e| ConversionError::InvalidSvg(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(ConversionError::InvalidSvg("root is not <svg>".into()));
    }

    // Reject unsupported features anywhere in the tree (no silent drop).
    for node in doc.descendants().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        if UNSUPPORTED.contains(&name) {
            return Err(ConversionError::UnsupportedFeature(name.to_string()));
        }
    }

    let (vw, vh) = parse_viewbox(&root)?;
    let width = svg_attr_f32(&root, "width").unwrap_or(vw);
    let height = svg_attr_f32(&root, "height").unwrap_or(vh);
    let root_alpha = svg_attr_f32(&root, "opacity").unwrap_or(1.0);

    let grads = gradients::collect(&doc);
    let clips = collect_clip_paths(&doc);
    let root_style = StyleContext::root().cascade(&root);
    let mut nodes = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        if let Some(n) = parse_node(&child, &grads, &clips, &root_style)? {
            nodes.push(n);
        }
    }

    Ok(NormalizedSvg {
        width, height, viewport_w: vw, viewport_h: vh, root_alpha, nodes,
    })
}

fn parse_viewbox(root: &roxmltree::Node) -> Result<(f32, f32), ConversionError> {
    if let Some(vb) = root.attribute("viewBox") {
        let parts: Vec<f32> = vb.split_whitespace()
            .filter_map(|s| s.parse().ok()).collect();
        if parts.len() == 4 {
            return Ok((parts[2], parts[3]));
        }
        return Err(ConversionError::InvalidSvg("bad viewBox".into()));
    }
    // Fall back to width/height
    let w = svg_attr_f32(root, "width");
    let h = svg_attr_f32(root, "height");
    match (w, h) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err(ConversionError::InvalidSvg("no viewBox or width/height".into())),
    }
}

fn parse_node(el: &roxmltree::Node, grads: &BTreeMap<String, Gradient>, clips: &BTreeMap<String, String>, ctx: &StyleContext) -> Result<Option<Node>, ConversionError> {
    let tag = el.tag_name().name();
    let own_ctx = ctx.cascade(el);
    match tag {
        "path" => {
            let d = el.attribute("d")
                .ok_or_else(|| ConversionError::InvalidSvg("<path> missing d".into()))?;
            let path_data = normalize_path_data(d)?;
            let vd_path = style_path(el, path_data, grads, &own_ctx)?;
            // If the <path> itself carries a transform (common in VTracer output
            // where every path has translate(x,y) instead of being inside a <g>),
            // wrap it in a VdGroup so the position is preserved. Without this,
            // every translated path lands at (0,0) and the image is scrambled.
            let own_clip = resolve_clip_path(el, clips);
            if el.attribute("transform").is_some() || own_clip.is_some() {
                let mut group = parse_group_transform(el);
                group.clip_path = own_clip;
                group.children.push(Node::Path(vd_path));
                Ok(Some(Node::Group(group)))
            } else {
                Ok(Some(Node::Path(vd_path)))
            }
        }
        "g" => {
            let mut group = parse_group_transform(el);
            group.clip_path = resolve_clip_path(el, clips);
            for child in el.children().filter(|n| n.is_element()) {
                if let Some(n) = parse_node(&child, grads, clips, &own_ctx)? {
                    group.children.push(n);
                }
            }
            Ok(Some(Node::Group(group)))
        }
        // Tier 1: basic shapes -> path data, then styled like any path.
        "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => {
            let raw = match tag {
                "rect" => shapes::rect_to_path(el),
                "circle" => shapes::circle_to_path(el),
                "ellipse" => shapes::ellipse_to_path(el),
                "line" => shapes::line_to_path(el),
                "polyline" => shapes::polyline_to_path(el),
                "polygon" => shapes::polygon_to_path(el),
                _ => None,
            };
            match raw {
                Some(d) => {
                    let path_data = normalize_path_data(&d)?;
                    let vd_path = style_path(el, path_data, grads, &own_ctx)?;
                    let own_clip = resolve_clip_path(el, clips);
                    if own_clip.is_some() {
                        let mut group = VdGroup { clip_path: own_clip, ..VdGroup::default() };
                        group.children.push(Node::Path(vd_path));
                        Ok(Some(Node::Group(group)))
                    } else {
                        Ok(Some(Node::Path(vd_path)))
                    }
                }
                None => Ok(None), // degenerate shape (e.g. zero size) -> skip
            }
        }
        _ => Ok(None), // ignore <title>, <desc>, <defs>, etc.
    }
}

/// Apply fill/stroke/opacity styling to an already-normalized path-data string.
fn style_path(el: &roxmltree::Node, path_data: String, grads: &BTreeMap<String, Gradient>, ctx: &StyleContext)
    -> Result<VdPath, ConversionError>
{
    let opacity = ctx.opacity.unwrap_or(1.0);
    let fill_opacity = ctx.fill_opacity.unwrap_or(1.0) * opacity;
    let stroke_opacity = ctx.stroke_opacity.unwrap_or(1.0) * opacity;

    // SVG default fill is black; "none" disables; url(#id) is a gradient.
    // ctx.fill already reflects inline style="" and ancestor <g fill> —
    // this element's own fill/style (if any) always wins, per cascade().
    let fill_raw = ctx.fill.as_deref().unwrap_or("black");
    let fill = if fill_raw.trim().eq_ignore_ascii_case("none") {
        Fill::None
    } else if let Some(id) = gradients::url_ref(fill_raw) {
        match grads.get(id) {
            Some(g) => Fill::Gradient(g.clone()),
            None => Fill::None, // dangling reference -> nothing drawn (don't crash)
        }
    } else {
        match parse_rgb(fill_raw) {
            Some(rgb) => Fill::Solid(to_aarrggbb(rgb, fill_opacity)),
            None => Fill::None,
        }
    };

    let stroke_color = ctx.stroke.as_deref()
        .and_then(parse_rgb)
        .map(|rgb| to_aarrggbb(rgb, stroke_opacity));
    let stroke_width = ctx.stroke_width.unwrap_or(0.0);

    let fill_type = match ctx.fill_rule.as_deref() {
        Some("evenodd") => FillType::EvenOdd,
        _ => FillType::NonZero,
    };

    // Suppress unused-warning for el: kept in the signature since callers
    // already have it in scope and future style needs (e.g. id-based CSS
    // class matching) will likely need it again.
    let _ = el;

    Ok(VdPath { path_data, fill, stroke_color, stroke_width, fill_type })
}

fn parse_group_transform(el: &roxmltree::Node) -> VdGroup {
    let mut g = VdGroup::default();
    if let Some(t) = el.attribute("transform") {
        apply_transform_str(t, &mut g);
    }
    g
}

/// Minimal transform parser: translate/scale/rotate. matrix() -> best-effort skip.
fn apply_transform_str(t: &str, g: &mut VdGroup) {
    let mut rest = t;
    while let Some(open) = rest.find('(') {
        let name = rest[..open].trim().to_string();
        let close = match rest[open..].find(')') { Some(c) => open + c, None => break };
        let args: Vec<f32> = rest[open + 1..close]
            .split([',', ' '])
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        match name.as_str() {
            "translate" => {
                g.translate_x += args.first().copied().unwrap_or(0.0);
                g.translate_y += args.get(1).copied().unwrap_or(0.0);
            }
            "scale" => {
                let sx = args.first().copied().unwrap_or(1.0);
                g.scale_x *= sx;
                g.scale_y *= args.get(1).copied().unwrap_or(sx);
            }
            "rotate" => {
                g.rotation += args.first().copied().unwrap_or(0.0);
                if args.len() >= 3 { g.pivot_x = args[1]; g.pivot_y = args[2]; }
            }
            _ => {} // matrix/skew: out of v1 scope, left as identity
        }
        rest = &rest[close + 1..];
    }
}

/// Normalize path data: tokenize, convert relative->absolute, re-emit with
/// commas between coordinate pairs. Supports M L H V C S Q T Z (+ relative).
fn normalize_path_data(d: &str) -> Result<String, ConversionError> {
    let tokens = tokenize(d)?;
    let mut out = String::new();
    let mut i = 0;
    let (mut cx, mut cy) = (0.0f32, 0.0f32);   // current point
    let (mut sx, mut sy) = (0.0f32, 0.0f32);   // subpath start
    // last control point of the previous C/S (for S) and Q/T (for T) reflection
    let (mut last_cubic_ctrl, mut last_quad_ctrl): (Option<(f32, f32)>, Option<(f32, f32)>) = (None, None);
    let mut last_cmd = ' ';

    macro_rules! num { () => {{
        let v = match tokens.get(i) { Some(Token::Num(n)) => *n, _ =>
            return Err(ConversionError::InvalidSvg("expected number in path".into())) };
        i += 1; v
    }}; }

    while i < tokens.len() {
        let cmd = match &tokens[i] {
            Token::Cmd(c) => { i += 1; last_cmd = *c; *c }
            Token::Num(_) => implicit_cmd(last_cmd),
        };
        let abs = cmd.is_ascii_uppercase();
        let upper = cmd.to_ascii_uppercase();
        // Any command other than C/S clears the cubic reflection point; other
        // than Q/T clears the quad one.
        match upper {
            'C' | 'S' => {}
            _ => last_cubic_ctrl = None,
        }
        match upper {
            'Q' | 'T' => {}
            _ => last_quad_ctrl = None,
        }
        match upper {
            'M' => {
                let (mut x, mut y) = (num!(), num!());
                if !abs { x += cx; y += cy; }
                cx = x; cy = y; sx = x; sy = y;
                out.push_str(&format!("M{},{} ", fmt_num(x), fmt_num(y)));
                last_cmd = if abs { 'L' } else { 'l' };
            }
            'L' => {
                let (mut x, mut y) = (num!(), num!());
                if !abs { x += cx; y += cy; }
                cx = x; cy = y;
                out.push_str(&format!("L{},{} ", fmt_num(x), fmt_num(y)));
            }
            'H' => {
                let mut x = num!(); if !abs { x += cx; }
                cx = x;
                out.push_str(&format!("L{},{} ", fmt_num(x), fmt_num(cy)));
            }
            'V' => {
                let mut y = num!(); if !abs { y += cy; }
                cy = y;
                out.push_str(&format!("L{},{} ", fmt_num(cx), fmt_num(y)));
            }
            'C' => {
                let mut c = [0f32; 6];
                for k in 0..6 { c[k] = num!(); if !abs { c[k] += if k % 2 == 0 { cx } else { cy }; } }
                last_cubic_ctrl = Some((c[2], c[3]));
                cx = c[4]; cy = c[5];
                out.push_str(&format!("C{},{} {},{} {},{} ",
                    fmt_num(c[0]), fmt_num(c[1]), fmt_num(c[2]), fmt_num(c[3]), fmt_num(c[4]), fmt_num(c[5])));
            }
            'S' => {
                // smooth cubic: first control = reflection of previous cubic ctrl
                let mut p = [0f32; 4];
                for k in 0..4 { p[k] = num!(); if !abs { p[k] += if k % 2 == 0 { cx } else { cy }; } }
                let (rx, ry) = match last_cubic_ctrl {
                    Some((px, py)) => (2.0 * cx - px, 2.0 * cy - py),
                    None => (cx, cy), // no previous cubic: first ctrl = current point
                };
                last_cubic_ctrl = Some((p[0], p[1]));
                cx = p[2]; cy = p[3];
                out.push_str(&format!("C{},{} {},{} {},{} ",
                    fmt_num(rx), fmt_num(ry), fmt_num(p[0]), fmt_num(p[1]), fmt_num(p[2]), fmt_num(p[3])));
            }
            'Q' => {
                let mut c = [0f32; 4];
                for k in 0..4 { c[k] = num!(); if !abs { c[k] += if k % 2 == 0 { cx } else { cy }; } }
                last_quad_ctrl = Some((c[0], c[1]));
                cx = c[2]; cy = c[3];
                out.push_str(&format!("Q{},{} {},{} ",
                    fmt_num(c[0]), fmt_num(c[1]), fmt_num(c[2]), fmt_num(c[3])));
            }
            'T' => {
                // smooth quad: control = reflection of previous quad ctrl
                let mut p = [0f32; 2];
                for k in 0..2 { p[k] = num!(); if !abs { p[k] += if k % 2 == 0 { cx } else { cy }; } }
                let (qx, qy) = match last_quad_ctrl {
                    Some((px, py)) => (2.0 * cx - px, 2.0 * cy - py),
                    None => (cx, cy),
                };
                last_quad_ctrl = Some((qx, qy));
                out.push_str(&format!("Q{},{} {},{} ",
                    fmt_num(qx), fmt_num(qy), fmt_num(p[0]), fmt_num(p[1])));
                cx = p[0]; cy = p[1];
            }
            'A' => {
                // elliptical arc -> cubic Beziers (Tier 2)
                let rx = num!(); let ry = num!(); let rot = num!();
                let large = num!() != 0.0; let sweep = num!() != 0.0;
                let (mut x, mut y) = (num!(), num!());
                if !abs { x += cx; y += cy; }
                let cubics = crate::arc::arc_to_cubics(
                    cx as f64, cy as f64, rx as f64, ry as f64, rot as f64,
                    large, sweep, x as f64, y as f64,
                );
                for c in cubics {
                    out.push_str(&format!("C{},{} {},{} {},{} ",
                        fmt_num(c[0] as f32), fmt_num(c[1] as f32),
                        fmt_num(c[2] as f32), fmt_num(c[3] as f32),
                        fmt_num(c[4] as f32), fmt_num(c[5] as f32)));
                }
                cx = x; cy = y;
            }
            'Z' => {
                cx = sx; cy = sy;
                out.push_str("Z ");
            }
            other => {
                return Err(ConversionError::InvalidSvg(
                    format!("unknown path command '{other}'")));
            }
        }
    }
    Ok(out.trim_end().to_string())
}

fn implicit_cmd(last: char) -> char {
    // After an M, implicit pairs are L (matching case handled by caller via last_cmd)
    last
}

#[derive(Debug)]
enum Token { Cmd(char), Num(f32) }

fn tokenize(d: &str) -> Result<Vec<Token>, ConversionError> {
    let mut tokens = Vec::new();
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() || c == ',' { i += 1; continue; }
        if c.is_ascii_alphabetic() {
            tokens.push(Token::Cmd(c));
            i += 1;
            continue;
        }
        // number: optional sign, digits, dot, exponent
        let start = i;
        if c == '+' || c == '-' { i += 1; }
        let mut seen_dot = false;
        while i < bytes.len() {
            let ch = bytes[i] as char;
            if ch.is_ascii_digit() { i += 1; }
            else if ch == '.' && !seen_dot { seen_dot = true; i += 1; }
            else if (ch == 'e' || ch == 'E') && i + 1 < bytes.len() {
                i += 1;
                if (bytes[i] as char) == '+' || (bytes[i] as char) == '-' { i += 1; }
            } else { break; }
        }
        if i == start { return Err(ConversionError::InvalidSvg(format!("bad char in path: {c}"))); }
        let num: f32 = d[start..i].parse()
            .map_err(|_| ConversionError::InvalidSvg(format!("bad number: {}", &d[start..i])))?;
        tokens.push(Token::Num(num));
    }
    Ok(tokens)
}

fn svg_attr_f32(el: &roxmltree::Node, name: &str) -> Option<f32> {
    el.attribute(name).and_then(|s| {
        let t = s.trim_end_matches("px").trim();
        t.parse().ok()
    })
}