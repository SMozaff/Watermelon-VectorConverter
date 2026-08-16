// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! ZIP batch conversion (Contract C-2).
//! Rayon parallelizes bounded per-file conversion; a single coordinator thread
//! aggregates progress and invokes the sink. Workers never call the sink.

use crate::convert_svg;
use crate::convert_vd;
use crate::error::ConversionError;
use crate::limits;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

pub struct ProgressEvent {
    pub done: u32,
    pub total: u32,
    pub current_name: String,
}

pub type CancelFlag = AtomicBool;

struct FileOutcome {
    name: String,
    result: Result<String, ConversionError>,
}

fn collect_archive_inputs(
    zip_bytes: &[u8],
    extension: &str,
    output_extension: &str,
) -> Result<Vec<(String, Vec<u8>)>, ConversionError> {
    limits::ensure_zip_input_size(zip_bytes)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|error| ConversionError::ZipReadError(error.to_string()))?;
    if archive.len() > limits::MAX_ZIP_ENTRIES {
        return Err(ConversionError::InputLimit(format!(
            "ZIP contains more than the {}-entry limit",
            limits::MAX_ZIP_ENTRIES
        )));
    }

    let mut total_uncompressed = 0u64;
    let mut output_names = BTreeSet::new();
    let mut inputs = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| ConversionError::ZipReadError(error.to_string()))?;
        let name = entry.name().to_owned();
        if !name.to_ascii_lowercase().ends_with(extension) {
            continue;
        }
        limits::validate_archive_entry(&name, entry.size(), entry.compressed_size())?;
        total_uncompressed = total_uncompressed
            .checked_add(entry.size())
            .ok_or_else(|| ConversionError::InputLimit("ZIP uncompressed size overflow".into()))?;
        if total_uncompressed > limits::MAX_ZIP_UNCOMPRESSED_BYTES {
            return Err(ConversionError::InputLimit(format!(
                "ZIP selected entries exceed the {} MiB aggregate limit",
                limits::MAX_ZIP_UNCOMPRESSED_BYTES / 1024 / 1024
            )));
        }

        let output_name = swap_ext(&name, output_extension);
        if !output_names.insert(output_name) {
            return Err(ConversionError::InputLimit(
                "ZIP entries would create duplicate output names".into(),
            ));
        }

        let capacity = usize::try_from(entry.size()).map_err(|_| {
            ConversionError::InputLimit("archive entry is too large for this platform".into())
        })?;
        let mut bytes = Vec::with_capacity(capacity);
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| ConversionError::ZipReadError(error.to_string()))?;
        if bytes.len() as u64 != entry.size() {
            return Err(ConversionError::ZipReadError(
                "archive entry size changed while reading".into(),
            ));
        }
        inputs.push((name, bytes));
    }
    Ok(inputs)
}

fn convert_archive(
    zip_bytes: &[u8],
    input_extension: &str,
    output_extension: &str,
    convert: fn(&[u8]) -> Result<String, ConversionError>,
    progress: &(dyn Fn(ProgressEvent) + Send + Sync),
    cancel: &CancelFlag,
) -> Result<Vec<u8>, ConversionError> {
    let inputs = collect_archive_inputs(zip_bytes, input_extension, output_extension)?;
    let total = u32::try_from(inputs.len())
        .map_err(|_| ConversionError::InputLimit("too many ZIP inputs".into()))?;
    if total == 0 {
        return Err(ConversionError::ZipReadError(format!(
            "no {input_extension} entries"
        )));
    }

    let (tx, rx) = mpsc::channel::<FileOutcome>();
    let cancelled_during = AtomicBool::new(false);
    let mut outcomes = Vec::with_capacity(inputs.len());
    std::thread::scope(|scope| {
        let inputs_ref = &inputs;
        let cancel_ref = cancel;
        let cancelled_ref = &cancelled_during;
        scope.spawn(move || {
            inputs_ref
                .par_iter()
                .for_each_with(tx, |sender, (name, bytes)| {
                    if cancel_ref.load(Ordering::Relaxed) {
                        cancelled_ref.store(true, Ordering::Relaxed);
                        return;
                    }
                    let _ = sender.send(FileOutcome {
                        name: name.clone(),
                        result: convert(bytes),
                    });
                });
        });

        for (index, outcome) in rx.iter().enumerate() {
            progress(ProgressEvent {
                done: u32::try_from(index + 1).expect("ZIP entry count is bounded"),
                total,
                current_name: outcome.name.clone(),
            });
            outcomes.push(outcome);
        }
    });

    if cancel.load(Ordering::Relaxed) || cancelled_during.load(Ordering::Relaxed) {
        return Err(ConversionError::Cancelled);
    }
    build_output_archive(&outcomes, output_extension)
}

