use svg_converter_core::analyze_vector;

fn wrap(body: &str) -> Vec<u8> {
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\">"
    );
    s.push_str(body);
    s.push_str("</svg>");
    s.into_bytes()
}

#[test]
fn detects_paths_and_dimensions() {
    let bytes = wrap("<path d=\"M2 2 L22 2 L22 22 Z\" fill=\"#FF0000\"/>");
    let a = analyze_vector(&bytes).expect("should analyze");
    assert!(a.uses_paths);
    assert_eq!(a.path_count, 1);
    assert!(a.uses_solid_colors);
    assert!(!a.uses_gradients);
    assert_eq!(a.viewport_w, 24.0);
    assert_eq!(a.viewport_h, 24.0);
}

#[test]
fn group_count() {
    let bytes = wrap("<g><path d=\"M0 0 L10 10\" fill=\"#FF0000\"/></g>");
    let a = analyze_vector(&bytes).expect("should analyze");
    assert_eq!(a.group_count, 1);
    assert_eq!(a.path_count, 1);
}

#[test]
fn detects_gradients() {
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\" viewBox=\"0 0 100 100\">"
    );
    s.push_str("<defs><linearGradient id=\"g1\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">");
    s.push_str("<stop offset=\"0\" stop-color=\"#FF0000\"/>");
    s.push_str("<stop offset=\"1\" stop-color=\"#0000FF\"/>");
    s.push_str("</linearGradient></defs>");
    s.push_str("<path d=\"M0 0 L100 0 L100 100 Z\" fill=\"url(#g1)\"/>");
    s.push_str("</svg>");
    let a = analyze_vector(s.as_bytes()).expect("should analyze");
    assert!(a.uses_gradients);
    assert!(!a.single_color_tintable);
}

#[test]
fn single_color_is_tintable() {
    let body = "<path d=\"M2 2 L10 2 Z\" fill=\"#000000\"/><path d=\"M12 12 L20 20 Z\" fill=\"#000000\"/>";
    let bytes = wrap(body);
    let a = analyze_vector(&bytes).expect("should analyze");
    assert!(a.single_color_tintable);
    assert!(a.tint_color.is_some());
}

#[test]
fn multi_color_is_not_tintable() {
    let body = "<path d=\"M2 2 L10 2 Z\" fill=\"#FF0000\"/><path d=\"M12 12 L20 20 Z\" fill=\"#00FF00\"/>";
    let bytes = wrap(body);
    let a = analyze_vector(&bytes).expect("should analyze");
    assert!(!a.single_color_tintable);
    assert!(a.tint_color.is_none());
}

#[test]
fn detects_strokes() {
    let bytes = wrap("<path d=\"M2 2 L22 22\" fill=\"none\" stroke=\"#000000\" stroke-width=\"2\"/>");
    let a = analyze_vector(&bytes).expect("should analyze");
    assert!(a.uses_strokes);
}

#[test]
fn detects_smil_animation() {
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\">"
    );
    s.push_str("<path d=\"M2 2 L22 22\" fill=\"#000000\">");
    s.push_str("<animate attributeName=\"opacity\" from=\"0\" to=\"1\" dur=\"1s\"/>");
    s.push_str("</path></svg>");
    let a = analyze_vector(s.as_bytes()).expect("should analyze");
    assert!(a.is_animated);
}

#[test]
fn detects_css_animation() {
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\">"
    );
    s.push_str("<style>@keyframes spin { from { opacity: 0; } to { opacity: 1; } }</style>");
    s.push_str("<path d=\"M2 2 L22 22\" fill=\"#000000\"/></svg>");
    let a = analyze_vector(s.as_bytes()).expect("should analyze");
    assert!(a.is_animated);
}

#[test]
fn static_file_not_animated() {
    let bytes = wrap("<path d=\"M2 2 L22 22\" fill=\"#000000\"/>");
    let a = analyze_vector(&bytes).expect("should analyze");
    assert!(!a.is_animated);
}

#[test]
fn invalid_svg_returns_error() {
    assert!(analyze_vector(b"not svg at all").is_err());
}