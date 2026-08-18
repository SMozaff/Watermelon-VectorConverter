// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

//! Structural analysis of a vector file, for the file-manager properties panel.
//! Reuses the existing parser so the report is authoritative (it reflects what
//! the converter actually found, not a string-scan guess).

use crate::error::ConversionError;
use crate::models::{Fill, Node, NormalizedSvg, VdPath};
use crate::svg_parser;
use crate::vd_models::{RevFill, RevNode, RevPath};
use crate::vd_parser;

/// What graphical structures a vector file contains. Plain data, serializable
/// across the FFI boundary as primitive fields.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorAnalysis {
    pub width: f32,
    pub height: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub path_count: u32,
    pub group_count: u32,
    pub uses_paths: bool,
    pub uses_gradients: bool,
    pub uses_solid_colors: bool,
    pub uses_strokes: bool,
    /// True if every painted fill/stroke resolves to one single solid color,
    /// i.e. the art could be recolored with a single tint. Heuristic.
    pub single_color_tintable: bool,
    /// The single color (if single_color_tintable), as #AARRGGBB.
    pub tint_color: Option<String>,
    /// Whether this is an animated vector. Detection only — playback is the
    /// deferred C-5 engine. SVG SMIL/CSS animation is detected separately (it
    /// is not representable in NormalizedSvg), see `detect_animation_marker`.
    pub is_animated: bool,
}

/// Analyze a (static) SVG or VD file by parsing it and walking the tree.
pub fn analyze(bytes: &[u8]) -> Result<VectorAnalysis, ConversionError> {
    // Animation markers must be checked on the RAW bytes first, because the
    // normalizing parser discards animation elements it cannot represent.
    let animated = detect_animation_marker(bytes);

    let svg: NormalizedSvg = svg_parser::parse(bytes)?;

    let mut acc = Acc::default();
    for node in &svg.nodes {
        walk(node, &mut acc);
    }

    let single = acc.distinct_colors.len() == 1 && !acc.uses_gradients;
    let tint = if single {
        acc.distinct_colors.iter().next().cloned()
    } else {
        None
    };

    Ok(VectorAnalysis {
        width: svg.width,
        height: svg.height,
        viewport_w: svg.viewport_w,
        viewport_h: svg.viewport_h,
        path_count: acc.path_count,
        group_count: acc.group_count,
        uses_paths: acc.path_count > 0,
        uses_gradients: acc.uses_gradients,
        uses_solid_colors: acc.uses_solid,
        uses_strokes: acc.uses_strokes,
        single_color_tintable: single,
        tint_color: tint,
        is_animated: animated,
    })
}

#[derive(Default)]
struct Acc {
    path_count: u32,
    group_count: u32,
    uses_gradients: bool,
    uses_solid: bool,
    uses_strokes: bool,
    distinct_colors: std::collections::BTreeSet<String>,
}

fn walk(node: &Node, acc: &mut Acc) {
    match node {
        Node::Path(p) => {
            acc.path_count += 1;
            account_path(p, acc);
        }
        Node::Group(g) => {
            acc.group_count += 1;
            for child in &g.children {
                walk(child, acc);
            }
        }
    }
}

fn account_path(p: &VdPath, acc: &mut Acc) {
    match &p.fill {
        Fill::Solid(c) => {
            acc.uses_solid = true;
            acc.distinct_colors.insert(c.clone());
        }
        Fill::Gradient(_) => {
            acc.uses_gradients = true;
        }
        Fill::None => {}
    }
    if let Some(sc) = &p.stroke_color {
        acc.uses_strokes = true;
        acc.distinct_colors.insert(sc.clone());
    }
}

/// Analyze a VectorDrawable XML file (the reverse direction's input format).
/// Mirrors [analyze] exactly, but walks the RevNode tree from vd_parser
/// instead of NormalizedSvg from svg_parser — kept as a separate function so
/// the existing, already-shipped analyze() is never touched.
pub fn analyze_vd(bytes: &[u8]) -> Result<VectorAnalysis, ConversionError> {
    let animated = detect_animation_marker(bytes);
    let doc = vd_parser::parse(bytes)?;

    let mut acc = Acc::default();
    for node in &doc.nodes {
        walk_rev(node, &mut acc);
    }

    let single = acc.distinct_colors.len() == 1 && !acc.uses_gradients;
    let tint = if single {
        acc.distinct_colors.iter().next().cloned()
    } else {
        None
    };

    Ok(VectorAnalysis {
        width: doc.width,
        height: doc.height,
        viewport_w: doc.viewport_w,
        viewport_h: doc.viewport_h,
        path_count: acc.path_count,
        group_count: acc.group_count,
        uses_paths: acc.path_count > 0,
        uses_gradients: acc.uses_gradients,
        uses_solid_colors: acc.uses_solid,
        uses_strokes: acc.uses_strokes,
        single_color_tintable: single,
        tint_color: tint,
        is_animated: animated,
    })
}

fn walk_rev(node: &RevNode, acc: &mut Acc) {
    match node {
        RevNode::Path(p) => {
            acc.path_count += 1;
            account_rev_path(p, acc);
        }
        RevNode::Group(g) => {
            acc.group_count += 1;
            for child in &g.children {
                walk_rev(child, acc);
            }
        }
    }
}

fn account_rev_path(p: &RevPath, acc: &mut Acc) {
    match &p.fill {
        RevFill::Solid(c) => {
            acc.uses_solid = true;
            acc.distinct_colors.insert(c.clone());
        }
        RevFill::Gradient(_) => {
            acc.uses_gradients = true;
        }
        RevFill::None => {}
    }
    if let Some(sc) = &p.stroke_color {
        acc.uses_strokes = true;
        acc.distinct_colors.insert(sc.clone());
    }
}

/// Cheap raw-bytes scan for animation markers that the normalizing parser
/// drops. Covers Android AVD (`<animated-vector>`), SVG SMIL (`<animate`,
/// `<animateTransform`), and CSS keyframe animation (`@keyframes`).
pub fn detect_animation_marker(bytes: &[u8]) -> bool {
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => return false,
    };
    text.contains("<animated-vector")
        || text.contains("<animate")          // covers <animate and <animateTransform
        || text.contains("@keyframes")
}