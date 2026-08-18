// Watermelon Vector Converter — Module R contract tests (C-1).
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

use svg_converter_core::{convert_svg, ConversionError};

#[test]
fn basic_triangle_matches_contract_vector() {
    let svg = br##"<svg viewBox="0 0 24 24"><path d="M2 2 L22 2 L22 22 Z" fill="#f00"/></svg>"##;
    let out = convert_svg(svg).expect("should convert");
    assert!(out.contains("android:viewportWidth=\"24\""));
    assert!(out.contains("android:viewportHeight=\"24\""));
    assert!(out.contains("android:pathData=\"M2,2 L22,2 L22,22 Z\""));
    assert!(out.contains("android:fillColor=\"#FFFF0000\""));
    assert!(out.contains("android:fillType=\"nonZero\""));
}

#[test]
fn filter_is_still_rejected_1002() {
    // gradients are now supported; filters remain out of scope
    let svg = br##"<svg viewBox="0 0 10 10"><filter id="f"/><path d="M0 0 L1 1"/></svg>"##;
    assert_eq!(convert_svg(svg).unwrap_err().code(), 1002);
}

#[test]
fn text_is_rejected_1002() {
    let svg = br##"<svg viewBox="0 0 10 10"><text>hi</text></svg>"##;
    assert_eq!(convert_svg(svg).unwrap_err().code(), 1002);
}

#[test]
fn malformed_xml_is_invalid_1001() {
    let svg = b"<svg><path d=";
    assert_eq!(convert_svg(svg).unwrap_err().code(), 1001);
}

#[test]
fn non_utf8_is_invalid_1001() {
    let svg = &[0xff, 0xfe, 0x00, 0x01];
    assert_eq!(convert_svg(svg).unwrap_err().code(), 1001);
}

#[test]
fn relative_commands_convert_to_absolute() {
    // m 2 2 l 5 0 -> absolute M2,2 L7,2
    let svg = br##"<svg viewBox="0 0 24 24"><path d="m2 2 l5 0" fill="black"/></svg>"##;
    let out = convert_svg(svg).unwrap();
    assert!(out.contains("M2,2 L7,2"), "got: {out}");
}

#[test]
fn hex_short_form_expands() {
    let svg = br##"<svg viewBox="0 0 1 1"><path d="M0 0 L1 1" fill="#0f0"/></svg>"##;
    let out = convert_svg(svg).unwrap();
    assert!(out.contains("android:fillColor=\"#FF00FF00\""), "got: {out}");
}

#[test]
fn fill_none_omits_fill_color() {
    let svg = br##"<svg viewBox="0 0 1 1"><path d="M0 0 L1 1" fill="none" stroke="black" stroke-width="2"/></svg>"##;
    let out = convert_svg(svg).unwrap();
    assert!(!out.contains("android:fillColor"), "got: {out}");
    assert!(out.contains("android:strokeColor=\"#FF000000\""));
    assert!(out.contains("android:strokeWidth=\"2\""));
}

#[test]
fn evenodd_fill_rule_mapped() {
    let svg = br##"<svg viewBox="0 0 1 1"><path d="M0 0 L1 1 Z" fill-rule="evenodd"/></svg>"##;
    let out = convert_svg(svg).unwrap();
    assert!(out.contains("android:fillType=\"evenOdd\""), "got: {out}");
}

#[test]
fn group_translate_emitted() {
    let svg = br##"<svg viewBox="0 0 24 24"><g transform="translate(10,20)"><path d="M0 0 L1 1"/></g></svg>"##;
    let out = convert_svg(svg).unwrap();
    assert!(out.contains("<group"), "got: {out}");
    assert!(out.contains("android:translateX=\"10\""));
    assert!(out.contains("android:translateY=\"20\""));
}

#[test]
fn determinism_same_input_same_output() {
    let svg = br##"<svg viewBox="0 0 24 24"><path d="M2 2 L22 2 L22 22 Z" fill="#abc"/></svg>"##;
    let a = convert_svg(svg).unwrap();
    let b = convert_svg(svg).unwrap();
    assert_eq!(a, b);
}

#[test]
fn never_panics_on_garbage() {
    // property-ish: a range of random-ish byte inputs must return Err, not panic.
    for seed in 0u32..200 {
        let bytes: Vec<u8> = (0..((seed % 40) + 1))
            .map(|i| ((seed.wrapping_mul(31).wrapping_add(i)) % 256) as u8)
            .collect();
        let _ = convert_svg(&bytes); // must not panic
    }
}

#[test]
fn opacity_folds_into_alpha() {
    let svg = br##"<svg viewBox="0 0 1 1"><path d="M0 0 L1 1" fill="#ff0000" fill-opacity="0.5"/></svg>"##;
    let out = convert_svg(svg).unwrap();
    // 0.5 * 255 = 127.5 -> 128 -> 0x80
    assert!(out.contains("android:fillColor=\"#80FF0000\""), "got: {out}");
}
