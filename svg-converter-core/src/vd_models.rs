// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Lightweight reverse-direction model: VectorDrawable XML -> this tree ->
//! SVG. Deliberately separate from models.rs (NormalizedSvg is the forward
//! SVG->VD model; reusing it for the reverse direction would force awkward
//! round-tripping through concepts, like SVG-specific normalization, that
//! don't apply when reading VD XML back in).

#[derive(Debug, Clone, PartialEq)]
pub struct RevDoc {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub width: f32,
    pub height: f32,
    pub alpha: f32,
    pub nodes: Vec<RevNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RevNode {
    Path(RevPath),
    Group(RevGroup),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RevGroup {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    /// Normalized pathData from this group's <clip-path> child, if any.
    pub clip_path: Option<String>,
    pub children: Vec<RevNode>,
}

impl RevGroup {
    pub fn is_identity_transform(&self) -> bool {
        self.translate_x == 0.0 && self.translate_y == 0.0
            && self.scale_x == 1.0 && self.scale_y == 1.0
            && self.rotation == 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevGradientStop {
    pub offset: f32,
    pub color: String, // #AARRGGBB
}

#[derive(Debug, Clone, PartialEq)]
pub enum RevGradient {
    Linear { x1: f32, y1: f32, x2: f32, y2: f32, stops: Vec<RevGradientStop> },
    Radial { cx: f32, cy: f32, r: f32, stops: Vec<RevGradientStop> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RevFill {
    None,
    Solid(String), // #AARRGGBB
    Gradient(RevGradient),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevPath {
    pub path_data: String,
    pub fill: RevFill,
    pub stroke_color: Option<String>,
    pub stroke_width: f32,
    pub fill_type_evenodd: bool,
}
