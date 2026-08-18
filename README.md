# Watermelon Vector Converter

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![Compose](https://img.shields.io/badge/Jetpack%20Compose-2024.10-6200EE?logo=android&logoColor=white)](https://developer.android.com/jetpack/compose)
[![Kotlin](https://img.shields.io/badge/Kotlin-1.9.x-7F52FF?logo=kotlin&logoColor=white)](https://kotlinlang.org/)
[![License: Proprietary](https://img.shields.io/badge/License-Proprietary-red.svg)](LICENSE.md)
[![Platforms](https://img.shields.io/badge/platforms-Android%20%7C%20Windows%20%7C%20Linux-brightgreen)]()
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Version](https://img.shields.io/badge/version-2.0-red)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](http://makeapullrequest.com)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Android](https://img.shields.io/badge/Android-8.0%2B-green?logo=android)](https://developer.android.com/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-purple?logo=tauri)](https://tauri.app/)

**A fast, fully offline, multiplatform converter between SVG vector graphics
and Android VectorDrawable XML — in both directions.**

Convert single files, multiple loose files, or ZIP batches between `.svg`
and Android `VectorDrawable` `.xml`, with an approximate dual visual preview
and flexible export — entirely offline, with zero network calls.

> **Copyright © 2026 Suhail Muzaffari. All rights reserved.**
> This is **proprietary, source-available** software. It is **not** open source.
> See [`LICENSE.md`](./LICENSE.md). The right to commercialize this software
> belongs exclusively to the copyright holder.

---

## Highlights

- **Offline-first.** No telemetry, no network, no external dependencies at runtime. Android builds ship without the `INTERNET` permission.
- **Bidirectional.** SVG → VectorDrawable XML and VectorDrawable XML → SVG, both driven by the same Rust engine.
- **One shared core.** A single high-performance Rust engine (`svg-converter-core`) drives every platform, so conversion behaves identically everywhere.
- **Smart batching.** Drop a single file for an instant conversion, drop several loose files or a ZIP for a batch job — each ZIP you select produces its own independent output.
- **Dual preview.** See the original file and the generated result side by side (both rendered approximately via resvg).
- **Multiplatform.** Native Android (Jetpack Compose), native-feeling desktop (Tauri) for Windows and Linux, and a lightweight companion viewer for opening SVG/VectorDrawable files directly from the file system.

## Platforms

| Platform | UI | Status |
|----------|----|--------|
| Android (8.0+) | Jetpack Compose + Material 3 | Native |
| Windows 10/11 | Tauri (Svelte frontend) | Desktop |
| Linux | Tauri (Svelte frontend) | Desktop |
| Desktop viewer (`wvgc-viewer`) | Tauri (Svelte frontend) | Companion app — registers as the default handler for `.svg`/`.xml` so double-clicking a file opens a lightweight preview window instead of the full converter |

## Architecture

A single Rust core is shared across all platforms; thin platform layers
bridge to it. Conversion works in both directions — the core exposes
`convert_svg`/`convert_vd` (single file) and `batch_processor::{convert_zip,
convert_vd_zip}` (batch), all built on the same parse → normalize → emit
pipeline in both directions.

```
svg-converter-core (Rust)   ← SVG↔VectorDrawable parsing, conversion, rendering, batch
        │
        ├── JNI bridge       → Android (Kotlin + Jetpack Compose)
        └── Tauri commands   → Desktop converter app (Windows / Linux)
                              → Desktop viewer app (wvgc-viewer, shares the same core)
```

The project follows an **Interface-First Execution Methodology**: interface
contracts between the Rust core and each platform layer (JNI for Android,
Tauri commands for desktop) are treated as the governing artifact — each
platform is built against those contracts independently, and changes to a
contract are made deliberately rather than incidentally.

## Repository Layout

```
Watermelon-VectorConverter/
├── svg-converter-core/   Rust core: SVG↔VectorDrawable parsing, conversion,
│                         preview rendering, batch processing, analysis, FFI
├── android/              Android app (Compose UI, ViewModels, JNI bridge)
├── tauri/                Desktop converter app (Tauri backend + Svelte frontend)
├── viewer/               Desktop companion viewer (separate Tauri binary,
│                         wvgc-viewer — file-association handler for .svg/.xml)
└── .github/workflows/    CI pipeline
```

## Status

This repository is published in **source-available** form for reference and
transparency. It is under active development and is not accepting external
contributions or redistribution at this time.

## License

Proprietary and source-available — see [`LICENSE.md`](./LICENSE.md).

- You may **view** the source.
- You may **not** use, copy, modify, distribute, compile, run, or
  commercialize it without prior written permission.
- Third-party dependencies retain their own licenses; see
  [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md).

For commercial licensing or any other use, contact **Suhail Muzaffari** —
so.muzaff@gmail.com.
