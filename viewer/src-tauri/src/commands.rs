// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

use crate::PendingFile;
use serde::Serialize;
use std::fs;
use std::path::Path;
use svg_converter_core::animation::{AnimationKind, FileKind};
use svg_converter_core::detect_animation;
use svg_converter_core::image_export::{render_svg_preview, render_vd_preview};
use svg_converter_core::limits::MAX_SINGLE_INPUT_BYTES;
use svg_converter_core::render_avd_frames;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ViewerErrorDto {
    pub message: String,
}

impl From<std::io::Error> for ViewerErrorDto {
    fn from(error: std::io::Error) -> Self {
        ViewerErrorDto {
            message: error.to_string(),
        }
    }
}

impl From<svg_converter_core::error::ConversionError> for ViewerErrorDto {
    fn from(error: svg_converter_core::error::ConversionError) -> Self {
        ViewerErrorDto {
            message: error.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AvdFramesPayload {
    pub width: u32,
    pub height: u32,
    pub loop_mode: String,
    pub frame_durations_ms: Vec<u32>,
    pub frames: Vec<Vec<u8>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum FilePreviewDto {
    Static { png: Vec<u8> },
    Avd { frames: AvdFramesPayload },
    AnimatedSvg { svg_text: String },
}

/// A preview result has a display name but never exposes the source path to
/// the webview. This keeps user filesystem paths out of the IPC surface.
#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub name: String,
    pub preview: FilePreviewDto,
}

fn bounded_regular_file(path: &Path) -> Result<(String, Vec<u8>), ViewerErrorDto> {
    let metadata = fs::symlink_metadata(path).map_err(ViewerErrorDto::from)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ViewerErrorDto {
            message: "selected path is not a regular file".into(),
        });
    }
    if metadata.len() > MAX_SINGLE_INPUT_BYTES as u64 {
        return Err(ViewerErrorDto {
            message: format!(
                "selected file exceeds the {} MiB input limit",
                MAX_SINGLE_INPUT_BYTES / 1024 / 1024
            ),
        });
    }
    let bytes = fs::read(path).map_err(ViewerErrorDto::from)?;
    if bytes.len() > MAX_SINGLE_INPUT_BYTES {
        return Err(ViewerErrorDto {
            message: "selected file grew beyond the input limit while reading".into(),
        });
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ViewerErrorDto {
            message: "selected file name is not valid UTF-8".into(),
        })?
        .to_owned();
    Ok((name, bytes))
}

fn preview_path(path: &Path, px: u32) -> Result<PreviewResponse, ViewerErrorDto> {
    let (name, bytes) = bounded_regular_file(path)?;
    let content = String::from_utf8(bytes.clone()).map_err(|error| ViewerErrorDto {
        message: format!("selected file is not valid UTF-8: {error}"),
    })?;

    let trimmed = content.trim_start();
    let is_vector_drawable = trimmed
        .lines()
        .find(|line| !line.trim().is_empty() && !line.trim_start().starts_with("<?xml"))
        .map(|line| {
            line.trim_start().starts_with("<vector")
                || line.trim_start().starts_with("<animated-vector")
        })
        .unwrap_or(false);

    let file_kind = if is_vector_drawable {
        FileKind::Avd
    } else {
        FileKind::Svg
    };
    let preview = match detect_animation(&bytes, file_kind) {
        AnimationKind::Avd => {
            let frames = render_avd_frames(&bytes, 30, 90, px).map_err(ViewerErrorDto::from)?;
            let loop_mode = match frames.loop_mode {
                svg_converter_core::animation_engine::LoopMode::Once => "Once",
                svg_converter_core::animation_engine::LoopMode::Repeat => "Repeat",
                svg_converter_core::animation_engine::LoopMode::Reverse => "Reverse",
            };
            FilePreviewDto::Avd {
                frames: AvdFramesPayload {
                    width: frames.width,
                    height: frames.height,
                    loop_mode: loop_mode.to_string(),
                    frame_durations_ms: frames.frame_durations_ms,
                    frames: frames.frames,
                },
            }
        }
        AnimationKind::SvgSmil | AnimationKind::SvgCss => {
            FilePreviewDto::AnimatedSvg { svg_text: content }
        }
        AnimationKind::None => {
            let png = if is_vector_drawable {
                render_vd_preview(&content, px).map_err(ViewerErrorDto::from)?
            } else {
                render_svg_preview(&bytes, px).map_err(ViewerErrorDto::from)?
            };
            FilePreviewDto::Static { png }
        }
    };

    Ok(PreviewResponse { name, preview })
}

/// Show a native dialog and render the selected file without returning a path.
#[tauri::command]
pub fn pick_file_preview(px: u32) -> Result<Option<PreviewResponse>, ViewerErrorDto> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("SVG / VectorDrawable", &["svg", "xml"])
        .pick_file()
    else {
        return Ok(None);
    };
    preview_path(&path, px).map(Some)
}

/// Consume the command-line or single-instance file request and render it in
/// Rust. The path remains inside the application state and never crosses IPC.
#[tauri::command]
pub fn take_pending_file_preview(
    state: State<PendingFile>,
    px: u32,
) -> Result<Option<PreviewResponse>, ViewerErrorDto> {
    let path = state
        .0
        .lock()
        .map_err(|_| ViewerErrorDto {
            message: "pending-file state is unavailable".into(),
        })?
        .take();
    match path {
        Some(path) => preview_path(Path::new(&path), px).map(Some),
        None => Ok(None),
    }
}
