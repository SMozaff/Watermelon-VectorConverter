// Tier 3: gradient conversion tests. Copyright (c) 2026 Suhail Muzaffari.
use svg_converter_core::convert_svg;

fn conv(svg: &str) -> String { convert_svg(svg.as_bytes()).expect("should convert") }

#[test]
fn linear_gradient_emitted_as_aapt() {
    let svg = r##"<svg viewBox="0 0 24 24">
      <defs>
        <linearGradient id="g" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stop-color="#ff0000"/>
          <stop offset="1" stop-color="#0000ff"/>
        </linearGradient>
      </defs>
      <path d="M0 0 L24 0 L24 24 Z" fill="url(#g)"/>
    </svg>"##;
    let out = conv(svg);
    assert!(out.contains("xmlns:aapt"), "root needs aapt ns: {out}");
    assert!(out.contains("<aapt:attr name=\"android:fillColor\">"), "{out}");
    assert!(out.contains("android:type=\"linear\""), "{out}");
    assert!(out.contains("android:color=\"#FFFF0000\""), "first stop: {out}");
    assert!(out.contains("android:color=\"#FF0000FF\""), "last stop: {out}");
    assert!(out.contains("</path>"), "gradient path is not self-closing: {out}");
}

#[test]
fn radial_gradient_emitted() {
    let svg = r##"<svg viewBox="0 0 24 24">
      <defs>
        <radialGradient id="r" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stop-color="#fff"/>
          <stop offset="1" stop-color="#000"/>
        </radialGradient>
      </defs>
      <circle cx="12" cy="12" r="12" fill="url(#r)"/>
    </svg>"##;
    let out = conv(svg);
    assert!(out.contains("android:type=\"radial\""), "{out}");
    assert!(out.contains("android:gradientRadius=\"0.5\""), "{out}");
    // circle also converted to a path with curves
    assert!(out.contains("<path"));
}

#[test]
fn stop_opacity_folds_into_color_alpha() {
    let svg = r##"<svg viewBox="0 0 10 10">
      <defs><linearGradient id="g">
        <stop offset="0" stop-color="#ff0000" stop-opacity="0.5"/>
        <stop offset="1" stop-color="#00ff00"/>
      </linearGradient></defs>
      <path d="M0 0 L10 10" fill="url(#g)"/>
    </svg>"##;
    let out = conv(svg);
    // 0.5 -> 0x80
    assert!(out.contains("android:color=\"#80FF0000\""), "{out}");
}

#[test]
fn style_attribute_stops_parsed() {
    let svg = r##"<svg viewBox="0 0 10 10">
      <defs><linearGradient id="g">
        <stop offset="0" style="stop-color:#ff0000;stop-opacity:1"/>
        <stop offset="1" style="stop-color:#0000ff"/>
      </linearGradient></defs>
      <path d="M0 0 L10 10" fill="url(#g)"/>
    </svg>"##;
    let out = conv(svg);
    assert!(out.contains("android:color=\"#FFFF0000\""), "{out}");
    assert!(out.contains("android:color=\"#FF0000FF\""), "{out}");
}

#[test]
fn dangling_gradient_ref_does_not_crash() {
    // fill references a gradient id that doesn't exist -> no fill, no panic
    let svg = r##"<svg viewBox="0 0 10 10"><path d="M0 0 L10 10" fill="url(#missing)"/></svg>"##;
    let out = conv(svg);
    assert!(out.contains("<path"));
    assert!(!out.contains("aapt:attr"), "no gradient should be emitted: {out}");
}
