// Reverse batch conversion tests (Contract C-4 batch). Copyright (c) 2026 Suhail Muzaffari.

use svg_converter_core::batch_processor::{convert_vd_zip, ProgressEvent};
use std::io::{Cursor, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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

const VD: &str = r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:viewportWidth="24" android:viewportHeight="24">
    <path android:pathData="M2,2 L22,2 L22,22 Z" android:fillColor="#FFFF0000"/>
</vector>"##;

#[test]
fn batch_converts_all_xmls() {
    let zip = make_zip(&[("a.xml", VD), ("b.xml", VD), ("notes.txt", "ignore me")]);
    let cancel = AtomicBool::new(false);
    let out = convert_vd_zip(&zip, &|_e: ProgressEvent| {}, &cancel).unwrap();
    let names = names_in_zip(&out);
    assert!(names.contains(&"a.svg".to_string()), "{names:?}");
    assert!(names.contains(&"b.svg".to_string()), "{names:?}");
    assert_eq!(names.iter().filter(|n| n.ends_with(".svg")).count(), 2);
}

#[test]
fn per_file_error_becomes_sidecar_not_fatal() {
    let zip = make_zip(&[("good.xml", VD), ("bad.xml", "<not-a-vector/>")]);
    let cancel = AtomicBool::new(false);
    let out = convert_vd_zip(&zip, &|_e| {}, &cancel).unwrap();
    let names = names_in_zip(&out);
    assert!(names.contains(&"good.svg".to_string()), "{names:?}");
    assert!(names.contains(&"bad.error.txt".to_string()), "{names:?}");
}

#[test]
fn empty_zip_errors() {
    let zip = make_zip(&[("readme.txt", "no xmls here")]);
    let cancel = AtomicBool::new(false);
    assert_eq!(convert_vd_zip(&zip, &|_e| {}, &cancel).unwrap_err().code(), 1003);
}

#[test]
fn cancellation_returns_cancelled() {
    let many: Vec<(String, String)> = (0..200)
        .map(|i| (format!("f{i}.xml"), VD.to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = many.iter().map(|(n, b)| (n.as_str(), b.as_str())).collect();
    let zip = make_zip(&refs);
    let cancel = AtomicBool::new(true); // pre-cancelled
    let err = convert_vd_zip(&zip, &|_e| {}, &cancel).unwrap_err();
    assert_eq!(err.code(), 1007);
}

#[test]
fn progress_is_monotonic_and_complete() {
    let zip = make_zip(&[("a.xml", VD), ("b.xml", VD), ("c.xml", VD)]);
    let cancel = AtomicBool::new(false);
    let last = AtomicU32::new(0);
    let count = AtomicU32::new(0);
    convert_vd_zip(&zip, &|e: ProgressEvent| {
        let prev = last.swap(e.done, Ordering::Relaxed);
        assert!(e.done >= prev, "progress went backwards");
        assert_eq!(e.total, 3);
        count.fetch_add(1, Ordering::Relaxed);
    }, &cancel).unwrap();
    assert_eq!(count.load(Ordering::Relaxed), 3, "one event per file");
    assert_eq!(last.load(Ordering::Relaxed), 3, "final done == total");
}
