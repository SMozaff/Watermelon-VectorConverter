// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Animation Preview Engine — Contract C-5.
//! Phase 1: C-5.1 Animation Detection only. AVD frame rendering (C-5.2) and
//! the FFI bridges (C-5.3/C-5.4) are later phases and are NOT implemented
//! here. Detection is pure, deterministic, and must never panic or throw:
//! any unparseable input falls through to `AnimationKind::None`, which
//! routes to the existing static preview path unchanged.

/// Which raw-file family the caller is asking about, since the same bytes
/// could otherwise be ambiguous between an AVD-style XML and a plain SVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Avd,
    Svg,
}

/// Result of detection. Routing, per Contract C-5 Section 4.1:
///   None    -> existing static preview path, unchanged
///   Avd     -> Section 5 AVD frame-evaluation engine (C-5.2, later phase)
///   SvgSmil -> platform WebView (C-5.5)
///   SvgCss  -> platform WebView (C-5.5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationKind {
    None,
    Avd,
    SvgSmil,
    SvgCss,
}

/// C-5.1 — frozen once implementation begins (Section 4).
///
/// Pure, deterministic, no I/O — same character as `convert_svg` (C-1).
/// Never panics: any malformed/non-UTF-8/non-XML input yields
/// `AnimationKind::None` rather than propagating an error, since detection
/// failure must fall through to the existing static path (or existing
/// error path) unchanged, per the frozen contract.
pub fn detect_animation(file_bytes: &[u8], file_kind: FileKind) -> AnimationKind {
    let text = match std::str::from_utf8(file_bytes) {
        Ok(t) => t,
        Err(_) => return AnimationKind::None,
    };

    let doc = match roxmltree::Document::parse(text) {
        Ok(d) => d,
        Err(_) => return AnimationKind::None,
    };

    let root = doc.root_element();

    match file_kind {
        FileKind::Avd => detect_avd(&root),
        FileKind::Svg => detect_svg(&root),
    }
}

/// AVD root must be <animated-vector>; anything else (e.g. a plain
/// <vector>, or a file mistakenly routed here) is static/None rather than
/// an error — detection does not validate well-formedness beyond this.
fn detect_avd(root: &roxmltree::Node) -> AnimationKind {
    if root.tag_name().name() == "animated-vector" {
        AnimationKind::Avd
    } else {
        AnimationKind::None
    }
}

/// SVG root must be <svg>. Within it, SMIL animation elements
/// (<animate>, <animateTransform>, <animateMotion>, <set>) take priority
/// over CSS <style>/@keyframes detection when both are present in the same
/// file: SMIL presence means the file is at minimum SMIL-animated even if
/// it also carries unrelated CSS, and C-5.5 routes both cases to the same
/// WebView path regardless, so the ordering only affects which enum
/// variant is reported, not behavior.
fn detect_svg(root: &roxmltree::Node) -> AnimationKind {
    if root.tag_name().name() != "svg" {
        return AnimationKind::None;
    }

    if has_smil_descendant(root) {
        return AnimationKind::SvgSmil;
    }

    if has_css_keyframes(root) {
        return AnimationKind::SvgCss;
    }

    AnimationKind::None
}

const SMIL_TAGS: &[&str] = &["animate", "animateTransform", "animateMotion", "animateColor", "set"];

fn has_smil_descendant(root: &roxmltree::Node) -> bool {
    root.descendants()
        .any(|n| n.is_element() && SMIL_TAGS.contains(&n.tag_name().name()))
}

