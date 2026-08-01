// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Rust JNI side of the FFI bridge (Contract C-3). THIN marshalling only.
//!
//! Signatures here MUST match android/.../jni/SvgConverterNative.kt byte-for-byte.
//! The CI parity checker (ci/verify_interfaces.py) enforces this on every push.
//!
//! STATUS: write + type-check only in CI containers. The real round-trip
//! (JVM loads the .so, ByteArray<->String, exception throwing, progress
//! callbacks across 500 events) must be validated by an on-device
//! instrumented test (Module B test plan, Doc 5 §6).
//!
//! This module is compiled only for Android targets.
#![cfg(target_os = "android")]

use crate::batch_processor::{convert_zip, convert_vd_zip, ProgressEvent};
use crate::error::ConversionError;
use crate::image_export::{render_svg_preview, render_vd_preview};
use crate::convert_svg;
use crate::convert_vd;

use jni::objects::{JByteArray, JClass, JObject, JString, JThrowable, JValue};
use jni::sys::{jbyteArray, jint, jobject, jstring};
use jni::JNIEnv;
use std::sync::atomic::{AtomicBool, Ordering};

const EXCEPTION_CLASS: &str = "com/watermelon/converter/jni/ConversionException";

/// Process-wide cancel flag, shared between a running batch and nativeCancel().
static CANCEL: AtomicBool = AtomicBool::new(false);

// SAFETY: nativeConvertZip is called synchronously from a single JNI thread.
// convert_zip's coordinator (the only caller of the sink) runs on THIS same
// thread via std::thread::scope, so these raw handles are only ever
// dereferenced from the thread that owns the JNIEnv for the duration of this
// call — satisfying the invariant despite the wrapper asserting Send + Sync.
struct SendSyncEnvPtr(*mut jni::sys::JNIEnv);
unsafe impl Send for SendSyncEnvPtr {}
unsafe impl Sync for SendSyncEnvPtr {}
impl SendSyncEnvPtr {
    // A method call (rather than a bare `.0` field access) makes the closure
    // below capture this whole wrapper, not just the raw-pointer field —
    // otherwise Rust 2021's disjoint closure capture would capture the
    // field's bare pointer type directly, which isn't Send/Sync.
    fn get(&self) -> *mut jni::sys::JNIEnv { self.0 }
}

struct SendSyncObjPtr(jobject);
unsafe impl Send for SendSyncObjPtr {}
unsafe impl Sync for SendSyncObjPtr {}
impl SendSyncObjPtr {
    fn get(&self) -> jobject { self.0 }
}

/// Throw com.watermelon.converter.jni.ConversionException(code, message).
/// Never returns a value to Java on error; the pending exception is what Java sees.
fn throw_conversion(env: &mut JNIEnv, err: &ConversionError) {
    // Construct the exception object with (int, String) so the Kotlin side
    // can read .code. Fall back to a plain throw if construction fails.
    let msg = env.new_string(err.to_string()).ok();
    let built = match (env.find_class(EXCEPTION_CLASS), msg) {
        (Ok(cls), Some(m)) => env
            .new_object(
                cls,
                "(ILjava/lang/String;)V",
                &[JValue::Int(err.code() as jint), JValue::Object(&JObject::from(m))],
            )
            .ok(),
        _ => None,
    };
    if let Some(obj) = built {
        let _ = env.throw(JThrowable::from(obj));
    } else {
        let _ = env.throw_new(EXCEPTION_CLASS, err.to_string());
    }
}

fn bytes_from(env: &mut JNIEnv, arr: &JByteArray) -> Result<Vec<u8>, ConversionError> {
    env.convert_byte_array(arr)
        .map_err(|e| ConversionError::Internal(format!("jni byte read: {e}")))
}

