# Immediate Release Blockers: Step-by-Step Implementation Plan

**Target revision:** `5f71d4052d4c0436ffe743964ccbc815bb569480`
**Scope:** The two P0 usability blockers only: the missing desktop converter and Android’s missing working/error feedback.
**Delivery principle:** Preserve the existing core conversion engine and Android success/export path. Add explicit UI state around those capabilities rather than duplicating conversion logic.

> **Definition of done:** A desktop user can choose or drop an SVG/XML file, convert in either direction, inspect the result, and copy or save it. An Android user always sees an intentional state—working, success, failure, or no active conversion—after choosing a file, with an understandable recovery path.

## Sequencing overview

| Step group | Dependency | Main files | Outcome |
| --- | --- | --- | --- |
| 1. Freeze the UI contracts | None | New acceptance fixtures and concise state definitions | Teams agree on exact supported inputs, resulting states, and primary actions. |
| 2. Build desktop state and input plumbing | Step 1 | `desktop/src/app/converter.rs`, `desktop/src/app/mod.rs` | The standard desktop launch opens an interactive converter shell. |
| 3. Connect desktop conversion and output actions | Step 2 | `desktop/src/app/converter.rs`, `svg-converter-core` public APIs | The shell performs forward/reverse conversion, previews output, and handles errors. |
| 4. Repair Android state modeling | Step 1 | Forward and reverse conversion view models | Working and error states contain the information needed to render and retry safely. |
| 5. Render Android working/error screens | Step 4 | `HomeScreen.kt`, `PreviewScreen.kt` | File selection never lands on an unexplained empty preview. |
| 6. Add verification and tighten release gates | Steps 2–5 | Desktop tests, Android tests, CI workflow | The two journeys are protected against regression. |

## 1. Freeze the release-scope contracts

Create a short engineering checklist before coding. The desktop release scope should support **one input file at a time** in both directions: SVG to Android VectorDrawable XML, and VectorDrawable XML to SVG. It should accept input through both a system file picker and drag-and-drop. Batch conversion, animated SVG support, advanced editing, and a code editor are explicitly out of scope for this blocker fix.

The Android release scope should retain its existing navigation model: after document selection, the app may continue navigating immediately to Preview, but Preview must render `Working`, `Done`, `Error`, and no-active-conversion states. A cancellation control should not be promised unless conversion-job cancellation is implemented in the view model and native layer.

| Contract | Required behavior | Explicit non-goal for this pass |
| --- | --- | --- |
| Desktop input | File picker and dropped `.svg` or `.xml` files; reject unsupported extensions with a clear message. | Directory import and batch ZIP import. |
| Desktop conversion | Forward and reverse conversion use the existing Rust core functions. | Manual path editing or design-authoring tools. |
| Desktop output | Show a rendered comparison where available, raw output text, Copy, Save As, and New conversion actions. | Syntax highlighting, diffing, and persistent project history. |
| Android feedback | The current result route visibly handles working, success, error, and idle. | Background notification and true cancellation. |
| Android recovery | Error state provides Retry and Choose another file. | Silent automatic retry. |

## 2. Implement the desktop converter foundation

### Step 2.1 — Replace the placeholder model with a real state machine

Refactor `desktop/src/app/converter.rs`. The current `Screen::Converter` state only returns a placeholder after splash; replace it with a conversion-specific state model. Keep the splash as a separate launch state, but route it to an interactive workspace.

A minimal model can use the following responsibilities:

| Type | Purpose |
| --- | --- |
| `Direction` | `SvgToVectorDrawable` or `VectorDrawableToSvg`; default forward but user-selectable. |
| `InputFile` | Source `PathBuf`, display name, and bytes after validated loading. |
| `ConversionState` | `Empty`, `Ready`, `Working { name }`, `Done { name, source_preview, output_preview, output_text }`, or `Error { name: Option<String>, message }`. |
| `Message` | Splash tick, direction selection, picker request/result, file drop, input load result, conversion request/result, copy output, save result, reset. |

Do not put synchronous file IO, conversion, rendering, clipboard access, or save dialogs inside `view`. Use `Task::perform` for each operation so the Iced event loop remains responsive. The existing viewer already provides proven patterns for `rfd::AsyncFileDialog`, dropped-file events, and `Task::perform`; reuse those patterns rather than adding a second UI framework.[1]

