// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Central resource budgets for untrusted SVG, VectorDrawable, and ZIP input.

use crate::error::ConversionError;
use std::path::{Component, Path};

pub const MAX_SINGLE_INPUT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_ZIP_INPUT_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_ZIP_ENTRIES: usize = 1_000;
pub const MAX_ZIP_ENTRY_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_ZIP_UNCOMPRESSED_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_ZIP_OUTPUT_BYTES: usize = 200 * 1024 * 1024;
pub const MAX_COMPRESSION_RATIO: u64 = 100;
pub const MAX_ARCHIVE_PATH_DEPTH: usize = 16;
pub const MAX_XML_NODES: u32 = 100_000;
pub const MAX_RENDER_PIXELS: u64 = 16 * 1024 * 1024;
pub const MAX_ANIMATION_FRAMES: u32 = 180;

pub fn ensure_input_size(bytes: &[u8], context: &str) -> Result<(), ConversionError> {
    if bytes.len() > MAX_SINGLE_INPUT_BYTES {
        return Err(ConversionError::InputLimit(format!(
            "{context} exceeds the {} MiB input limit",
            MAX_SINGLE_INPUT_BYTES / 1024 / 1024
        )));
    }
    Ok(())
}

pub fn ensure_zip_input_size(bytes: &[u8]) -> Result<(), ConversionError> {
    if bytes.len() > MAX_ZIP_INPUT_BYTES {
        return Err(ConversionError::InputLimit(format!(
            "ZIP input exceeds the {} MiB compressed-size limit",
            MAX_ZIP_INPUT_BYTES / 1024 / 1024
        )));
    }
    Ok(())
}

pub fn xml_options() -> roxmltree::ParsingOptions {
    roxmltree::ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_XML_NODES,
    }
}

pub fn validate_archive_entry(
    name: &str,
    uncompressed_size: u64,
    compressed_size: u64,
) -> Result<(), ConversionError> {
    validate_archive_path(name)?;
    if uncompressed_size > MAX_ZIP_ENTRY_BYTES {
        return Err(ConversionError::InputLimit(format!(
            "archive entry '{name}' exceeds the {} MiB per-entry limit",
            MAX_ZIP_ENTRY_BYTES / 1024 / 1024
        )));
    }
    if uncompressed_size > 0 && compressed_size == 0 {
        return Err(ConversionError::InputLimit(format!(
            "archive entry '{name}' has an invalid compressed size"
        )));
    }
    if compressed_size > 0 && uncompressed_size / compressed_size > MAX_COMPRESSION_RATIO {
        return Err(ConversionError::InputLimit(format!(
            "archive entry '{name}' exceeds the {MAX_COMPRESSION_RATIO}:1 compression-ratio limit"
        )));
    }
    Ok(())
}

pub fn validate_archive_path(name: &str) -> Result<(), ConversionError> {
    if name.is_empty() || name.contains('\\') {
        return Err(ConversionError::InputLimit(
            "archive entry has an unsafe path".into(),
        ));
    }
    let path = Path::new(name);
    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::Normal(_) => {
                depth += 1;
                if depth > MAX_ARCHIVE_PATH_DEPTH {
                    return Err(ConversionError::InputLimit(format!(
                        "archive entry path exceeds the {MAX_ARCHIVE_PATH_DEPTH}-component depth limit"
                    )));
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ConversionError::InputLimit(
                    "archive entry has an unsafe path".into(),
                ));
            }
        }
    }
    if depth == 0 || name.ends_with('/') {
        return Err(ConversionError::InputLimit(
            "archive entry is not a regular file path".into(),
        ));
    }
    Ok(())
}

pub fn ensure_render_area(width: u32, height: u32) -> Result<(), ConversionError> {
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_RENDER_PIXELS {
        return Err(ConversionError::InputLimit(format!(
            "render area exceeds the {} megapixel limit",
            MAX_RENDER_PIXELS / (1024 * 1024)
        )));
    }
    Ok(())
}
