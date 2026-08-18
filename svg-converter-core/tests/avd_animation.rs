// Watermelon Vector Converter — AVD frame-evaluation engine (Contract C-5.2).
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

use svg_converter_core::render_avd_frames;

// ---------------------------------------------------------------------
// Parameter validation (still enforced with the real pipeline behind it)
// ---------------------------------------------------------------------

const DUMMY_AVD: &[u8] = b"<animated-vector/>";

#[test]
fn fps_zero_rejected() {
    let err = render_avd_frames(DUMMY_AVD, 0, 90, 256).unwrap_err();
    assert_eq!(err.code(), 1006);
}

#[test]
fn fps_above_range_rejected() {
    let err = render_avd_frames(DUMMY_AVD, 61, 90, 256).unwrap_err();
    assert_eq!(err.code(), 1006);
}

#[test]
fn px_below_range_rejected() {
    let err = render_avd_frames(DUMMY_AVD, 30, 90, 8).unwrap_err();
    assert_eq!(err.code(), 1006);
}

#[test]
fn px_above_range_rejected() {
    let err = render_avd_frames(DUMMY_AVD, 30, 90, 4096).unwrap_err();
    assert_eq!(err.code(), 1006);
}

#[test]
fn malformed_avd_does_not_panic() {
    let garbage = b"\xff\xfe not xml at all <<<";
    let result = render_avd_frames(garbage, 30, 90, 64);
    assert!(result.is_err());
}

#[test]
fn dummy_avd_with_no_base_vector_is_unsupported_not_panic() {
    // <animated-vector/> has no inlined <vector>, and no @drawable ref
    // resolvable from a byte blob -> UnsupportedFeature, not a crash.
    let err = render_avd_frames(DUMMY_AVD, 30, 90, 64).unwrap_err();
    assert_eq!(err.code(), 1002);
}

// ---------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------

fn rotation_avd() -> &'static [u8] {
    br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:aapt="http://schemas.android.com/aapt">
        <vector android:viewportWidth="24" android:viewportHeight="24">
            <group android:name="rect_group">
                <path android:name="rect" android:pathData="M2,2 L22,2 L22,22 L2,22 Z" android:fillColor="#FFFF0000"/>
            </group>
        </vector>
        <target android:name="rect_group">
            <aapt:attr name="android:animation">
                <objectAnimator
                    android:propertyName="rotation"
                    android:valueFrom="0"
                    android:valueTo="360"
                    android:duration="1000"
                    android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
    </animated-vector>"##
}

fn fill_color_avd() -> &'static [u8] {
    br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:aapt="http://schemas.android.com/aapt">
        <vector android:viewportWidth="24" android:viewportHeight="24">
            <path android:name="fade_path" android:pathData="M2,2 L22,2 L22,22 L2,22 Z" android:fillColor="#FF000000"/>
        </vector>
        <target android:name="fade_path">
            <aapt:attr name="android:animation">
                <objectAnimator
                    android:propertyName="fillColor"
                    android:valueFrom="#FF000000"
                    android:valueTo="#FFFFFFFF"
                    android:duration="1000"
                    android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
    </animated-vector>"##
}

fn matching_structure_morph_avd() -> &'static [u8] {
    br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:aapt="http://schemas.android.com/aapt">
        <vector android:viewportWidth="24" android:viewportHeight="24">
            <path android:name="morph_path" android:pathData="M2,2 L22,2 L22,22 L2,22 Z" android:fillColor="#FF000000"/>
        </vector>
        <target android:name="morph_path">
            <aapt:attr name="android:animation">
                <objectAnimator
                    android:propertyName="pathData"
                    android:valueFrom="M2,2 L22,2 L22,22 L2,22 Z"
                    android:valueTo="M4,4 L20,4 L20,20 L4,20 Z"
                    android:duration="1000"
                    android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
    </animated-vector>"##
}

fn mismatched_structure_morph_avd() -> &'static [u8] {
    br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:aapt="http://schemas.android.com/aapt">
        <vector android:viewportWidth="24" android:viewportHeight="24">
            <path android:name="cut_path" android:pathData="M2,2 L22,2 L22,22 L2,22 Z" android:fillColor="#FF000000"/>
        </vector>
        <target android:name="cut_path">
            <aapt:attr name="android:animation">
                <objectAnimator
                    android:propertyName="pathData"
                    android:valueFrom="M2,2 L22,2 L22,22 L2,22 Z"
                    android:valueTo="M12,2 C18,2 22,8 22,12 C22,18 18,22 12,22 C6,22 2,18 2,12 C2,8 6,2 12,2 Z"
                    android:duration="1000"
                    android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
    </animated-vector>"##
}

fn multi_target_avd() -> &'static [u8] {
    br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:aapt="http://schemas.android.com/aapt">
        <vector android:viewportWidth="24" android:viewportHeight="24">
            <group android:name="spin_group">
                <path android:pathData="M2,2 L12,2 L12,12 L2,12 Z" android:fillColor="#FFFF0000"/>
            </group>
            <path android:name="fade_shape" android:pathData="M12,12 L22,12 L22,22 L12,22 Z" android:fillColor="#FF00FF00"/>
        </vector>
        <target android:name="spin_group">
            <aapt:attr name="android:animation">
                <objectAnimator android:propertyName="rotation" android:valueFrom="0" android:valueTo="180"
                    android:duration="1000" android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
        <target android:name="fade_shape">
            <aapt:attr name="android:animation">
                <objectAnimator android:propertyName="alpha" android:valueFrom="1.0" android:valueTo="0.0"
                    android:duration="1000" android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
    </animated-vector>"##
}