### Step 2.2 — Make the standard desktop window suitable for an actual workspace

Update `desktop/src/app/mod.rs` after the workspace is implemented. Replace the fixed `460 × 640` non-resizable converter window with a practical initial size, such as `1040 × 760`, plus a sensible minimum size. Make it resizable. The existing fixed launch configuration was appropriate for the splash but not for side-by-side previews and output controls.[2]

Maintain the viewer launch path as a separate capability for files opened through the operating system. It should not be the only route by which users can access a functional product.

### Step 2.3 — Build the empty/ready input workspace

The post-splash workspace should have one task-focused visual hierarchy:

1. A compact header with the app name and a two-option direction selector.
2. A large central input zone reading **Drop an SVG here** or **Drop a VectorDrawable XML here**, adjusted for the selected direction.
3. A secondary **Choose file** control that opens a filtered system picker.
4. Once a file is selected, a file-summary row showing the filename, detected type, and a **Change** action.
5. A single primary **Convert** action, disabled until the selected direction and detected input are compatible.

Handle `iced::Event::Window(window::Event::FileDropped(path))` in the converter subscription. Validate file extension as a fast early check, then determine true type from the document’s root element as the existing viewer does. If direction and detected root tag differ, show a reversible action such as **Switch to XML → SVG** rather than a generic failure.

### Step 2.4 — Connect the existing conversion core

Implement an async conversion job with the public APIs already exposed by `svg-converter-core`: `convert_svg` for forward conversion and `convert_vd` for reverse conversion.[3] Once conversion succeeds, render the input and output previews with the existing `image_export` module, using the appropriate SVG/VectorDrawable rendering functions. Preview rendering failure must not invalidate a successful text conversion; show a small inline “Preview unavailable” state while retaining Copy and Save As.

Conversion errors should be rendered as a dedicated error card, not a modal that disappears. The error card should contain the filename, a human-readable message from `ConversionError`, and two actions: **Try again** and **Choose another file**. Preserve the selected input for Retry, but do not retain its contents longer than the in-memory session.

### Step 2.5 — Implement result and output actions

Show the result only after `Done`. Keep the first desktop version deliberately simple:

| Area | Required content |
| --- | --- |
| Result summary | Success status, source filename, conversion direction, and output size. |
| Preview comparison | Source and converted output side by side or stacked on narrow windows, each with a clear label. |
| Output text | Scrollable monospaced read-only text area with a visible Copy action. |
| Primary actions | **Save As…** and **New conversion**. |
| Secondary action | **Open in viewer** only if it can use the existing viewer without duplicating state or creating a second instance. |

Use the platform clipboard integration already available through Iced or a minimal, well-supported desktop crate. Use a save-file dialog with a direction-derived default extension: `.xml` for forward output and `.svg` for reverse output. Write files atomically where practical, and surface any write failure in the same persistent error area.

### Step 2.6 — Keep the splash from blocking the functional flow

The existing desktop splash advances exactly 25 steps of 90ms, so it holds entry for 2.25 seconds independent of readiness.[1] For this release fix, either cap the splash at a short branding moment or remove its fake percentage. The functional workspace must be present and immediately interactive once the splash ends; the splash must never conceal unfinished setup.

## 3. Repair Android conversion state feedback

### Step 3.1 — Enrich the conversion state with retry context

Update both forward and reverse view models so that `Working` and `Error` include a user-facing source name. Store the most recently requested `Uri` in each view model for the life of the current session. Add a `retry()` method that invokes the existing conversion routine with that `Uri`; it must be a no-op if no previous source is available.

For the forward flow, update `ConvertUiState` in `ConversionViewModel.kt` from generic `Working` and `Error(message)` variants to equivalents such as `Working(sourceName)` and `Error(sourceName, message, retryAvailable)`. Apply the same shape to the reverse view model. The current view model already has distinct working, done, and error transitions, but its working/error variants lack source context and retry input.[4]

Preserve the existing conversion result, history logging, and export fields. Reset should clear the last URI, last source name, and result to avoid a retry against stale input after a new conversion session begins.

### Step 3.2 — Keep immediate navigation but prevent duplicate back-stack entries

