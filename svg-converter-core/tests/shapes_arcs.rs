// Tier 1 + 2 tests: shapes and arc/smooth-curve handling.
// Copyright (c) 2026 Suhail Muzaffari.
use svg_converter_core::convert_svg;

fn conv(svg: &str) -> String { convert_svg(svg.as_bytes()).expect("should convert") }

/// Extract the first android:pathData="..." value so assertions don't trip over
/// letters in attribute names (fillColor, strokeWidth) or hex colors.
fn path_data(xml: &str) -> String {
    let key = "android:pathData=\"";
    let start = xml.find(key).expect("has pathData") + key.len();
    let end = xml[start..].find('"').expect("closing quote") + start;
    xml[start..end].to_string()
}

#[test]
fn rect_becomes_path() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><rect x="2" y="3" width="10" height="8" fill="#f00"/></svg>"##);
    assert!(out.contains("<path"), "{out}");
    assert!(out.contains("M2,3"), "{out}");
    assert!(out.contains("android:fillColor=\"#FFFF0000\""));
    assert!(!out.contains("UnsupportedFeature"));
}

#[test]
fn rounded_rect_uses_arcs_converted_to_cubics() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><rect x="0" y="0" width="20" height="20" rx="4" fill="#000"/></svg>"##);
    assert!(out.contains("<path"));
    assert!(out.contains("C"), "rounded rect should emit cubic curves: {out}");
}

#[test]
fn circle_becomes_path_with_curves() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="#00f"/></svg>"##);
    assert!(out.contains("<path"));
    assert!(out.contains("C"), "circle should be cubic curves: {out}");
    assert!(out.contains("android:fillColor=\"#FF0000FF\""));
}

#[test]
fn ellipse_becomes_path() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><ellipse cx="12" cy="12" rx="10" ry="6"/></svg>"##);
    assert!(out.contains("<path"));
    assert!(out.contains("C"));
}

#[test]
fn polygon_closes() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><polygon points="0,0 10,0 5,10" fill="#0f0"/></svg>"##);
    assert!(out.contains("M0,0"));
    assert!(out.contains("L10,0"));
    assert!(out.contains("Z"), "polygon must close: {out}");
}

#[test]
fn polyline_does_not_close() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><polyline points="0,0 10,0 5,10" stroke="#000" stroke-width="1" fill="none"/></svg>"##);
    let d = path_data(&out);
    assert!(d.contains("M0,0"));
    assert!(!d.contains('Z'), "polyline must NOT close: {d}");
}

#[test]
fn line_becomes_path() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><line x1="0" y1="0" x2="10" y2="10" stroke="#000" stroke-width="2"/></svg>"##);
    assert!(out.contains("M0,0 L10,10"), "{out}");
}

#[test]
fn arc_command_in_path_converts_to_cubics() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><path d="M2 2 A 10 10 0 0 1 22 22" stroke="#000" fill="none"/></svg>"##);
    let d = path_data(&out);
    assert!(d.contains('C'), "arc should become cubics: {d}");
    assert!(!d.contains('A'), "no raw arc command should remain: {d}");
}

#[test]
fn smooth_cubic_S_expands_to_C() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><path d="M0 0 C 1 1 2 2 3 3 S 5 5 6 6" fill="#000"/></svg>"##);
    let d = path_data(&out);
    assert!(!d.contains('S'), "S must expand: {d}");
    assert_eq!(d.matches('C').count(), 2, "two cubic segments expected: {d}");
}

#[test]
fn smooth_quad_T_expands_to_Q() {
    let out = conv(r##"<svg viewBox="0 0 24 24"><path d="M0 0 Q 1 1 2 2 T 4 4" fill="#000"/></svg>"##);
    let d = path_data(&out);
    assert!(!d.contains('T'), "T must expand: {d}");
    assert_eq!(d.matches('Q').count(), 2, "two quad segments expected: {d}");
}

#[test]
fn degenerate_zero_size_rect_skipped() {
    // zero-width rect -> skipped, not an error; SVG still valid with nothing drawn
    let out = conv(r##"<svg viewBox="0 0 24 24"><rect x="0" y="0" width="0" height="10"/><circle cx="5" cy="5" r="3"/></svg>"##);
    assert!(out.contains("<path")); // the circle still converts
}