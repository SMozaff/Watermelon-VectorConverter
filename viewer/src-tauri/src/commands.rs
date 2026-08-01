// Watermelon Vector Viewer
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

use crate::PendingFile;
use serde::Serialize;
use std::fs;
use svg_converter_core::animation::{AnimationKind, FileKind};
use svg_converter_core::detect_animation;
use svg_converter_core::image_export::{render_svg_preview, render_vd_preview};
use svg_converter_core::render_avd_frames;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ViewerErrorDto {
    pub message: String,
}

impl From<std::io::Error> for ViewerErrorDto {
    fn from(e: std::io::Error) -> Self {
        ViewerErrorDto { message: e.to_string() }
    }
}

impl From<svg_converter_core::error::ConversionError> for ViewerErrorDto {
    fn from(e: svg_converter_core::error::ConversionError) -> Self {
        ViewerErrorDto { message: e.to_string() }
    }
}

/// One frame-set entry, mirroring AnimationFrames. Frame PNGs are plain
/// Vec<u8> (serde_json serializes each as a JSON number array), matching
/// the same convention the main app's AvdFramesDto already uses — no
/// base64 needed on this side of the IPC boundary.
#[derive(Debug, Serialize)]
pub struct AvdFramesPayload {
    pub width: u32,
    pub height: u32,
    pub loop_mode: String,
    pub frame_durations_ms: Vec<u32>,
    pub frames: Vec<Vec<u8>>,
}

/// Tagged result covering all three Contract C-5 outcomes. The frontend
/// branches on `kind` to decide how to render: a plain <img> for Static, a
/// canvas + setTimeout playback loop for Avd, or a sandboxed inline-SVG
/// render for AnimatedSvg.
#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum FilePreviewDto {
    Static { png: Vec<u8> },
    Avd { frames: AvdFramesPayload },
    /// Raw SVG text — Contract C-5.5 explicitly hands this to the existing
    /// webview frontend with no native rendering at all, so we return the
    /// file's own text unchanged rather than any rendered form of it.
    AnimatedSvg { svg_text: String },
}

/// Read a file from disk, detect whether it's a raw SVG or a VectorDrawable
/// XML, detect whether either is animated (Contract C-5.1), and return
/// whichever of the three C-5 preview forms applies.
#[tauri::command]
pub fn render_file_preview(path: String, px: u32) -> Result<FilePreviewDto, ViewerErrorDto> {
    let bytes = fs::read(&path).map_err(ViewerErrorDto::from)?;
    let content = String::from_utf8_lossy(&bytes).into_owned();

    // VectorDrawable XML starts with <vector ...>, plain SVG starts with <svg ...>.
    // Check the root tag rather than the file extension, since a user could
    // rename either type.
    let trimmed = content.trim_start();
    let is_vector_drawable = trimmed
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with("<?xml"))
        .map(|l| l.trim_start().starts_with("<vector") || l.trim_start().starts_with("<animated-vector"))
        .unwrap_or(false);

    let file_kind = if is_vector_drawable { FileKind::Avd } else { FileKind::Svg };
    let anim_kind = detect_animation(&bytes, file_kind);

    match anim_kind {
        AnimationKind::Avd => {
            let frames = render_avd_frames(&bytes, 30, 90, px).map_err(ViewerErrorDto::from)?;
            let loop_mode = match frames.loop_mode {
                svg_converter_core::animation_engine::LoopMode::Once => "Once",
                svg_converter_core::animation_engine::LoopMode::Repeat => "Repeat",
                svg_converter_core::animation_engine::LoopMode::Reverse => "Reverse",
            };
            Ok(FilePreviewDto::Avd {
                frames: AvdFramesPayload {
                    width: frames.width,
                    height: frames.height,
                    loop_mode: loop_mode.to_string(),
                    frame_durations_ms: frames.frame_durations_ms,
                    frames: frames.frames,
                },
            })
        }
        AnimationKind::SvgSmil | AnimationKind::SvgCss => {
            Ok(FilePreviewDto::AnimatedSvg { svg_text: content })
        }
        AnimationKind::None => {
            let png = if is_vector_drawable {
                render_vd_preview(&content, px).map_err(ViewerErrorDto::from)?
            } else {
                render_svg_preview(&bytes, px).map_err(ViewerErrorDto::from)?
            };
            Ok(FilePreviewDto::Static { png })
        }
    }
}

/// Called by the frontend once on startup to retrieve the file path that
/// launched this instance (from CLI args), then clears it so it's only
/// consumed once.
#[tauri::command]
pub fn take_pending_file(state: State<PendingFile>) -> Option<String> {
    state.0.lock().unwrap().take()
}