/// nativeConvertSvg(svg: ByteArray): String   [Contract C-3]
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeConvertSvg<'a>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    svg: JByteArray<'a>,
) -> jstring {
    let null = JObject::null().into_raw();
    let bytes = match bytes_from(&mut env, &svg) {
        Ok(b) => b,
        Err(e) => { throw_conversion(&mut env, &e); return null; }
    };
    match convert_svg(&bytes) {
        Ok(xml) => match env.new_string(xml) {
            Ok(s) => s.into_raw(),
            Err(e) => { throw_conversion(&mut env, &ConversionError::Internal(e.to_string())); null }
        },
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeConvertVd(vdXml: ByteArray): String   [Contract C-4]
/// Reverse of nativeConvertSvg: VectorDrawable XML -> SVG string.
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeConvertVd<'a>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    vd_xml: JByteArray<'a>,
) -> jstring {
    let null = JObject::null().into_raw();
    let bytes = match bytes_from(&mut env, &vd_xml) {
        Ok(b) => b,
        Err(e) => { throw_conversion(&mut env, &e); return null; }
    };
    match convert_vd(&bytes) {
        Ok(svg) => match env.new_string(svg) {
            Ok(s) => s.into_raw(),
            Err(e) => { throw_conversion(&mut env, &ConversionError::Internal(e.to_string())); null }
        },
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeRenderSvgPreview<'a>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    svg: JByteArray<'a>,
    px: jint,
) -> jbyteArray {
    let null = JObject::null().into_raw();
    let bytes = match bytes_from(&mut env, &svg) { Ok(b) => b, Err(e) => { throw_conversion(&mut env, &e); return null; } };
    match render_svg_preview(&bytes, px.max(0) as u32) {
        Ok(png) => env.byte_array_from_slice(&png).map(|a| a.into_raw()).unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeRenderVdPreview(vdXml: String, px: Int): ByteArray   [Contract C-3]
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeRenderVdPreview<'a>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    vd_xml: JString<'a>,
    px: jint,
) -> jbyteArray {
    let null = JObject::null().into_raw();
    let xml: String = match env.get_string(&vd_xml) {
        Ok(s) => s.into(),
        Err(e) => { throw_conversion(&mut env, &ConversionError::Internal(e.to_string())); return null; }
    };
    match render_vd_preview(&xml, px.max(0) as u32) {
        Ok(png) => env.byte_array_from_slice(&png).map(|a| a.into_raw()).unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeConvertZip(zip: ByteArray, cb: ProgressCallback): ByteArray   [Contract C-3]
/// Progress is delivered by calling cb.onProgress(done, total, currentName)
/// from the coordinator. The JNIEnv passed in is valid for the duration of
/// this call (single-threaded w.r.t. the callback, because convert_zip's
/// coordinator runs on THIS thread).
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeConvertZip<'a>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    zip: JByteArray<'a>,
    cb: JObject<'a>,
) -> jbyteArray {
    let null = JObject::null().into_raw();
    CANCEL.store(false, Ordering::SeqCst);

    let bytes = match bytes_from(&mut env, &zip) { Ok(b) => b, Err(e) => { throw_conversion(&mut env, &e); return null; } };

    // Wrap the raw handles so the closure can satisfy Send + Sync. cb.as_raw()
    // returns the real underlying jobject handle (documented API), not the
    // address of the JObject wrapper.
    let env_ptr = SendSyncEnvPtr(env.get_raw());
    let cb_raw = SendSyncObjPtr(cb.as_raw());

    let sink = move |ev: ProgressEvent| {
        // SAFETY: convert_zip's coordinator invokes the sink synchronously on
        // the same thread that owns `env` for the duration of this JNI call.
        let mut env = unsafe { JNIEnv::from_raw(env_ptr.get()).expect("valid env") };
        let cb_obj = unsafe { JObject::from_raw(cb_raw.get()) };
        if let Ok(name) = env.new_string(&ev.current_name) {
            let _ = env.call_method(
                &cb_obj,
                "onProgress",
                "(IILjava/lang/String;)V",
                &[
                    JValue::Int(ev.done as jint),
                    JValue::Int(ev.total as jint),
                    JValue::Object(&JObject::from(name)),
                ],
            );
        }
    };

    match convert_zip(&bytes, &sink, &CANCEL) {
        Ok(out) => env.byte_array_from_slice(&out).map(|a| a.into_raw()).unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeConvertVdZip(zip: ByteArray, cb: ProgressCallback): ByteArray   [Contract C-4]
/// Reverse batch: every .xml in the zip -> .svg. Mirrors nativeConvertZip's
/// marshalling exactly; kept separate so the existing C-2/C-3 contract for
/// nativeConvertZip is never touched.
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeConvertVdZip<'a>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    zip: JByteArray<'a>,
    cb: JObject<'a>,
) -> jbyteArray {
    let null = JObject::null().into_raw();
    CANCEL.store(false, Ordering::SeqCst);

    let bytes = match bytes_from(&mut env, &zip) { Ok(b) => b, Err(e) => { throw_conversion(&mut env, &e); return null; } };

    let env_ptr = SendSyncEnvPtr(env.get_raw());
    let cb_raw = SendSyncObjPtr(cb.as_raw());

    let sink = move |ev: ProgressEvent| {
        // SAFETY: same as nativeConvertZip — convert_vd_zip's coordinator
        // invokes the sink synchronously on the thread that owns `env`.
        let mut env = unsafe { JNIEnv::from_raw(env_ptr.get()).expect("valid env") };
        let cb_obj = unsafe { JObject::from_raw(cb_raw.get()) };
        if let Ok(name) = env.new_string(&ev.current_name) {
            let _ = env.call_method(
                &cb_obj,
                "onProgress",
                "(IILjava/lang/String;)V",
                &[
                    JValue::Int(ev.done as jint),
                    JValue::Int(ev.total as jint),
                    JValue::Object(&JObject::from(name)),
                ],
            );
        }
    };

    match convert_vd_zip(&bytes, &sink, &CANCEL) {
        Ok(out) => env.byte_array_from_slice(&out).map(|a| a.into_raw()).unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeCancel()   [Contract C-3]
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeCancel<'a>(
    _env: JNIEnv<'a>,
    _cls: JClass<'a>,
) {
    CANCEL.store(true, Ordering::SeqCst);
}

fn analysis_json(a: &crate::analysis::VectorAnalysis) -> String {
    format!(
        r#"{{"width":{w},"height":{h},"viewportW":{vw},"viewportH":{vh},"pathCount":{pc},"groupCount":{gc},"usesPaths":{up},"usesGradients":{ug},"usesSolidColors":{us},"usesStrokes":{ust},"singleColorTintable":{sct},"tintColor":{tc},"isAnimated":{ia}}}"#,
        w = a.width, h = a.height, vw = a.viewport_w, vh = a.viewport_h,
        pc = a.path_count, gc = a.group_count,
        up = a.uses_paths, ug = a.uses_gradients, us = a.uses_solid_colors,
        ust = a.uses_strokes, sct = a.single_color_tintable,
        tc = match &a.tint_color {
            Some(c) => format!("\"{}\"", c),
            None => "null".to_string(),
        },
        ia = a.is_animated,
    )
}

/// nativeAnalyzeVector(bytes: ByteArray): String  [Contract C-5.0]
/// Returns a JSON string describing the vector's structure, or throws
/// ConversionException on failure.
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeAnalyzeVector<
    'a,
>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    bytes: JByteArray<'a>,
) -> jstring {
    let null = JObject::null().into_raw();
    let data = match bytes_from(&mut env, &bytes) {
        Ok(b) => b,
        Err(e) => { throw_conversion(&mut env, &e); return null; }
    };
    match crate::analyze_vector(&data) {
        Ok(a) => env.new_string(&analysis_json(&a)).map(|s| s.into_raw()).unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeAnalyzeVdVector(bytes: ByteArray): String  [reverse direction analysis]
/// Same JSON shape as nativeAnalyzeVector, but for VectorDrawable XML input.
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeAnalyzeVdVector<
    'a,
>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    bytes: JByteArray<'a>,
) -> jstring {
    let null = JObject::null().into_raw();
    let data = match bytes_from(&mut env, &bytes) {
        Ok(b) => b,
        Err(e) => { throw_conversion(&mut env, &e); return null; }
    };
    match crate::analyze_vd_vector(&data) {
        Ok(a) => env.new_string(&analysis_json(&a)).map(|s| s.into_raw()).unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// nativeDetectAnimation(fileBytes: ByteArray, isAvd: Boolean): Int   [Contract C-5.1]
///
/// Returns AnimationKind's ordinal: 0=None, 1=Avd, 2=SvgSmil, 3=SvgCss.
/// This exact numbering is part of the contract between this file and
/// SvgConverterNative.kt's decodeAnimationKind — if animation.rs's enum
/// variant order ever changes, this match arm (not the enum's derived
/// discriminant) is the source of truth, so a reorder there can't silently
/// desync the two sides.
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeDetectAnimation<
    'a,
>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    file_bytes: JByteArray<'a>,
    is_avd: jni::sys::jboolean,
) -> jint {
    let bytes = match bytes_from(&mut env, &file_bytes) {
        Ok(b) => b,
        // Detection never throws per its own contract (unparseable -> None),
        // so a byte-read failure here is the only path that can reach this
        // arm, and it degrades to None rather than surfacing a spurious
        // exception from a function callers expect to always succeed.
        Err(_) => return 0,
    };
    let kind = if is_avd != 0 {
        crate::animation::FileKind::Avd
    } else {
        crate::animation::FileKind::Svg
    };
    match crate::detect_animation(&bytes, kind) {
        crate::animation::AnimationKind::None => 0,
        crate::animation::AnimationKind::Avd => 1,
        crate::animation::AnimationKind::SvgSmil => 2,
        crate::animation::AnimationKind::SvgCss => 3,
    }
}

/// nativeRenderAvdFrames(avdBytes: ByteArray, fps: Int, maxFrames: Int, px: Int): String   [Contract C-5.2]
///
/// Returns a JSON string on success (see avd_frames_json below), following
/// the same "JSON string, not a constructed Java object" convention already
/// used by nativeAnalyzeVector/nativeAnalyzeVdVector in this file — chosen
/// over a custom binary encoding for consistency with that precedent, and
/// over constructing a Java object from JNI (more marshalling code, no
/// existing example of it in this file to follow). PNG frame bytes are
/// base64-encoded inside the JSON, matching how the Tauri side encodes them
/// for its own JSON transport (see commands.rs AvdFramesDto), so the two
/// platforms' wire formats are consistent.
/// Throws ConversionException (via throw_conversion) on any error,
/// including UnsupportedFeature while C-5.2's engine is still landing —
/// that is a normal, catchable error path, not a crash.
#[no_mangle]
pub extern "system" fn Java_com_watermelon_converter_jni_SvgConverterNative_nativeRenderAvdFrames<
    'a,
>(
    mut env: JNIEnv<'a>,
    _cls: JClass<'a>,
    avd_bytes: JByteArray<'a>,
    fps: jint,
    max_frames: jint,
    px: jint,
) -> jstring {
    let null = JObject::null().into_raw();
    let bytes = match bytes_from(&mut env, &avd_bytes) {
        Ok(b) => b,
        Err(e) => { throw_conversion(&mut env, &e); return null; }
    };
    match crate::render_avd_frames(&bytes, fps.max(0) as u32, max_frames.max(0) as u32, px.max(0) as u32) {
        Ok(frames) => env
            .new_string(&avd_frames_json(&frames))
            .map(|s| s.into_raw())
            .unwrap_or(null),
        Err(e) => { throw_conversion(&mut env, &e); null }
    }
}

/// Minimal base64 encoder (standard alphabet, with padding) so this module
/// doesn't need to pull in an external base64 crate for one call site.
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(b2 & 0x3f) as usize] as char } else { '=' });
    }
    out
}

fn avd_frames_json(f: &crate::animation_engine::AnimationFrames) -> String {
    let loop_mode = match f.loop_mode {
        crate::animation_engine::LoopMode::Once => "Once",
        crate::animation_engine::LoopMode::Repeat => "Repeat",
        crate::animation_engine::LoopMode::Reverse => "Reverse",
    };
    let durations = f
        .frame_durations_ms
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let frames_b64 = f
        .frames
        .iter()
        .map(|png| format!("\"{}\"", base64_encode(png)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"width":{w},"height":{h},"loopMode":"{lm}","frameDurationsMs":[{d}],"framesBase64":[{fr}]}}"#,
        w = f.width, h = f.height, lm = loop_mode, d = durations, fr = frames_b64,
    )
}