// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! RevDoc -> SVG XML (reverse-direction emission, Contract C-4).

use crate::utils::fmt_num;
use crate::vd_models::*;

pub fn emit(doc: &RevDoc) -> String {
    let mut defs = String::new();
    let mut body = String::new();
    let mut ctx = EmitCtx { next_id: 0, defs: &mut defs };

    for node in &doc.nodes {
        emit_node(node, 1, &mut body, &mut ctx);
    }

    let mut s = String::new();
    s.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"\n");
    s.push_str(&format!("    width=\"{}\" height=\"{}\"\n", fmt_num(doc.width), fmt_num(doc.height)));
    s.push_str(&format!(
        "    viewBox=\"0 0 {} {}\"",
        fmt_num(doc.viewport_w), fmt_num(doc.viewport_h),
    ));
    if doc.alpha < 1.0 {
        s.push_str(&format!("\n    opacity=\"{}\"", fmt_num(doc.alpha)));
    }
    s.push_str(">\n");
    if !defs.is_empty() {
        s.push_str("  <defs>\n");
        s.push_str(&defs);
        s.push_str("  </defs>\n");
    }
    s.push_str(&body);
    s.push_str("</svg>\n");
    s
}

struct EmitCtx<'a> {
    next_id: u32,
    defs: &'a mut String,
}

impl<'a> EmitCtx<'a> {
    fn fresh_id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }
}

fn indent(level: usize) -> String { "  ".repeat(level) }

fn emit_node(node: &RevNode, level: usize, s: &mut String, ctx: &mut EmitCtx) {
    match node {
        RevNode::Path(p) => emit_path(p, level, s, ctx),
        RevNode::Group(g) => emit_group(g, level, s, ctx),
    }
}

fn emit_group(g: &RevGroup, level: usize, s: &mut String, ctx: &mut EmitCtx) {
    let pad = indent(level);

    // clip-path, if present, becomes a <clipPath> def referenced by this <g>.
    let clip_attr = if let Some(cp) = &g.clip_path {
        let id = ctx.fresh_id("clip");
        ctx.defs.push_str(&format!(
            "    <clipPath id=\"{id}\"><path d=\"{}\"/></clipPath>\n", cp,
        ));
        Some(id)
    } else {
        None
    };

    s.push_str(&format!("{pad}<g"));
    if let Some(id) = &clip_attr {
        s.push_str(&format!(" clip-path=\"url(#{id})\""));
    }
    if !g.is_identity_transform() {
        let mut parts = Vec::new();
        if g.translate_x != 0.0 || g.translate_y != 0.0 {
            parts.push(format!("translate({},{})", fmt_num(g.translate_x), fmt_num(g.translate_y)));
        }
        if g.rotation != 0.0 {
            if g.pivot_x != 0.0 || g.pivot_y != 0.0 {
                parts.push(format!("rotate({},{},{})", fmt_num(g.rotation), fmt_num(g.pivot_x), fmt_num(g.pivot_y)));
            } else {
                parts.push(format!("rotate({})", fmt_num(g.rotation)));
            }
        }
        if g.scale_x != 1.0 || g.scale_y != 1.0 {
            parts.push(format!("scale({},{})", fmt_num(g.scale_x), fmt_num(g.scale_y)));
        }
        if !parts.is_empty() {
            s.push_str(&format!(" transform=\"{}\"", parts.join(" ")));
        }
    }
    s.push_str(">\n");
    for child in &g.children {
        emit_node(child, level + 1, s, ctx);
    }
    s.push_str(&format!("{pad}</g>\n"));
}

fn emit_path(p: &RevPath, level: usize, s: &mut String, ctx: &mut EmitCtx) {
    let pad = indent(level);
    s.push_str(&format!("{pad}<path d=\"{}\"", p.path_data));

    match &p.fill {
        RevFill::None => s.push_str(" fill=\"none\""),
        RevFill::Solid(c) => s.push_str(&format!(" fill=\"{}\"", to_svg_color(c))),
        RevFill::Gradient(grad) => {
            let id = ctx.fresh_id("grad");
            emit_gradient_def(grad, &id, ctx.defs);
            s.push_str(&format!(" fill=\"url(#{id})\""));
        }
    }

    if let Some(sc) = &p.stroke_color {
        s.push_str(&format!(" stroke=\"{}\"", to_svg_color(sc)));
        if p.stroke_width > 0.0 {
            s.push_str(&format!(" stroke-width=\"{}\"", fmt_num(p.stroke_width)));
        }
    }
    if p.fill_type_evenodd {
        s.push_str(" fill-rule=\"evenodd\"");
    }
    s.push_str("/>\n");
}

fn emit_gradient_def(g: &RevGradient, id: &str, defs: &mut String) {
    match g {
        RevGradient::Linear { x1, y1, x2, y2, stops } => {
            defs.push_str(&format!(
                "    <linearGradient id=\"{id}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">\n",
                fmt_num(*x1), fmt_num(*y1), fmt_num(*x2), fmt_num(*y2),
            ));
            emit_stops(stops, defs);
            defs.push_str("    </linearGradient>\n");
        }
        RevGradient::Radial { cx, cy, r, stops } => {
            defs.push_str(&format!(
                "    <radialGradient id=\"{id}\" cx=\"{}\" cy=\"{}\" r=\"{}\">\n",
                fmt_num(*cx), fmt_num(*cy), fmt_num(*r),
            ));
            emit_stops(stops, defs);
            defs.push_str("    </radialGradient>\n");
        }
    }
}

fn emit_stops(stops: &[RevGradientStop], defs: &mut String) {
    for st in stops {
        let (color, opacity) = split_alpha(&st.color);
        defs.push_str(&format!(
            "      <stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>\n",
            fmt_num(st.offset), color, opacity,
        ));
    }
}

/// VD colors are #AARRGGBB; SVG wants #RRGGBB + separate opacity.
fn to_svg_color(aarrggbb: &str) -> String {
    split_alpha(aarrggbb).0
}

fn split_alpha(aarrggbb: &str) -> (String, String) {
    let s = aarrggbb.trim_start_matches('#');
    if s.len() == 8 {
        let a = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        (format!("#{}", &s[2..8]), fmt_num(a as f32 / 255.0))
    } else {
        (format!("#{s}"), "1".to_string())
    }
}
