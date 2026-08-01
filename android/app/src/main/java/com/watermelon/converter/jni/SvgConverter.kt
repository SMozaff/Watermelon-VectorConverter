// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.jni

/** Abstraction over the native bridge so ViewModels can be unit-tested with a fake.
 *  The production implementation delegates to [SvgConverterNative] (the real .so). */
interface SvgConverter {
    fun convertSvg(svg: ByteArray): String
    fun convertZip(zip: ByteArray, cb: ProgressCallback): ByteArray

    /** Reverse conversion: VectorDrawable XML bytes -> SVG string. */
    fun convertVd(vdXml: ByteArray): String

    /** Reverse batch conversion: zip of .xml -> zip of .svg (+ .error.txt sidecars). */
    fun convertVdZip(zip: ByteArray, cb: ProgressCallback): ByteArray

    fun renderSvgPreview(svg: ByteArray, px: Int): ByteArray
    fun renderVdPreview(vdXml: String, px: Int): ByteArray
    fun cancel()
    /** Analyze a vector file's structure. Returns a JSON string (Contract C-5.0). */
    fun analyzeVector(bytes: ByteArray): String

    /** Analyze a VectorDrawable XML file's structure (reverse direction). Same JSON shape. */
    fun analyzeVdVector(bytes: ByteArray): String

    /** Detect whether a file is animated, and how (Contract C-5.1). */
    fun detectAnimation(fileBytes: ByteArray, isAvd: Boolean): AnimationKind

    /**
     * Render an AVD's animation frames (Contract C-5.2). Throws
     * [ConversionException] on failure, including UnsupportedFeature while
     * the underlying engine is still landing.
     */
    fun renderAvdFrames(avdBytes: ByteArray, fps: Int, maxFrames: Int, px: Int): AvdFramesResult
}

/** Production implementation — calls into libsvg_converter_core.so (Contract C-3). */
object RealSvgConverter : SvgConverter {
    override fun convertSvg(svg: ByteArray) = SvgConverterNative.nativeConvertSvg(svg)
    override fun convertZip(zip: ByteArray, cb: ProgressCallback) =
        SvgConverterNative.nativeConvertZip(zip, cb)
    override fun convertVd(vdXml: ByteArray) = SvgConverterNative.nativeConvertVd(vdXml)
    override fun convertVdZip(zip: ByteArray, cb: ProgressCallback) =
        SvgConverterNative.nativeConvertVdZip(zip, cb)
    override fun renderSvgPreview(svg: ByteArray, px: Int) =
        SvgConverterNative.nativeRenderSvgPreview(svg, px)
    override fun renderVdPreview(vdXml: String, px: Int) =
        SvgConverterNative.nativeRenderVdPreview(vdXml, px)
    override fun cancel() = SvgConverterNative.nativeCancel()
    override fun analyzeVector(bytes: ByteArray) =
        SvgConverterNative.nativeAnalyzeVector(bytes)
    override fun analyzeVdVector(bytes: ByteArray) =
        SvgConverterNative.nativeAnalyzeVdVector(bytes)
    override fun detectAnimation(fileBytes: ByteArray, isAvd: Boolean): AnimationKind =
        AnimationKind.fromOrdinal(SvgConverterNative.nativeDetectAnimation(fileBytes, isAvd))
    override fun renderAvdFrames(avdBytes: ByteArray, fps: Int, maxFrames: Int, px: Int): AvdFramesResult =
        AvdFramesResult.decode(SvgConverterNative.nativeRenderAvdFrames(avdBytes, fps, maxFrames, px))
}