## Cover

# Release Blockers: Implementation Plan

**Watermelon Vector Converter**
Stakeholder review · Desktop conversion and Android state feedback

## Slide 1

# Two usability defects block release

- **Desktop:** the standard launch path ends at a placeholder instead of a converter.
- **Android:** a selected file can lead to an unexplained empty preview while conversion is working or has failed.
- The plan fixes only these P0 paths while preserving the existing Rust conversion engine and Android export flow.

## Slide 2

# The release scope is deliberately narrow

| Included now | Deferred deliberately |
| --- | --- |
| Single-file forward and reverse conversion | Batch ZIP redesign |
| Desktop file picker and drag-and-drop | Advanced vector editing |
| Desktop preview, copy, Save As, retry | Syntax highlighting and diff tools |
| Android Working, Done, Error, Idle states | Background notifications and true cancellation |

**Principle:** complete the core journey before broader UX polish.

## Slide 3

# Definition of done is user-observable

**Desktop user**

Choose or drop an SVG/XML file → select direction → convert → inspect result → copy or save.

**Android user**

Choose a file → always see **Working**, **Done**, **Error**, or **Idle** → recover clearly from failure.

> No command-line knowledge, unexplained empty screen, or stale export action.

## Slide 4

# Desktop becomes a real conversion workspace

1. Replace the post-splash placeholder with a state-driven workspace.
2. Add direction selector, drag/drop zone, filtered file picker, and file summary.
3. Run conversion and rendering asynchronously to keep the window responsive.
4. Present success, preview-unavailable, and persistent error states distinctly.
5. Add Copy, Save As, New conversion, and a resizable workspace window.

## Slide 5

# Desktop delivery reuses proven capabilities

| Existing capability | Planned use |
| --- | --- |
| Rust core conversion APIs | Forward SVG→VectorDrawable and reverse VectorDrawable→SVG |
| Existing native viewer patterns | Async file picker, dropped-file events, background tasks |
| Existing rendering pipeline | Source/output previews; degradation allowed without losing output text |
| Existing error taxonomy | Human-readable error card with Retry and Choose another file |

**Key safeguard:** preview failure never invalidates a successful text conversion.

## Slide 6

# Android receives an explicit conversion-state contract

| State | What the user sees | Valid next action |
| --- | --- | --- |
| Idle | “Choose a file to begin” | Return to Home |
| Working | Filename, accessible progress, preparation message | Return to Home |
| Error | Filename, reason, no-output reassurance | Retry or Choose another file |
| Done | Current report, previews, Copy, Share, Export | Current success actions |

## Slide 7

# Android changes remain low-risk and localized

- Enrich forward and reverse view models with source name, last URI, and a safe `retry()` path.
- Keep immediate navigation to Preview, but render Working/Error rather than “Nothing to preview yet.”
- Resolve active direction predictably across the existing forward and reverse view models.
- Hide Copy, Share, and Export unless the result state is **Done**.

## Slide 8

# Delivery proceeds in three controlled increments

| Increment | Output | Review point |
| --- | --- | --- |
| **1. Desktop foundation** | State model, interactive input shell, direction/type validation | Workspace is usable before conversion wiring |
| **2. Core journeys** | Desktop conversion/output; Android retry-aware state screens | Both blockers demonstrably resolved |
| **3. Release candidate** | Tests, manual scenario matrix, CI green | Release go/no-go decision |

**Recommended integration:** land desktop and Android changes as separate focused pull requests, then validate together.

## Slide 9

# Release gates protect against regression

- Desktop: valid SVG, valid VectorDrawable, malformed input, preview-render failure, repeated selection.
- Android: Working, Done, Error, Idle; forward and reverse directions; no stale Copy/Export actions.
- Accessibility: descriptive state semantics, light/dark review, and 130%/200% font-scale checks.
- Engineering: workspace formatting, tests, strict linting, Android build/emulator tests, and all four CI workflows green.

## Slide 10

# Stakeholder decision requested

**Approve the P0 scope:**

1. Functional desktop converter before desktop visual polish.
2. Explicit Android Working/Error recovery before secondary feature work.
3. Separate P0 implementation from color-system, file-browser, and broader visual-polish backlog.

**Outcome:** a release candidate with a complete, understandable conversion journey on both platforms.
