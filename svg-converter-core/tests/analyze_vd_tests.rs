// Reverse-direction analysis tests. Copyright (c) 2026 Suhail Muzaffari.

use svg_converter_core::analyze_vd_vector;

fn wrap(body: &str) -> Vec<u8> {
    let mut s = String::from(
        r#"<vector xmlns:android="http://schemas.android.com/apk/res/android" android:viewportWidth="24" android:viewportHeight="24">"#
    );
    s.push_str(body);
    s.push_str("</vector>");
    s.into_bytes()
}

#[test]
fn detects_paths_and_dimensions() {
    let bytes = wrap(r#"<path android:pathData="M2,2 L22,2 L22,22 Z" android:fillColor="#FFFF0000"/>"#);
    let a = analyze_vd_vector(&bytes).expect("should analyze");
    assert!(a.uses_paths);
    assert_eq!(a.path_count, 1);
    assert!(a.uses_solid_colors);
    assert!(!a.uses_gradients);
    assert_eq!(a.viewport_w, 24.0);
    assert_eq!(a.viewport_h, 24.0);
}

#[test]
fn group_count() {
    let bytes = wrap(r#"<group><path android:pathData="M0,0 L10,10" android:fillColor="#FF000000"/></group>"#);
    let a = analyze_vd_vector(&bytes).expect("should analyze");
    assert_eq!(a.group_count, 1);
    assert_eq!(a.path_count, 1);
}

#[test]
fn single_color_tintable() {
    let bytes = wrap(concat!(
        r#"<path android:pathData="M0,0 L1,1" android:fillColor="#FF112233"/>"#,
        r#"<path android:pathData="M1,1 L2,2" android:fillColor="#FF112233"/>"#,
    ));
    let a = analyze_vd_vector(&bytes).expect("should analyze");
    assert!(a.single_color_tintable);
    assert_eq!(a.tint_color.as_deref(), Some("#FF112233"));
}

#[test]
fn invalid_root_rejected() {
    let bytes = b"<not-a-vector/>".to_vec();
    assert!(analyze_vd_vector(&bytes).is_err());
}