fn build_output_archive(
    outcomes: &[FileOutcome],
    output_extension: &str,
) -> Result<Vec<u8>, ConversionError> {
    let mut out = Vec::new();
    let mut output_payload_bytes = 0usize;
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for outcome in outcomes {
            let (name, body) = match &outcome.result {
                Ok(converted) => (
                    swap_ext(&outcome.name, output_extension),
                    converted.as_bytes().to_vec(),
                ),
                Err(error) => (
                    swap_ext(&outcome.name, "error.txt"),
                    format!("[{}] {error}", error.code()).into_bytes(),
                ),
            };
            output_payload_bytes =
                output_payload_bytes
                    .checked_add(body.len())
                    .ok_or_else(|| {
                        ConversionError::InputLimit("output archive size overflow".into())
                    })?;
            if output_payload_bytes > limits::MAX_ZIP_OUTPUT_BYTES {
                return Err(ConversionError::InputLimit(format!(
                    "output archive exceeds the {} MiB limit",
                    limits::MAX_ZIP_OUTPUT_BYTES / 1024 / 1024
                )));
            }
            writer
                .start_file(name, options)
                .map_err(|error| ConversionError::ZipWriteError(error.to_string()))?;
            writer
                .write_all(&body)
                .map_err(|error| ConversionError::ZipWriteError(error.to_string()))?;
        }
        writer
            .finish()
            .map_err(|error| ConversionError::ZipWriteError(error.to_string()))?;
    }
    if out.len() > limits::MAX_ZIP_OUTPUT_BYTES {
        return Err(ConversionError::InputLimit(format!(
            "output archive exceeds the {} MiB limit",
            limits::MAX_ZIP_OUTPUT_BYTES / 1024 / 1024
        )));
    }
    Ok(out)
}

/// Convert every `.svg` entry in a ZIP to a VectorDrawable `.xml` entry.
pub fn convert_zip(
    zip_bytes: &[u8],
    progress: &(dyn Fn(ProgressEvent) + Send + Sync),
    cancel: &CancelFlag,
) -> Result<Vec<u8>, ConversionError> {
    convert_archive(zip_bytes, ".svg", "xml", convert_svg, progress, cancel)
}

fn swap_ext(name: &str, new_ext: &str) -> String {
    match name.rfind('.') {
        Some(index) => format!("{}.{}", &name[..index], new_ext),
        None => format!("{name}.{new_ext}"),
    }
}

/// Package several bounded loose files into a safe in-memory ZIP archive.
pub fn zip_files_into_archive(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, ConversionError> {
    if files.len() > limits::MAX_ZIP_ENTRIES {
        return Err(ConversionError::InputLimit(format!(
            "batch contains more than the {}-file limit",
            limits::MAX_ZIP_ENTRIES
        )));
    }
    let mut aggregate = 0u64;
    let mut names = BTreeSet::new();
    let mut out = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in files {
            limits::validate_archive_path(name)?;
            if bytes.len() > limits::MAX_ZIP_ENTRY_BYTES as usize {
                return Err(ConversionError::InputLimit(
                    "loose batch file exceeds the per-file limit".into(),
                ));
            }
            aggregate = aggregate
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| ConversionError::InputLimit("loose batch size overflow".into()))?;
            if aggregate > limits::MAX_ZIP_UNCOMPRESSED_BYTES {
                return Err(ConversionError::InputLimit(
                    "loose batch exceeds the aggregate size limit".into(),
                ));
            }
            if !names.insert(name) {
                return Err(ConversionError::InputLimit(
                    "loose batch contains duplicate file names".into(),
                ));
            }
            writer
                .start_file(name, options)
                .map_err(|error| ConversionError::ZipWriteError(error.to_string()))?;
            writer
                .write_all(bytes)
                .map_err(|error| ConversionError::ZipWriteError(error.to_string()))?;
        }
        writer
            .finish()
            .map_err(|error| ConversionError::ZipWriteError(error.to_string()))?;
    }
    if out.len() > limits::MAX_ZIP_OUTPUT_BYTES {
        return Err(ConversionError::InputLimit(
            "loose batch archive exceeds output limit".into(),
        ));
    }
    Ok(out)
}

/// Convert every `.xml` VectorDrawable entry in a ZIP to an SVG entry.
pub fn convert_vd_zip(
    zip_bytes: &[u8],
    progress: &(dyn Fn(ProgressEvent) + Send + Sync),
    cancel: &CancelFlag,
) -> Result<Vec<u8>, ConversionError> {
    convert_archive(zip_bytes, ".xml", "svg", convert_vd, progress, cancel)
}