The existing Home screen calls `convert(uri)` and immediately navigates to Preview.[5] Retain this behavior for minimal disruption, but navigate with `launchSingleTop = true` so duplicate picker callbacks cannot stack multiple identical Preview destinations. The destination must be prepared to show `Working` on the first frame.

Add a navigation icon to Preview that returns to the main pager without resetting a running operation. This gives users an escape path while maintaining the result state in the shared graph-scoped view model. Do not represent this as cancellation unless a cancellation primitive is added.

### Step 3.3 — Render all result-route states explicitly

Refactor `PreviewScreen.kt` so it selects the active direction first, then renders one of four state-specific bodies. The current implementation only casts `Done` states and shows “Nothing to preview yet” for everything else.[6]

| State | Required UI | Available actions |
| --- | --- | --- |
| No active conversion | Neutral empty state explaining that a file must be selected first. | **Choose file** returns to Home. |
| Working | Accessible progress indicator; “Converting *filename*”; compact text explaining that previews are being prepared. | Back to Home; no Export, Copy, or Share controls. |
| Error | Error icon, filename, human-readable error message, and guidance that no output was written. | **Retry** if URI exists; **Choose another file** resets and returns Home. |
| Done | Preserve the current conversion report, paired previews, Copy, Share, New, and Export controls. | Existing success actions only. |

Make top-bar title and actions state aware. Copy, Share, and Export must be absent while working or failed. This removes the risk of exporting stale content from a previous successful result.

### Step 3.4 — Use a clear active-direction resolver

Because the screen receives independent forward and reverse view models, add a small pure resolver that chooses the active state predictably. It should prefer the non-idle forward state, otherwise the non-idle reverse state, otherwise `Idle`. Write tests for forward working, forward error, reverse working, reverse error, forward done, reverse done, and both idle.

Avoid an architecture-wide navigation rewrite in this release. A resolver plus state-specific composables isolates the change to the existing result route while making later migration to one unified conversion state possible.

### Step 3.5 — Add semantic and visual feedback details

Use `semantics` or Material components so status is readable by TalkBack. Progress should have a descriptive state such as “Converting filename, please wait”; error actions should use clear text labels, not color alone. Ensure error copy is selectable or fully readable at increased font size. Keep the existing success content but change no-success controls so their availability is tied only to `Done` state.

## 4. Validation and release gates

### Step 4.1 — Add focused automated tests

The repository currently has comprehensive Rust core tests but no discovered Android unit or instrumentation test files for these screens.[7] Add narrow tests around the newly introduced state behavior rather than attempting full end-to-end desktop UI automation in the same change.

| Layer | Tests to add | Expected assertion |
| --- | --- | --- |
| Desktop Rust unit tests | Direction/type detection, state reducer transitions, conversion job for valid SVG, valid VectorDrawable, malformed input, and preview-render failure. | Conversion success remains usable even when preview generation fails; error state retains source/retry context. |
| Android view-model tests | Forward and reverse: idle → working → done; idle → working → error; retry reruns the previous URI; reset clears retry data. | Each user-visible state contains the expected source name and action availability. |
| Android Compose tests | Preview renders unique semantics/text for idle, working, error, and done. | Working and error never render “Nothing to preview yet”; Export/Copy are visible only for done. |
| Regression tests | Existing core conversion and Android emulator tests. | Core behavior and native bridge still pass unchanged. |

For testability, extract pure state-resolution and input-detection helpers from composables. If the current Android view model cannot be unit-tested due to hard-wired repositories, introduce interfaces for the file repository and settings lookup behind the existing production implementations, then pass fakes in tests. Keep this dependency-injection change minimal and local to the two conversion view models.

### Step 4.2 — Perform manual acceptance runs

Run the following matrix on each platform before release:

| Scenario | Desktop expected behavior | Android expected behavior |
| --- | --- | --- |
| Valid SVG input | Pick/drop → convert → source/output previews → Copy/Save As. | Choose → explicit Working → Done → Copy/Export. |
| Valid VectorDrawable XML input | Pick/drop → reverse convert → previews → Copy/Save As. | Choose reverse route → explicit Working → Done → Copy/Export. |
| Malformed or empty input | Persistent error card; Retry and Choose another file work. | Error screen shows a useful message, Retry, and Choose another file. |
| Preview renderer failure | Converted text remains copyable/saveable with preview-unavailable note. | Existing successful output controls remain available if conversion succeeded. |
| Repeated selection | New input replaces old session data; no stale output is exported. | No duplicate Preview destinations; no stale copy/export actions during work/error. |

