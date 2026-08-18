// Module P contract tests (C-2). Copyright (c) 2026 Suhail Muzaffari.

use svg_converter_core::batch_processor::{convert_zip, CancelFlag, ProgressEvent};
use svg_converter_core::image_export::{render_svg_preview, render_vd_preview};
use svg_converter_core::convert_svg;
use std::io::{Cursor, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

fn make_zip(files: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (name, body) in files {
            zw.start_file(*name, opts).unwrap();
            zw.write_all(body.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
    }
    out
}

fn names_in_zip(bytes: &[u8]) -> Vec<String> {
    let mut a = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..a.len()).map(|i| a.by_index(i).unwrap().name().to_string()).collect()
}

const SVG: &str = r##"<svg viewBox="0 0 24 24"><path d="M2 2 L22 2 L22 22 Z" fill="#ff0000"/></svg>"##;

#[test]
fn batch_converts_all_svgs() {
    let zip = make_zip(&[("a.svg", SVG), ("b.svg", SVG), ("notes.txt", "ignore me")]);
    let cancel = AtomicBool::new(false);
    let out = convert_zip(&zip, &|_e: ProgressEvent| {}, &cancel).unwrap();
    let names = names_in_zip(&out);
    assert!(names.contains(&"a.xml".to_string()), "{names:?}");
    assert!(names.contains(&"b.xml".to_string()), "{names:?}");
    // non-svg ignored
    assert_eq!(names.iter().filter(|n| n.ends_with(".xml")).count(), 2);
}

#[test]
fn progress_is_monotonic_and_complete() {
    let zip = make_zip(&[("a.svg", SVG), ("b.svg", SVG), ("c.svg", SVG)]);
    let cancel = AtomicBool::new(false);
    let last = AtomicU32::new(0);
    let total_seen = AtomicU32::new(0);
    let count = AtomicU32::new(0);
    convert_zip(&zip, &|e: ProgressEvent| {
        // done never decreases
        let prev = last.swap(e.done, Ordering::Relaxed);
        assert!(e.done >= prev, "progress went backwards");
        assert_eq!(e.total, 3);
        total_seen.store(e.total, Ordering::Relaxed);
        count.fetch_add(1, Ordering::Relaxed);
    }, &cancel).unwrap();
    assert_eq!(count.load(Ordering::Relaxed), 3, "one event per file");
    assert_eq!(last.load(Ordering::Relaxed), 3, "final done == total");
}

#[test]
fn cancellation_returns_cancelled() {
    let many: Vec<(String, String)> = (0..200)
        .map(|i| (format!("f{i}.svg"), SVG.to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = many.iter().map(|(n, b)| (n.as_str(), b.as_str())).collect();
    let zip = make_zip(&refs);
    let cancel = AtomicBool::new(true); // pre-cancelled
    let err = convert_zip(&zip, &|_e| {}, &cancel).unwrap_err();
    assert_eq!(err.code(), 1007);
}

#[test]
fn per_file_error_becomes_sidecar_not_fatal() {
    let zip = make_zip(&[("good.svg", SVG), ("bad.svg", "<svg><text>x</text></svg>")]);
    let cancel = AtomicBool::new(false);
    let out = convert_zip(&zip, &|_e| {}, &cancel).unwrap();
    let names = names_in_zip(&out);
    assert!(names.contains(&"good.xml".to_string()), "{names:?}");
    assert!(names.contains(&"bad.error.txt".to_string()), "{names:?}");
}

#[test]
fn empty_zip_errors() {
    let zip = make_zip(&[("readme.txt", "no svgs here")]);
    let cancel = AtomicBool::new(false);
    assert_eq!(convert_zip(&zip, &|_e| {}, &cancel).unwrap_err().code(), 1003);
}

#[test]
fn preview_svg_produces_png() {
    let png = render_svg_preview(SVG.as_bytes(), 64).unwrap();
    // PNG magic number
    assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
}

#[test]
fn preview_vd_produces_png() {
    let vd = convert_svg(SVG.as_bytes()).unwrap();
    let png = render_vd_preview(&vd, 64).unwrap();
    assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
}

#[test]
fn preview_px_out_of_range_errors() {
    assert_eq!(render_svg_preview(SVG.as_bytes(), 8).unwrap_err().code(), 1006);
    assert_eq!(render_svg_preview(SVG.as_bytes(), 4096).unwrap_err().code(), 1006);
}

#[test]
fn large_batch_completes() {
    let many: Vec<(String, String)> = (0..500)
        .map(|i| (format!("f{i}.svg"), SVG.to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = many.iter().map(|(n, b)| (n.as_str(), b.as_str())).collect();
    let zip = make_zip(&refs);
    let cancel = AtomicBool::new(false);
    let seen = Mutex::new(0u32);
    let out = convert_zip(&zip, &|_e| { *seen.lock().unwrap() += 1; }, &cancel).unwrap();
    assert_eq!(names_in_zip(&out).iter().filter(|n| n.ends_with(".xml")).count(), 500);
    assert_eq!(*seen.lock().unwrap(), 500);
}
