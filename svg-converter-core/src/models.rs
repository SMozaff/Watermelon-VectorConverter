// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Internal normalized types. Implementation detail of Module R; NOT frozen.

/// A normalized SVG document ready for VectorDrawable emission.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedSvg {
    pub width: f32,
    pub height: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub root_alpha: f32,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Path(VdPath),
    Group(VdGroup),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VdGroup {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    /// Normalized path data for a <clip-path> scoped to this group, if any.
    /// VectorDrawable's <clip-path> is a sibling element (emitted as the
    /// group's first child) that clips everything else drawn in the group.
    pub clip_path: Option<String>,
    pub children: Vec<Node>,
}

impl Default for VdGroup {
    fn default() -> Self {
        VdGroup {
            translate_x: 0.0, translate_y: 0.0,
            scale_x: 1.0, scale_y: 1.0,
            rotation: 0.0, pivot_x: 0.0, pivot_y: 0.0,
            clip_path: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillType { NonZero, EvenOdd }

/// A gradient color stop: offset 0..1 and #AARRGGBB color.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub offset: f32,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    Linear { x1: f32, y1: f32, x2: f32, y2: f32, stops: Vec<GradientStop> },
    Radial { cx: f32, cy: f32, r: f32, stops: Vec<GradientStop> },
}

/// A fill is either a solid color, a gradient, or none.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    None,
    Solid(String),        // #AARRGGBB
    Gradient(Gradient),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VdPath {
    pub path_data: String,
    pub fill: Fill,
    pub stroke_color: Option<String>,
    pub stroke_width: f32,
    pub fill_type: FillType,
}
