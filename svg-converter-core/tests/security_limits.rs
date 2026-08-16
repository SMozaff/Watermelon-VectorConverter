use std::io::{Cursor, Write};
use std::sync::atomic::AtomicBool;

use svg_converter_core::batch_processor::convert_zip;
use svg_converter_core::convert_svg;
use svg_converter_core::limits::MAX_SINGLE_INPUT_BYTES;

fn zip_with_entry(name: &str, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        writer.start_file(name, options).unwrap();
        writer.write_all(body).unwrap();
        writer.finish().unwrap();
    }
    out
}

#[test]
fn oversized_svg_is_rejected_before_xml_parsing() {
    let input = vec![b' '; MAX_SINGLE_INPUT_BYTES + 1];
    let error = convert_svg(&input).expect_err("oversized SVG must be rejected");
    assert_eq!(error.code(), 1008);
}

#[test]
fn dtd_svg_is_rejected_by_hardened_xml_options() {
    let input = br#"<!DOCTYPE svg [<!ENTITY harmless "x">]><svg>&harmless;</svg>"#;
    let error = convert_svg(input).expect_err("DTD input must be rejected");
    assert_eq!(error.code(), 1001);
}

#[test]
fn traversal_archive_entry_is_rejected() {
    let zip = zip_with_entry("../escape.svg", b"<svg viewBox=\"0 0 1 1\"/>");
    let error = convert_zip(&zip, &|_| {}, &AtomicBool::new(false))
        .expect_err("path traversal must be rejected");
    assert_eq!(error.code(), 1008);
}

#[test]
fn suspicious_compression_ratio_is_rejected() {
    let body = vec![b'a'; 512 * 1024];
    let zip = zip_with_entry("dense.svg", &body);
    let error = convert_zip(&zip, &|_| {}, &AtomicBool::new(false))
        .expect_err("high-ratio archive entry must be rejected");
    assert_eq!(error.code(), 1008);
}
