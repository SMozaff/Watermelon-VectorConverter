// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! AVD frame rendering (Contract C-5.2).
//!
//! Parses an `<animated-vector>` document (base `<vector>` + one or more
//! `android:name`-targeted animators, most commonly inlined via
//! `<aapt:attr name="android:animation"><set>...</set></aapt:attr>`),
//! evaluates the animation timeline at a sampled set of frame times, and
//! renders each frame to PNG via the EXISTING resvg-based renderer
//! (`vector_drawable::emit` + `image_export::render_vd_preview`).
//!
//! This module intentionally does NOT reuse `models::NormalizedSvg` as its
//! working document type: `NormalizedSvg`/`VdPath`/`VdGroup` (and the
//! reverse-direction `RevDoc`/`RevPath`/`RevGroup`) have no node-identity
//! field, but AVD animator targets are matched by `android:name` against a
//! specific node — a concept neither existing model carries. Rather than
//! adding a `name` field to those frozen-adjacent shared models, this
//! module parses the embedded `<vector>` into its own small `AvdNode` tree
//! that carries `name: Option<String>`, evaluates animations against that
//! tree, and only converts to `NormalizedSvg` at the final render step
//! (reusing the existing emit+rasterize pipeline unchanged).

use crate::error::ConversionError;
use crate::models::{Fill, FillType, NormalizedSvg, Node as VdNode, VdGroup, VdPath};

const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";
const PX_MIN: u32 = 16;
const PX_MAX: u32 = 2048;
const FPS_MIN: u32 = 1;
const FPS_MAX: u32 = 60;
const DEFAULT_MAX_FRAMES: u32 = 90;

/// Result of C-5.2: a pre-rendered sequence of frames for an AVD.
#[derive(Debug, Clone)]
pub struct AnimationFrames {
    pub width: u32,
    pub height: u32,
    /// One entry per frame — supports variable per-frame timing.
    pub frame_durations_ms: Vec<u32>,
    /// One PNG (sRGB) per frame; same length as frame_durations_ms.
    pub frames: Vec<Vec<u8>>,
    pub loop_mode: LoopMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Once,
    Repeat,
    Reverse,
}

/// C-5.2: render an AVD's frames.
///
/// Validation is enforced up front (frozen contract): `px` 16..=2048,
/// `fps` 1..=60. `max_frames` of 0 is treated as the contract default (90)
/// rather than an error, matching "max_frames default 90" in the Section 5
/// pipeline notes — 0 isn't a meaningful frame cap, so we don't want a
/// caller's zero-initialized default to silently produce zero frames.
pub fn render_avd_frames(
    avd_bytes: &[u8],
    fps: u32,
    max_frames: u32,
    px: u32,
) -> Result<AnimationFrames, ConversionError> {
    if !(PX_MIN..=PX_MAX).contains(&px) {
        return Err(ConversionError::RenderError(format!(
            "px {px} out of range {PX_MIN}..={PX_MAX}"
        )));
    }
    if !(FPS_MIN..=FPS_MAX).contains(&fps) {
        return Err(ConversionError::RenderError(format!(
            "fps {fps} out of range {FPS_MIN}..={FPS_MAX}"
        )));
    }
    let max_frames = if max_frames == 0 { DEFAULT_MAX_FRAMES } else { max_frames };

    let doc = parse_avd_bundle(avd_bytes)?;
    let total_duration_ms = compute_timeline(&doc);
    let frame_times = sample_frame_times(total_duration_ms, fps, max_frames);

    let natural_frame_count =
        ((total_duration_ms as u64 * fps as u64) / 1000).max(1) as u32;
    let loop_mode = if natural_frame_count > max_frames {
        LoopMode::Once
    } else {
        default_loop_mode()
    };

    let mut frames = Vec::with_capacity(frame_times.len());
    let mut frame_durations_ms = Vec::with_capacity(frame_times.len());
    let frame_dur = if fps > 0 { (1000 / fps).max(1) } else { 1 };

    for &t in &frame_times {
        let evaluated = evaluate_frame(&doc, t);
        let png = render_frame(&evaluated, &doc, px)?;
        frames.push(png);
        frame_durations_ms.push(frame_dur);
    }

    Ok(AnimationFrames {
        width: px,
        height: px,
        frame_durations_ms,
        frames,
        loop_mode,
    })
}

