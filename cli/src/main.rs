// Watermelon Vector Converter — CLI
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
//
// Scriptable command-line front end over svg-converter-core. Supports the
// same two conversion directions as the GUI (SVG <-> Android VectorDrawable
// XML), for a single file or a ZIP batch, entirely offline.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use clap::{Parser, Subcommand, ValueEnum};
use svg_converter_core::{
    analyze_vd_vector, analyze_vector, batch_processor, convert_svg, convert_vd,
};

#[derive(Parser)]
#[command(
    name = "wvgc-cli",
    version,
    about = "Watermelon Vector Converter — command-line conversion tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Direction {
    /// SVG -> Android VectorDrawable XML
    Vd,
    /// Android VectorDrawable XML -> SVG
    Svg,
}

#[derive(Subcommand)]
enum Command {
    /// Convert a single .svg or VectorDrawable .xml file.
    Convert {
        input: PathBuf,
        /// Output file path. Defaults to the input path with the target extension.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Conversion direction. Inferred from the input extension when omitted.
        #[arg(short, long, value_enum)]
        to: Option<Direction>,
    },
    /// Convert every matching entry inside a ZIP archive.
    Batch {
        input: PathBuf,
        /// Output ZIP path. Defaults to the input path with "_converted" appended.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Conversion direction. Inferred from the archive's contents when omitted.
        #[arg(short, long, value_enum)]
        to: Option<Direction>,
    },
    /// Print the structural analysis of a single file as JSON.
    Analyze { input: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Convert { input, output, to } => convert_file(&input, output, to),
        Command::Batch { input, output, to } => convert_batch(&input, output, to),
        Command::Analyze { input } => analyze_file(&input),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn infer_direction(input: &Path) -> Result<Direction, String> {
    match input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ext) if ext == "svg" => Ok(Direction::Vd),
        Some(ext) if ext == "xml" => Ok(Direction::Svg),
        _ => Err(format!(
            "cannot infer conversion direction from '{}' — pass --to",
            input.display()
        )),
    }
}

fn convert_file(
    input: &Path,
    output: Option<PathBuf>,
    to: Option<Direction>,
) -> Result<(), String> {
    let direction = match to {
        Some(direction) => direction,
        None => infer_direction(input)?,
    };
    let bytes = std::fs::read(input).map_err(|e| format!("reading '{}': {e}", input.display()))?;
    let (converted, default_ext) = match direction {
        Direction::Vd => (convert_svg(&bytes).map_err(|e| e.to_string())?, "xml"),
        Direction::Svg => (convert_vd(&bytes).map_err(|e| e.to_string())?, "svg"),
    };
    let output = output.unwrap_or_else(|| input.with_extension(default_ext));
    std::fs::write(&output, converted)
        .map_err(|e| format!("writing '{}': {e}", output.display()))?;
    println!("{} -> {}", input.display(), output.display());
    Ok(())
}

fn infer_batch_direction(zip_bytes: &[u8]) -> Result<Direction, String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|e| format!("reading zip: {e}"))?;
    let mut svg_count = 0usize;
    let mut xml_count = 0usize;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| format!("reading zip entry: {e}"))?;
        let name = entry.name().to_ascii_lowercase();
        if name.ends_with(".svg") {
            svg_count += 1;
        } else if name.ends_with(".xml") {
            xml_count += 1;
        }
    }
    if svg_count == 0 && xml_count == 0 {
        return Err("archive contains no .svg or .xml entries — pass --to".into());
    }
    Ok(if svg_count >= xml_count {
        Direction::Vd
    } else {
        Direction::Svg
    })
}

fn convert_batch(
    input: &Path,
    output: Option<PathBuf>,
    to: Option<Direction>,
) -> Result<(), String> {
    let mut bytes = Vec::new();
    std::fs::File::open(input)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| format!("reading '{}': {e}", input.display()))?;

    let direction = match to {
        Some(direction) => direction,
        None => infer_batch_direction(&bytes)?,
    };

    let cancel = AtomicBool::new(false);
    let progress = |event: batch_processor::ProgressEvent| {
        eprintln!("[{}/{}] {}", event.done, event.total, event.current_name);
    };
    let converted = match direction {
        Direction::Vd => batch_processor::convert_zip(&bytes, &progress, &cancel),
        Direction::Svg => batch_processor::convert_vd_zip(&bytes, &progress, &cancel),
    }
    .map_err(|e| e.to_string())?;

    let output = output.unwrap_or_else(|| {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        input.with_file_name(format!("{stem}_converted.zip"))
    });
    std::fs::write(&output, converted)
        .map_err(|e| format!("writing '{}': {e}", output.display()))?;
    println!("{} -> {}", input.display(), output.display());
    Ok(())
}

fn analyze_file(input: &Path) -> Result<(), String> {
    let bytes = std::fs::read(input).map_err(|e| format!("reading '{}': {e}", input.display()))?;
    let direction = infer_direction(input)?;
    let analysis = match direction {
        Direction::Vd => analyze_vector(&bytes),
        Direction::Svg => analyze_vd_vector(&bytes),
    }
    .map_err(|e| e.to_string())?;
    println!("{analysis:#?}");
    Ok(())
}