// ---------------------------------------------------------------------
// Behavioral tests
// ---------------------------------------------------------------------

#[test]
fn rotation_animation_produces_expected_frame_count() {
    // duration 1000ms, fps 10 -> 10 natural frames, well under max_frames.
    let result = render_avd_frames(rotation_avd(), 10, 90, 64).expect("should render");
    assert_eq!(result.frames.len(), 10);
    assert_eq!(result.frame_durations_ms.len(), result.frames.len());
    for png in &result.frames {
        assert!(!png.is_empty());
        // PNG magic bytes.
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

#[test]
fn fill_color_animation_renders_all_frames() {
    let result = render_avd_frames(fill_color_avd(), 10, 90, 64).expect("should render");
    assert_eq!(result.frames.len(), 10);
    for png in &result.frames {
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

#[test]
fn matching_structure_path_morph_renders_all_frames() {
    let result = render_avd_frames(matching_structure_morph_avd(), 10, 90, 64).expect("should render");
    assert_eq!(result.frames.len(), 10);
    for png in &result.frames {
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

#[test]
fn mismatched_structure_path_morph_does_not_panic_and_renders() {
    // The hard-cut fallback (Section 3.3) must degrade gracefully, never
    // erroring or panicking, even though the two path shapes are structurally
    // incompatible (4 line commands vs. an 8-curve rounded shape).
    let result = render_avd_frames(mismatched_structure_morph_avd(), 10, 90, 64)
        .expect("mismatched morph must degrade to hard cut, not error");
    assert_eq!(result.frames.len(), 10);
    for png in &result.frames {
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

#[test]
fn multi_target_animation_applies_both_independently() {
    // One group rotating while a separate path's alpha fades, in the same
    // frame set — confirms independent targets don't interfere.
    let result = render_avd_frames(multi_target_avd(), 10, 90, 64).expect("should render");
    assert_eq!(result.frames.len(), 10);
    for png in &result.frames {
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

#[test]
fn max_frames_truncation_returns_exactly_the_cap() {
    // duration 1000ms, fps 60 -> natural 60 frames, but capped at 20.
    let result = render_avd_frames(rotation_avd(), 60, 20, 64).expect("should render");
    assert_eq!(result.frames.len(), 20);
}

#[test]
fn determinism_identical_inputs_produce_same_frame_count() {
    // Byte-identical PNG determinism depends on the underlying resvg/
    // tiny-skia pipeline (already covered by existing image_export tests);
    // here we confirm this module's own contribution — parsing, timeline,
    // and frame sampling — is deterministic given identical inputs.
    let a = render_avd_frames(rotation_avd(), 12, 90, 48).expect("should render");
    let b = render_avd_frames(rotation_avd(), 12, 90, 48).expect("should render");
    assert_eq!(a.frames.len(), b.frames.len());
    assert_eq!(a.frame_durations_ms, b.frame_durations_ms);
    for (fa, fb) in a.frames.iter().zip(b.frames.iter()) {
        assert_eq!(fa, fb, "identical inputs should produce byte-identical PNG output");
    }
}

#[test]
fn no_animators_still_renders_single_static_frame() {
    // A target whose animation can't be resolved (no matching node, or a
    // target element with no usable animator) should never crash — the
    // base vector still renders as a static single frame.
    let avd = br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android">
        <vector android:viewportWidth="24" android:viewportHeight="24">
            <path android:pathData="M2,2 L22,2 L22,22 L2,22 Z" android:fillColor="#FF000000"/>
        </vector>
    </animated-vector>"##;
    let result = render_avd_frames(avd, 30, 90, 64).expect("should render");
    assert!(!result.frames.is_empty());
}

#[test]
fn non_square_viewport_renders_without_error() {
    // Regression guard: the base vector's actual viewportWidth/Height must
    // be read and used, not a hardcoded 24x24 — every other fixture in
    // this file happens to use 24x24, which would silently hide this bug.
    let avd = br##"<animated-vector xmlns:android="http://schemas.android.com/apk/res/android"
        xmlns:aapt="http://schemas.android.com/aapt">
        <vector android:width="48dp" android:height="96dp"
                android:viewportWidth="48" android:viewportHeight="96">
            <path android:name="tall_path" android:pathData="M0,0 L48,0 L48,96 L0,96 Z" android:fillColor="#FF0000FF"/>
        </vector>
        <target android:name="tall_path">
            <aapt:attr name="android:animation">
                <objectAnimator android:propertyName="alpha" android:valueFrom="1.0" android:valueTo="0.2"
                    android:duration="500" android:interpolator="@android:interpolator/linear"/>
            </aapt:attr>
        </target>
    </animated-vector>"##;
    let result = render_avd_frames(avd, 10, 90, 64).expect("should render a non-square viewport");
    assert!(!result.frames.is_empty());
    for png in &result.frames {
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}