For Android, repeat the working/error scenarios in light and dark themes and at 130% and 200% font scale. Validate both with TalkBack enabled. For desktop, resize the window below the initial size and confirm that the action hierarchy remains usable rather than clipping.

### Step 4.3 — Enforce build and CI gates

The blocker work is releasable only when all relevant commands and workflow jobs pass:

```bash
# Rust workspace quality gate
rustup run 1.97.1 cargo fmt --all -- --check
rustup run 1.97.1 cargo test --workspace
rustup run 1.97.1 cargo clippy --workspace --all-targets -- -D warnings

# Android gates
cd android
./gradlew test
./gradlew assembleDebug
./gradlew connectedAndroidTest
```

Then push the implementation as one or two focused commits and confirm the existing Quality and Security, Android, Desktop, and Build All Platforms workflows complete successfully. Do not mix the P0 usability changes with the separate color-system, file-browser, or icon-polish backlog unless they are required to preserve the new state screens.

## Release acceptance checklist

| Blocker | Release condition |
| --- | --- |
| Desktop converter gap | The standard desktop command launches a functional workspace; a nontechnical user can select/drop a file, choose direction, convert, inspect a result, copy it, and save it. The placeholder string is removed. |
| Android feedback gap | A selected file visibly produces Working, Done, or Error—not “Nothing to preview yet.” Error state preserves filename/message and exposes recovery actions. |
| Regression safety | Both conversion directions work on both platforms; malformed input does not crash; existing export behavior remains functional. |
| Accessibility baseline | New state and action controls have descriptive semantics and remain readable at 200% font scale. |
| CI readiness | Formatting, tests, strict linting, Android build/instrumentation tests, desktop packaging, and all platform-release jobs are green. |

## Recommended implementation order

Start with **Step 2.1 through Step 2.4** and land the desktop converter as a focused pull request. In parallel or immediately afterward, implement **Step 3.1 through Step 3.4** as a focused Android pull request. Complete Step 2.5, Step 3.5, and the full verification work as the release-candidate pass. This keeps every change small enough to review, preserves the shared Rust core, and ensures each blocker can be validated independently.

## References

[1]: https://github.com/So-Muzaff/Watermelon-VectorConverter/blob/5f71d4052d4c0436ffe743964ccbc815bb569480/desktop/src/app/converter.rs "Desktop converter shell and existing viewer interaction patterns"
[2]: https://github.com/So-Muzaff/Watermelon-VectorConverter/blob/5f71d4052d4c0436ffe743964ccbc815bb569480/desktop/src/app/mod.rs "Desktop window configuration"
[3]: https://github.com/So-Muzaff/Watermelon-VectorConverter/blob/5f71d4052d4c0436ffe743964ccbc815bb569480/svg-converter-core/src/lib.rs "Public core conversion APIs"
[4]: https://github.com/So-Muzaff/Watermelon-VectorConverter/blob/5f71d4052d4c0436ffe743964ccbc815bb569480/android/app/src/main/java/com/watermelon/converter/viewmodel/ConversionViewModel.kt "Forward conversion state transitions"
[5]: https://github.com/So-Muzaff/Watermelon-VectorConverter/blob/5f71d4052d4c0436ffe743964ccbc815bb569480/android/app/src/main/java/com/watermelon/converter/ui/screens/HomeScreen.kt "Android single-file conversion launch flow"
[6]: https://github.com/So-Muzaff/Watermelon-VectorConverter/blob/5f71d4052d4c0436ffe743964ccbc815bb569480/android/app/src/main/java/com/watermelon/converter/ui/screens/PreviewScreen.kt "Android Preview rendering behavior"
[7]: https://github.com/So-Muzaff/Watermelon-VectorConverter/tree/5f71d4052d4c0436ffe743964ccbc815bb569480/svg-converter-core/tests "Current Rust test suite"
