// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! ZIP batch conversion (Contract C-2).
//! Rayon parallelizes per-file conversion; a SINGLE coordinator thread
//! aggregates progress and invokes the sink. Workers never call the sink.
//! Cancellation is a shared atomic flag checked between files.

use crate::convert_svg;
use crate::convert_vd;
use crate::error::ConversionError;
use rayon::prelude::*;
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

/// C-2: Convert every .svg in `zip_bytes` to a VectorDrawable .xml, returning
/// a new ZIP. Progress is reported via `progress` from the coordinator only.
/// On cancellation, returns Err(Cancelled) and discards partial output.
pub fn convert_zip(
    zip_bytes: &[u8],
    progress: &(dyn Fn(ProgressEvent) + Send + Sync),
    cancel: &CancelFlag,
) -> Result<Vec<u8>, ConversionError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| ConversionError::ZipReadError(e.to_string()))?;

    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ConversionError::ZipReadError(e.to_string()))?;
        let name = entry.name().to_string();
        if !name.to_ascii_lowercase().ends_with(".svg") {
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| ConversionError::ZipReadError(e.to_string()))?;
        inputs.push((name, buf));
    }

    let total = inputs.len() as u32;
    if total == 0 {
        return Err(ConversionError::ZipReadError("no .svg entries".into()));
    }

    let (tx, rx) = mpsc::channel::<FileOutcome>();
    let cancelled_during = AtomicBool::new(false);

    // A producer thread drives the Rayon parallel conversion and sends each
    // outcome down the channel. THIS thread is the sole coordinator: it owns
    // the receiver and is the only place progress() is ever called. Workers
    // never touch the sink.
    let mut outcomes: Vec<FileOutcome> = Vec::with_capacity(total as usize);
    std::thread::scope(|s| {
        let inputs_ref = &inputs;
        let cancel_ref = &cancel;
        let cancelled_ref = &cancelled_during;
        s.spawn(move || {
            inputs_ref
                .par_iter()
                .for_each_with(tx, |tx, (name, bytes)| {
                    if cancel_ref.load(Ordering::Relaxed) {
                        cancelled_ref.store(true, Ordering::Relaxed);
                        return;
                    }
                    let result = convert_svg(bytes);
                    let _ = tx.send(FileOutcome {
                        name: name.clone(),
                        result,
                    });
                });
            // tx (and all clones) dropped here -> rx iteration ends
        });

        let mut done = 0u32;
        for outcome in rx.iter() {
            done += 1;
            progress(ProgressEvent {
                done,
                total,
                current_name: outcome.name.clone(),
            });
            outcomes.push(outcome);
        }
    });

    if cancel.load(Ordering::Relaxed) || cancelled_during.load(Ordering::Relaxed) {
        return Err(ConversionError::Cancelled);
    }

    // Build output ZIP. Per-file errors become .error.txt sidecars so a single
    // bad file does not fail the whole batch (graceful degradation, C-2).
    let mut out = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for outcome in &outcomes {
            match &outcome.result {
                Ok(xml) => {
                    zw.start_file(swap_ext(&outcome.name, "xml"), opts)
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                    zw.write_all(xml.as_bytes())
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                }
                Err(e) => {
                    zw.start_file(swap_ext(&outcome.name, "error.txt"), opts)
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                    zw.write_all(format!("[{}] {}", e.code(), e).as_bytes())
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                }
            }
        }
        zw.finish()
            .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
    }
    Ok(out)
}

fn swap_ext(name: &str, new_ext: &str) -> String {
    match name.rfind('.') {
        Some(idx) => format!("{}.{}", &name[..idx], new_ext),
        None => format!("{}.{}", name, new_ext),
    }
}

/// Package several loose files into one ZIP archive in memory. Used when the
/// person selects/drops multiple loose files at once — they're zipped here
/// so the existing convert_zip/convert_vd_zip batch path can handle them
/// uniformly, rather than duplicating the batch pipeline for a "list of
/// files" input shape.
pub fn zip_files_into_archive(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, ConversionError> {
    let mut out = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in files {
            zw.start_file(name, opts)
                .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
            zw.write_all(bytes)
                .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
        }
        zw.finish()
            .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
    }
    Ok(out)
}

/// C-4 batch: Convert every .xml in `zip_bytes` (VectorDrawable) to .svg,
/// returning a new ZIP. Mirrors convert_zip exactly, mapped to the reverse
/// direction — kept as a separate function rather than a flag on convert_zip
/// so the existing, already-shipped C-2 contract is never touched.
pub fn convert_vd_zip(
    zip_bytes: &[u8],
    progress: &(dyn Fn(ProgressEvent) + Send + Sync),
    cancel: &CancelFlag,
) -> Result<Vec<u8>, ConversionError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| ConversionError::ZipReadError(e.to_string()))?;

    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ConversionError::ZipReadError(e.to_string()))?;
        let name = entry.name().to_string();
        if !name.to_ascii_lowercase().ends_with(".xml") {
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| ConversionError::ZipReadError(e.to_string()))?;
        inputs.push((name, buf));
    }

    let total = inputs.len() as u32;
    if total == 0 {
        return Err(ConversionError::ZipReadError("no .xml entries".into()));
    }

    let (tx, rx) = mpsc::channel::<FileOutcome>();
    let cancelled_during = AtomicBool::new(false);

    let mut outcomes: Vec<FileOutcome> = Vec::with_capacity(total as usize);
    std::thread::scope(|s| {
        let inputs_ref = &inputs;
        let cancel_ref = &cancel;
        let cancelled_ref = &cancelled_during;
        s.spawn(move || {
            inputs_ref
                .par_iter()
                .for_each_with(tx, |tx, (name, bytes)| {
                    if cancel_ref.load(Ordering::Relaxed) {
                        cancelled_ref.store(true, Ordering::Relaxed);
                        return;
                    }
                    let result = convert_vd(bytes);
                    let _ = tx.send(FileOutcome {
                        name: name.clone(),
                        result,
                    });
                });
        });

        let mut done = 0u32;
        for outcome in rx.iter() {
            done += 1;
            progress(ProgressEvent {
                done,
                total,
                current_name: outcome.name.clone(),
            });
            outcomes.push(outcome);
        }
    });

    if cancel.load(Ordering::Relaxed) || cancelled_during.load(Ordering::Relaxed) {
        return Err(ConversionError::Cancelled);
    }

    let mut out = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for outcome in &outcomes {
            match &outcome.result {
                Ok(svg) => {
                    zw.start_file(swap_ext(&outcome.name, "svg"), opts)
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                    zw.write_all(svg.as_bytes())
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                }
                Err(e) => {
                    zw.start_file(swap_ext(&outcome.name, "error.txt"), opts)
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                    zw.write_all(format!("[{}] {}", e.code(), e).as_bytes())
                        .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
                }
            }
        }
        zw.finish()
            .map_err(|e| ConversionError::ZipWriteError(e.to_string()))?;
    }
    Ok(out)
}
