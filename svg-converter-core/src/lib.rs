// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Public API surface.
//!   R — Conversion Engine : IMPLEMENTED + tested (C-1, C-4)
//!   P — Preview & Batch    : IMPLEMENTED + tested (C-2)
//!   B — FFI Bridge         : IMPLEMENTED (jni.rs write+typecheck-only; Tauri tested; parity verified)
//!   M — Android App        : IMPLEMENTED full app (write+typecheck-only; builds via android.yml CI)
//!   C-5 — Animation Preview Engine : DEFERRED, premium, post-MVP.
//!     detect_animation (C-5.1) is real and complete — see animation.rs.
//!     render_avd_frames (C-5.2) is scaffolding only, pending the Phase 0
//!     test corpus — see animation_engine.rs.

pub mod error;
pub mod models;
pub mod svg_parser;
pub mod shapes;
pub mod arc;
pub mod gradients;
pub mod vector_drawable;
pub mod utils;
pub mod image_export;
pub mod batch_processor;
pub mod analysis;
pub mod vd_models;
pub mod vd_parser;
pub mod svg_emit;
pub mod animation;
pub mod animation_engine;

// FFI bridge (Module B). jni.rs gates its whole contents behind
// #![cfg(target_os = "android")] and the `jni` crate is an Android-only
// dependency, so the module is only declared for Android targets. Without
// this declaration the JNI exports are NOT compiled into the .so, which
// produces a runtime UnsatisfiedLinkError ("No implementation found for
// nativeConvertSvg") even though the library itself loads fine.
#[cfg(target_os = "android")]
pub mod jni;

pub use error::ConversionError;

/// C-1: Convert raw SVG bytes into Android VectorDrawable XML.
pub fn convert_svg(svg_bytes: &[u8]) -> Result<String, ConversionError> {
    let normalized = svg_parser::parse(svg_bytes)?;
    Ok(vector_drawable::emit(&normalized))
}

/// C-4: Convert Android VectorDrawable XML back into SVG.
pub fn convert_vd(vd_xml: &[u8]) -> Result<String, ConversionError> {
    let doc = vd_parser::parse(vd_xml)?;
    Ok(svg_emit::emit(&doc))
}

/// Analyze a vector file's structure for the properties panel.
pub fn analyze_vector(bytes: &[u8]) -> Result<analysis::VectorAnalysis, ConversionError> {
    analysis::analyze(bytes)
}

/// Analyze a VectorDrawable XML file's structure (reverse direction).
pub fn analyze_vd_vector(bytes: &[u8]) -> Result<analysis::VectorAnalysis, ConversionError> {
    analysis::analyze_vd(bytes)
}
/// C-5.1: detect whether a file is animated, and how. Real, complete,
/// frozen — see animation.rs.
pub fn detect_animation(
    file_bytes: &[u8],
    file_kind: animation::FileKind,
) -> animation::AnimationKind {
    animation::detect_animation(file_bytes, file_kind)
}

/// C-5.2: render an AVD's frames. See animation_engine.rs.
pub fn render_avd_frames(
    avd_bytes: &[u8],
    fps: u32,
    max_frames: u32,
    px: u32,
) -> Result<animation_engine::AnimationFrames, ConversionError> {
    animation_engine::render_avd_frames(avd_bytes, fps, max_frames, px)
}
