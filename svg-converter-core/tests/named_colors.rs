// Named CSS color coverage (bugfix: only 7 colors were supported before).
// Copyright (c) 2026 Suhail Muzaffari.

use svg_converter_core::convert_svg;

#[test]
fn common_named_colors_beyond_the_original_seven_are_recognized() {
    let cases = [
        ("orange", "FFA500"),
        ("purple", "800080"),
        ("navy", "000080"),
        ("teal", "008080"),
        ("pink", "FFC0CB"),
        ("cyan", "00FFFF"),
        ("magenta", "FF00FF"),
        ("brown", "A52A2A"),
        ("lime", "00FF00"),
        ("gold", "FFD700"),
        ("silver", "C0C0C0"),
        ("maroon", "800000"),
        ("olive", "808000"),
        ("coral", "FF7F50"),
        ("indigo", "4B0082"),
        ("turquoise", "40E0D0"),
        ("salmon", "FA8072"),
        ("khaki", "F0E68C"),
        ("crimson", "DC143C"),
        ("chocolate", "D2691E"),
    ];
    for (name, expected_hex) in cases {
        let svg = format!(
            r##"<svg viewBox="0 0 24 24"><path d="M0,0 L1,1" fill="{name}"/></svg>"##
        );
        let out = convert_svg(svg.as_bytes())
            .unwrap_or_else(|e| panic!("failed to convert with fill={name}: {e}"));
        assert!(
            out.contains(&format!("#FF{expected_hex}")),
            "expected #FF{expected_hex} for fill={name}, got: {out}"
        );
    }
}

#[test]
fn transparent_keyword_produces_no_fill() {
    let svg = br##"<svg viewBox="0 0 24 24"><path d="M0,0 L1,1" fill="transparent"/></svg>"##;
    let out = convert_svg(svg).expect("should convert");
    assert!(!out.contains("android:fillColor"), "{out}");
}

#[test]
fn case_insensitive_named_colors() {
    let svg = br##"<svg viewBox="0 0 24 24"><path d="M0,0 L1,1" fill="OrangeRed"/></svg>"##;
    let out = convert_svg(svg).expect("should convert");
    assert!(out.contains("#FFFF4500"), "{out}");
}