/// Looks for a <style> element anywhere in the document whose text content
/// contains an @keyframes rule. This is a lightweight lexical check
/// (detection must stay dependency-free and pure) — it does not parse CSS,
/// it only needs to know whether to route to the WebView, which will do
/// the real interpretation.
fn has_css_keyframes(root: &roxmltree::Node) -> bool {
    root.descendants().any(|n| {
        n.is_element()
            && n.tag_name().name() == "style"
            && n.text().map_or(false, |t| t.contains("@keyframes"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avd_root_detected() {
        let xml = br#"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android">
            <target android:name="rect" android:animation="@anim/rotate" />
        </animated-vector>"#;
        assert_eq!(detect_animation(xml, FileKind::Avd), AnimationKind::Avd);
    }

    #[test]
    fn static_vector_is_none() {
        let xml = br#"<vector xmlns:android="http://schemas.android.com/apk/res/android"
            android:width="24dp" android:height="24dp"
            android:viewportWidth="24" android:viewportHeight="24">
            <path android:pathData="M0,0L24,24" android:fillColor="#000000" />
        </vector>"#;
        assert_eq!(detect_animation(xml, FileKind::Avd), AnimationKind::None);
    }

    #[test]
    fn svg_smil_animate_detected() {
        let xml = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect width="10" height="10">
                <animate attributeName="x" from="0" to="90" dur="1s" repeatCount="indefinite" />
            </rect>
        </svg>"#;
        assert_eq!(detect_animation(xml, FileKind::Svg), AnimationKind::SvgSmil);
    }

    #[test]
    fn svg_smil_animate_transform_detected() {
        let xml = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <g>
                <animateTransform attributeName="transform" type="rotate"
                    from="0 50 50" to="360 50 50" dur="2s" repeatCount="indefinite" />
                <rect width="10" height="10" />
            </g>
        </svg>"#;
        assert_eq!(detect_animation(xml, FileKind::Svg), AnimationKind::SvgSmil);
    }

    #[test]
    fn svg_css_keyframes_detected() {
        let xml = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <style>
                @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
                rect { animation: spin 2s linear infinite; }
            </style>
            <rect width="10" height="10" />
        </svg>"#;
        assert_eq!(detect_animation(xml, FileKind::Svg), AnimationKind::SvgCss);
    }

    #[test]
    fn smil_takes_priority_over_css_when_both_present() {
        let xml = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <style>
                @keyframes fade { from { opacity: 1; } to { opacity: 0; } }
            </style>
            <rect width="10" height="10">
                <animate attributeName="x" from="0" to="90" dur="1s" />
            </rect>
        </svg>"#;
        assert_eq!(detect_animation(xml, FileKind::Svg), AnimationKind::SvgSmil);
    }

    #[test]
    fn static_svg_is_none() {
        let xml = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect width="10" height="10" fill="#ff0000" />
        </svg>"#;
        assert_eq!(detect_animation(xml, FileKind::Svg), AnimationKind::None);
    }

    #[test]
    fn malformed_xml_is_none_not_panic() {
        let xml = b"<svg><rect width=10></svg";
        assert_eq!(detect_animation(xml, FileKind::Svg), AnimationKind::None);
    }

    #[test]
    fn non_utf8_bytes_is_none_not_panic() {
        let bytes: &[u8] = &[0xFF, 0xFE, 0x00, 0x01, 0x02];
        assert_eq!(detect_animation(bytes, FileKind::Svg), AnimationKind::None);
        assert_eq!(detect_animation(bytes, FileKind::Avd), AnimationKind::None);
    }

    #[test]
    fn empty_bytes_is_none() {
        assert_eq!(detect_animation(b"", FileKind::Svg), AnimationKind::None);
        assert_eq!(detect_animation(b"", FileKind::Avd), AnimationKind::None);
    }

    #[test]
    fn wrong_root_for_avd_kind_is_none() {
        // An <svg> file mistakenly probed as FileKind::Avd should not be
        // misdetected as an AVD; it's just None (caller's job to route
        // FileKind correctly based on file extension/context).
        let xml = br#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="1" height="1"/></svg>"#;
        assert_eq!(detect_animation(xml, FileKind::Avd), AnimationKind::None);
    }

    #[test]
    fn determinism() {
        let xml = br#"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android">
            <target android:name="rect" android:animation="@anim/rotate" />
        </animated-vector>"#;
        let a = detect_animation(xml, FileKind::Avd);
        let b = detect_animation(xml, FileKind::Avd);
        assert_eq!(a, b);
    }
}