// ---------------------------------------------------------------------
// AVD document model (local to this module — see file header for why).
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AvdDocument {
    base: AvdNode,
    targets: Vec<AnimationTarget>,
    viewport_w: f32,
    viewport_h: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
enum AvdNode {
    Path(AvdPath),
    Group(AvdGroup),
}

#[derive(Debug, Clone)]
struct AvdGroup {
    name: Option<String>,
    translate_x: f32,
    translate_y: f32,
    scale_x: f32,
    scale_y: f32,
    rotation: f32,
    pivot_x: f32,
    pivot_y: f32,
    clip_path: Option<String>,
    children: Vec<AvdNode>,
}

impl Default for AvdGroup {
    fn default() -> Self {
        AvdGroup {
            name: None,
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            pivot_x: 0.0,
            pivot_y: 0.0,
            clip_path: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct AvdPath {
    name: Option<String>,
    path_data: String,
    fill_color: Option<String>, // #AARRGGBB, None = no fill
    stroke_color: Option<String>,
    stroke_width: f32,
    fill_type_evenodd: bool,
    alpha: f32,
}

/// One animator bound to a named node.
#[derive(Debug, Clone)]
struct AnimationTarget {
    target_name: String,
    animators: Vec<PropertyAnimator>,
}

#[derive(Debug, Clone)]
struct PropertyAnimator {
    property: Property,
    start_offset_ms: u32,
    duration_ms: u32,
    interpolator: Interpolator,
    from: AnimValue,
    to: AnimValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Property {
    Rotation,
    TranslateX,
    TranslateY,
    ScaleX,
    ScaleY,
    Alpha,
    FillColor,
    StrokeColor,
    PathData,
}

#[derive(Debug)]
enum AnimValue {
    Number(f32),
    Color(String),
    Path(String),
}

impl Clone for AnimValue {
    fn clone(&self) -> Self {
        match self {
            AnimValue::Number(n) => AnimValue::Number(*n),
            AnimValue::Color(c) => AnimValue::Color(c.clone()),
            AnimValue::Path(p) => AnimValue::Path(p.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interpolator {
    Linear,
    Accelerate,
    Decelerate,
    AccelerateDecelerate,
}

impl Interpolator {
    fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Interpolator::Linear => t,
            Interpolator::Accelerate => t * t,
            Interpolator::Decelerate => 1.0 - (1.0 - t) * (1.0 - t),
            Interpolator::AccelerateDecelerate => {
                0.5 * (1.0 - (std::f32::consts::PI * t).cos())
            }
        }
    }
}

// ---------------------------------------------------------------------
// Step 1: parse
// ---------------------------------------------------------------------

fn parse_avd_bundle(avd_bytes: &[u8]) -> Result<AvdDocument, ConversionError> {
    let text = std::str::from_utf8(avd_bytes)
        .map_err(|e| ConversionError::InvalidSvg(format!("not UTF-8: {e}")))?;
    let doc = roxmltree::Document::parse(text)
        .map_err(|e| ConversionError::InvalidSvg(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "animated-vector" {
        return Err(ConversionError::InvalidSvg(
            "root is not <animated-vector>".into(),
        ));
    }

    // Base vector: either a nested <vector> child directly, or referenced
    // via android:drawable="@drawable/foo" (the true multi-file form isn't
    // resolvable from a single byte blob, so that reference form is
    // reported as unsupported rather than guessed at).
    let vector_el = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "vector")
        .ok_or_else(|| {
            ConversionError::UnsupportedFeature(
                "animated-vector without an inlined <vector> base (external @drawable reference) is not supported".into(),
            )
        })?;
    let base = parse_vector(&vector_el)?;
    let viewport_w = android_f32(&vector_el, "viewportWidth").unwrap_or(24.0);
    let viewport_h = android_f32(&vector_el, "viewportHeight").unwrap_or(24.0);
    // android:width/height carry a "dp" suffix (e.g. "24dp"); fall back to
    // the viewport size (matching the forward converter's own convention)
    // when absent rather than a hardcoded constant.
    let width = android_attr(&vector_el, "width")
        .and_then(|s| s.trim_end_matches("dp").trim().parse().ok())
        .unwrap_or(viewport_w);
    let height = android_attr(&vector_el, "height")
        .and_then(|s| s.trim_end_matches("dp").trim().parse().ok())
        .unwrap_or(viewport_h);

    let mut targets = Vec::new();
    for target_el in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "target")
    {
        let target_name = match android_attr(&target_el, "name") {
            Some(n) => n.to_string(),
            None => continue, // malformed target, skip rather than error
        };

        // The animator is usually inlined: <aapt:attr name="android:animation"><set>...
        // Fall back to nothing (target has no effect) if neither form is present —
        // never error on a single unresolvable target, per "never throws" spirit
        // carried over from C-5.1's detection contract.
        let animator_root = target_el
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "attr")
            .find(|n| n.attribute("name") == Some("android:animation"))
            .and_then(|attr| attr.children().find(|n| n.is_element()));

        let animators = match animator_root {
            Some(el) => parse_animator_tree(&el, 0),
            None => Vec::new(),
        };

        if !animators.is_empty() {
            targets.push(AnimationTarget { target_name, animators });
        }
    }

    Ok(AvdDocument { base, targets, viewport_w, viewport_h, width, height })
}

/// Parse a `<set>` / `<objectAnimator>` tree into a flat list of
/// PropertyAnimators, threading start_offset for `sequentially` ordering.
/// `android:ordering` defaults to "together" (Android's default) when
/// absent — all children start at the same base offset in that case.
fn parse_animator_tree(el: &roxmltree::Node, base_offset_ms: u32) -> Vec<PropertyAnimator> {
    match el.tag_name().name() {
        "objectAnimator" => parse_object_animator(el, base_offset_ms)
            .map(|a| vec![a])
            .unwrap_or_default(),
        "set" => {
            let sequentially = android_attr(el, "ordering") == Some("sequentially");
            let mut result = Vec::new();
            let mut cursor = base_offset_ms;
            for child in el.children().filter(|n| n.is_element()) {
                let child_animators = parse_animator_tree(&child, cursor);
                if sequentially {
                    // Advance the cursor past the longest animator this
                    // child contributed, so the next sibling starts after.
                    if let Some(max_end) = child_animators
                        .iter()
                        .map(|a| a.start_offset_ms + a.duration_ms)
                        .max()
                    {
                        cursor = max_end;
                    }
                }
                result.extend(child_animators);
            }
            result
        }
        _ => Vec::new(),
    }
}

fn parse_object_animator(el: &roxmltree::Node, base_offset_ms: u32) -> Option<PropertyAnimator> {
    let property_name = android_attr(el, "propertyName")?;
    let property = match property_name {
        "rotation" => Property::Rotation,
        "translateX" => Property::TranslateX,
        "translateY" => Property::TranslateY,
        "scaleX" => Property::ScaleX,
        "scaleY" => Property::ScaleY,
        "alpha" => Property::Alpha,
        "fillColor" => Property::FillColor,
        "strokeColor" => Property::StrokeColor,
        "pathData" => Property::PathData,
        _ => return None, // unrecognized property: skip this animator, don't fail the whole file
    };

    let duration_ms = android_attr(el, "duration")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(300); // Android's own ValueAnimator default
    let start_offset_ms = base_offset_ms
        + android_attr(el, "startOffset")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

    let interpolator = match android_attr(el, "interpolator") {
        Some(s) if s.contains("accelerate_decelerate") => Interpolator::AccelerateDecelerate,
        Some(s) if s.contains("accelerate") => Interpolator::Accelerate,
        Some(s) if s.contains("decelerate") => Interpolator::Decelerate,
        Some(s) if s.contains("linear") => Interpolator::Linear,
        _ => Interpolator::Linear, // unresolved -> fall back to linear, never error
    };

    let from_raw = android_attr(el, "valueFrom");
    let to_raw = android_attr(el, "valueTo")?;

    let (from, to) = match property {
        Property::FillColor | Property::StrokeColor => {
            let to_v = AnimValue::Color(to_raw.to_string());
            let from_v = from_raw
                .map(|s| AnimValue::Color(s.to_string()))
                .unwrap_or_else(|| to_v.clone());
            (from_v, to_v)
        }
        Property::PathData => {
            let to_v = AnimValue::Path(to_raw.to_string());
            let from_v = from_raw
                .map(|s| AnimValue::Path(s.to_string()))
                .unwrap_or_else(|| to_v.clone());
            (from_v, to_v)
        }
        _ => {
            let to_n: f32 = to_raw.parse().ok()?;
            let from_n: f32 = from_raw.and_then(|s| s.parse().ok()).unwrap_or(to_n);
            (AnimValue::Number(from_n), AnimValue::Number(to_n))
        }
    };

    Some(PropertyAnimator {
        property,
        start_offset_ms,
        duration_ms,
        interpolator,
        from,
        to,
    })
}

fn parse_vector(el: &roxmltree::Node) -> Result<AvdNode, ConversionError> {
    // The <vector> root itself becomes a top-level group so root alpha and
    // any root-level name target can be handled uniformly with nested groups.
    let mut root_group = AvdGroup {
        name: android_attr(el, "name").map(|s| s.to_string()),
        ..Default::default()
    };
    for child in el.children().filter(|n| n.is_element()) {
        if let Some(n) = parse_avd_node(&child)? {
            root_group.children.push(n);
        }
    }
    Ok(AvdNode::Group(root_group))
}

fn parse_avd_node(el: &roxmltree::Node) -> Result<Option<AvdNode>, ConversionError> {
    match el.tag_name().name() {
        "path" => Ok(Some(AvdNode::Path(parse_avd_path(el)?))),
        "group" => Ok(Some(AvdNode::Group(parse_avd_group(el)?))),
        "clip-path" => Ok(None), // consumed by parent group
        _ => Ok(None),
    }
}

fn parse_avd_group(el: &roxmltree::Node) -> Result<AvdGroup, ConversionError> {
    let mut g = AvdGroup {
        name: android_attr(el, "name").map(|s| s.to_string()),
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
        if let Some(n) = parse_avd_node(&child)? {
            g.children.push(n);
        }
    }
    Ok(g)
}

fn parse_avd_path(el: &roxmltree::Node) -> Result<AvdPath, ConversionError> {
    let path_data = android_attr(el, "pathData")
        .ok_or_else(|| ConversionError::InvalidSvg("<path> missing android:pathData".into()))?
        .to_string();

    Ok(AvdPath {
        name: android_attr(el, "name").map(|s| s.to_string()),
        path_data,
        fill_color: android_attr(el, "fillColor").map(|s| s.to_string()),
        stroke_color: android_attr(el, "strokeColor").map(|s| s.to_string()),
        stroke_width: android_f32(el, "strokeWidth").unwrap_or(0.0),
        fill_type_evenodd: android_attr(el, "fillType")
            .map(|s| s.eq_ignore_ascii_case("evenOdd"))
            .unwrap_or(false),
        alpha: android_f32(el, "fillAlpha").unwrap_or(1.0),
    })
}

fn android_attr<'a>(el: &'a roxmltree::Node, name: &str) -> Option<&'a str> {
    el.attribute((ANDROID_NS, name)).or_else(|| el.attribute(name))
}

fn android_f32(el: &roxmltree::Node, name: &str) -> Option<f32> {
    android_attr(el, name).and_then(|s| s.trim().parse().ok())
}

// ---------------------------------------------------------------------
// Step 2: timeline
// ---------------------------------------------------------------------

fn compute_timeline(doc: &AvdDocument) -> u32 {
    doc.targets
        .iter()
        .flat_map(|t| t.animators.iter())
        .map(|a| a.start_offset_ms + a.duration_ms)
        .max()
        .unwrap_or(0)
        .max(1) // at least 1ms so sampling always yields >=1 frame
}

// ---------------------------------------------------------------------
// Step 3: sample frame times
// ---------------------------------------------------------------------

fn sample_frame_times(total_duration_ms: u32, fps: u32, max_frames: u32) -> Vec<u32> {
    let natural = ((total_duration_ms as u64 * fps as u64) / 1000).max(1);
    let count = natural.min(max_frames as u64).max(1) as u32;

    let mut times = Vec::with_capacity(count as usize);
    if count == 1 {
        times.push(0);
        return times;
    }
    for i in 0..count {
        // Evenly spaced samples across [0, total_duration_ms], inclusive of
        // both ends, so the first/last frame reflect the true start/end
        // animator values (important for "first frame ~= start value, last
        // frame ~= end value" expectations).
        let t = (total_duration_ms as u64 * i as u64) / (count as u64 - 1);
        times.push(t as u32);
    }
    times
}

// ---------------------------------------------------------------------
// Step 4: evaluate a single frame
// ---------------------------------------------------------------------

fn evaluate_frame(doc: &AvdDocument, time_ms: u32) -> AvdNode {
    let mut clone = doc.base.clone();
    for target in &doc.targets {
        apply_target(&mut clone, target, time_ms);
    }
    clone
}

fn apply_target(node: &mut AvdNode, target: &AnimationTarget, time_ms: u32) {
    let name_matches = match node {
        AvdNode::Group(g) => g.name.as_deref() == Some(target.target_name.as_str()),
        AvdNode::Path(p) => p.name.as_deref() == Some(target.target_name.as_str()),
    };

    if name_matches {
        for animator in &target.animators {
            apply_animator(node, animator, time_ms);
        }
    }

    if let AvdNode::Group(g) = node {
        for child in &mut g.children {
            apply_target(child, target, time_ms);
        }
    }
}

fn progress(animator: &PropertyAnimator, time_ms: u32) -> f32 {
    if animator.duration_ms == 0 {
        return if time_ms >= animator.start_offset_ms { 1.0 } else { 0.0 };
    }
    let elapsed = time_ms as i64 - animator.start_offset_ms as i64;
    let raw = (elapsed as f32 / animator.duration_ms as f32).clamp(0.0, 1.0);
    animator.interpolator.apply(raw)
}

fn apply_animator(node: &mut AvdNode, animator: &PropertyAnimator, time_ms: u32) {
    let t = progress(animator, time_ms);

    match (node, animator.property) {
        (AvdNode::Group(g), Property::Rotation) => {
            if let (AnimValue::Number(a), AnimValue::Number(b)) = (&animator.from, &animator.to) {
                g.rotation = lerp(*a, *b, t);
            }
        }
        (AvdNode::Group(g), Property::TranslateX) => {
            if let (AnimValue::Number(a), AnimValue::Number(b)) = (&animator.from, &animator.to) {
                g.translate_x = lerp(*a, *b, t);
            }
        }
        (AvdNode::Group(g), Property::TranslateY) => {
            if let (AnimValue::Number(a), AnimValue::Number(b)) = (&animator.from, &animator.to) {
                g.translate_y = lerp(*a, *b, t);
            }
        }
        (AvdNode::Group(g), Property::ScaleX) => {
            if let (AnimValue::Number(a), AnimValue::Number(b)) = (&animator.from, &animator.to) {
                g.scale_x = lerp(*a, *b, t);
            }
        }
        (AvdNode::Group(g), Property::ScaleY) => {
            if let (AnimValue::Number(a), AnimValue::Number(b)) = (&animator.from, &animator.to) {
                g.scale_y = lerp(*a, *b, t);
            }
        }
        (AvdNode::Path(p), Property::Alpha) => {
            if let (AnimValue::Number(a), AnimValue::Number(b)) = (&animator.from, &animator.to) {
                p.alpha = lerp(*a, *b, t);
            }
        }
        (AvdNode::Path(p), Property::FillColor) => {
            if let (AnimValue::Color(a), AnimValue::Color(b)) = (&animator.from, &animator.to) {
                p.fill_color = Some(lerp_color(a, b, t));
            }
        }
        (AvdNode::Path(p), Property::StrokeColor) => {
            if let (AnimValue::Color(a), AnimValue::Color(b)) = (&animator.from, &animator.to) {
                p.stroke_color = Some(lerp_color(a, b, t));
            }
        }
        (AvdNode::Path(p), Property::PathData) => {
            if let (AnimValue::Path(a), AnimValue::Path(b)) = (&animator.from, &animator.to) {
                p.path_data = morph_path(a, b, t);
            }
        }
        _ => {} // property doesn't apply to this node kind: ignore, don't error
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// #AARRGGBB per-channel linear lerp. Falls back to `b` verbatim (the "to"
/// color) if either string isn't a well-formed hex color — this is
/// evaluation-time input from an animator value, not a user-controlled
/// parse failure that should abort the whole render.
fn lerp_color(a: &str, b: &str, t: f32) -> String {
    let (pa, pb) = match (parse_argb(a), parse_argb(b)) {
        (Some(x), Some(y)) => (x, y),
        _ => return b.to_string(),
    };
    let mut out = [0u8; 4];
    for i in 0..4 {
        out[i] = (pa[i] as f32 + (pb[i] as f32 - pa[i] as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    format!("#{:02X}{:02X}{:02X}{:02X}", out[0], out[1], out[2], out[3])
}

fn parse_argb(s: &str) -> Option<[u8; 4]> {
    let hex = s.trim_start_matches('#');
    match hex.len() {
        8 => {
            let a = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let r = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let g = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let b = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some([a, r, g, b])
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some([255, r, g, b])
        }
        _ => None,
    }
}

/// Path morphing (Section 3.3): if `from`/`to` share the same command
/// sequence (letters, order, arg counts), per-coordinate lerp; otherwise a
/// hard cut at the midpoint. Never errors — an incompatible pair just
/// degrades to the cut, matching real Android behavior.
fn morph_path(from: &str, to: &str, t: f32) -> String {
    match (tokenize_path(from), tokenize_path(to)) {
        (Some(a), Some(b)) if commands_compatible(&a, &b) => interpolate_commands(&a, &b, t),
        _ => {
            if t < 0.5 {
                from.to_string()
            } else {
                to.to_string()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PathCommand {
    letter: char,
    args: Vec<f32>,
}

/// Minimal SVG/VD path-data tokenizer: splits into command letter + numeric
/// args. Sufficient for structural comparison and coordinate lerp; not a
/// full path grammar validator (that's svg_parser/shapes' job elsewhere).
fn tokenize_path(d: &str) -> Option<Vec<PathCommand>> {
    let mut commands = Vec::new();
    let mut chars = d.chars().peekable();
    let mut current_letter: Option<char> = None;
    let mut current_args: Vec<f32> = Vec::new();
    let mut num_buf = String::new();

    fn flush_num(buf: &mut String, args: &mut Vec<f32>) -> bool {
        if buf.is_empty() {
            return true;
        }
        match buf.parse::<f32>() {
            Ok(n) => {
                args.push(n);
                buf.clear();
                true
            }
            Err(_) => false,
        }
    }

    while let Some(&c) = chars.peek() {
        if c.is_ascii_alphabetic() {
            if !flush_num(&mut num_buf, &mut current_args) {
                return None;
            }
            if let Some(letter) = current_letter {
                commands.push(PathCommand { letter, args: std::mem::take(&mut current_args) });
            }
            current_letter = Some(c);
            chars.next();
        } else if c == ',' || c.is_whitespace() {
            if !flush_num(&mut num_buf, &mut current_args) {
                return None;
            }
            chars.next();
        } else if c == '-' && !num_buf.is_empty() {
            // New number starts (negative), flush current first.
            if !flush_num(&mut num_buf, &mut current_args) {
                return None;
            }
            num_buf.push(c);
            chars.next();
        } else {
            num_buf.push(c);
            chars.next();
        }
    }
    if !flush_num(&mut num_buf, &mut current_args) {
        return None;
    }
    if let Some(letter) = current_letter {
        commands.push(PathCommand { letter, args: current_args });
    }

    if commands.is_empty() {
        None
    } else {
        Some(commands)
    }
}

fn commands_compatible(a: &[PathCommand], b: &[PathCommand]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| x.letter == y.letter && x.args.len() == y.args.len())
}

fn interpolate_commands(a: &[PathCommand], b: &[PathCommand], t: f32) -> String {
    let mut out = String::new();
    for (ca, cb) in a.iter().zip(b.iter()) {
        out.push(ca.letter);
        for (va, vb) in ca.args.iter().zip(cb.args.iter()) {
            out.push_str(&format!("{} ", lerp(*va, *vb, t)));
        }
    }
    out.trim_end().to_string()
}

// ---------------------------------------------------------------------
// Step 5: render a frame -> PNG (via existing pipeline, no new rasterizer)
// ---------------------------------------------------------------------

fn render_frame(node: &AvdNode, doc: &AvdDocument, px: u32) -> Result<Vec<u8>, ConversionError> {
    let normalized = to_normalized_svg(node, doc);
    let xml = crate::vector_drawable::emit(&normalized);
    crate::image_export::render_vd_preview(&xml, px)
}

/// Convert the local AvdNode tree (post-evaluation) into the existing
/// NormalizedSvg shape so the existing emit+rasterize pipeline can be
/// reused unchanged. Uses the AVD's own declared width/height/viewport
/// (parsed in parse_avd_bundle) rather than a hardcoded size, so
/// non-24x24 icons render with the correct coordinate space.
fn to_normalized_svg(node: &AvdNode, doc: &AvdDocument) -> NormalizedSvg {
    let nodes = match node {
        AvdNode::Group(g) => g.children.iter().map(to_vd_node).collect(),
        AvdNode::Path(p) => vec![to_vd_node(&AvdNode::Path(p.clone()))],
    };
    NormalizedSvg {
        width: doc.width,
        height: doc.height,
        viewport_w: doc.viewport_w,
        viewport_h: doc.viewport_h,
        root_alpha: 1.0,
        nodes,
    }
}

fn to_vd_node(node: &AvdNode) -> VdNode {
    match node {
        AvdNode::Path(p) => VdNode::Path(VdPath {
            path_data: p.path_data.clone(),
            fill: match &p.fill_color {
                Some(c) => Fill::Solid(apply_alpha(c, p.alpha)),
                None => Fill::None,
            },
            stroke_color: p.stroke_color.clone(),
            stroke_width: p.stroke_width,
            fill_type: if p.fill_type_evenodd { FillType::EvenOdd } else { FillType::NonZero },
        }),
        AvdNode::Group(g) => VdNode::Group(VdGroup {
            translate_x: g.translate_x,
            translate_y: g.translate_y,
            scale_x: g.scale_x,
            scale_y: g.scale_y,
            rotation: g.rotation,
            pivot_x: g.pivot_x,
            pivot_y: g.pivot_y,
            clip_path: g.clip_path.clone(),
            children: g.children.iter().map(to_vd_node).collect(),
        }),
    }
}

/// Bake a path's `alpha` animator value into its fill color's alpha channel,
/// since `VdPath`/`Fill::Solid` has no separate alpha field to carry it.
fn apply_alpha(color: &str, alpha: f32) -> String {
    let argb = match parse_argb(color) {
        Some(c) => c,
        None => return color.to_string(),
    };
    let a = (argb[0] as f32 * alpha.clamp(0.0, 1.0)).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}{:02X}", a, argb[1], argb[2], argb[3])
}

fn default_loop_mode() -> LoopMode {
    LoopMode::Repeat
}
