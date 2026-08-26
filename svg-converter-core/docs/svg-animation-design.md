# SMIL/CSS SVG animation playback — design note

Status: DRAFT for review. No implementation code has been written against
this note yet. This covers item 3 of the open-items list: real,
frame-accurate SMIL and CSS animation playback in the desktop viewer,
replacing the current `ViewState::Unsupported` fallback.

## 1. Why this needs a real engine, not a shortcut

`resvg`/`usvg` (0.44, this repo's only rendering dependency) are
static-only by the crate's own documented scope: no animation support at
all. This was verified empirically against the actual dependency version
pinned in `svg-converter-core/Cargo.toml`, not assumed from memory —
parsing an SVG with `<animate>`, `<animateTransform>`, and a `<style>`
block containing `@keyframes` through `usvg::Tree::from_str` and dumping
the resulting tree shows the animated `<circle>` reduced to a plain static
`Path` at its base `r="20"`, with the `<animate>`/`<animateTransform>`
children entirely absent from the tree, and the CSS-animated `<rect>`
likewise reduced to a plain static `Path` with no trace of `@keyframes` or
`animation-*`. There is nothing to "extract" from usvg's tree — it never
retained the animation data in the first place. A real implementation
needs its own parse pass over the raw SVG source for `<animate*>` elements
and `<style>` contents, independent of (though feeding into, at sample
time) the existing usvg-based static renderer.

This confirms the original C-5 contract's own design note
(`animation.rs`'s module doc): `SvgSmil`/`SvgCss` were always meant to
route to a browser engine (C-5.5), not this Rust pipeline. That WebView
path was later deliberately deleted in a separate rewrite specifically to
drop the WebView2/WebKitGTK dependency that was causing blank-window
failures. Building real SMIL/CSS playback now means writing a genuine
replacement for the thing that rewrite removed, not reinstating it —
reinstating a WebView here would silently undo the whole point of that
earlier work and must not happen without asking first.

## 2. Scope, per explicit user confirmation

"Perfect and flawless… no quick resolves" was confirmed to mean: real
frame-by-frame SMIL *and* CSS animation playback, evaluated live, not a
first-frame snapshot and not a partial/best-effort subset. This note is
sized to that scope. It is a large addition — comparable in size to the
existing AVD engine (`animation_engine.rs`, ~800 lines) but materially
harder, because AVD animates a small, finite, Android-defined keyframe
format, while SMIL and CSS are two independent, general-purpose timing
models with indefinite-duration and multi-animation-composition cases AVD
doesn't have to handle.

## 3. Where this hooks into the existing code

- `svg-converter-core/src/animation.rs` — `detect_animation` already
  classifies `AnimationKind::SvgSmil` / `SvgCss` correctly (SMIL takes
  priority when both are present in one file, matching the C-5.5 routing
  note). No change needed there; this design only changes what happens
  *after* detection.
- `desktop/src/app/viewer.rs` — currently, `load_file()` maps
  `AnimationKind::SvgSmil | SvgCss` straight to
  `LoadedFile::UnsupportedAnimatedSvg`, and `ViewState::Unsupported` just
  shows a static "not supported" message. This is the fallback path being
  replaced (for real playback) or kept (as the error path when the new
  engine itself fails to parse/evaluate a given file — see §8).
- The AVD engine's frame-swap-on-a-timer approach
  (`ViewState::Avd { handles: Vec<image::Handle>, current, elapsed_in_frame,
  frame_durations_ms }`, driven by `Message::Tick` on a 16ms
  `iced::time::every` subscription) is the right *rendering* precedent —
  same "produce a static raster at time T, cache it, swap on a timer"
  shape — but AVD pre-renders a small fixed frame set up front because its
  source format has a known finite duration. SMIL's `repeatCount="indefinite"`
  (and CSS's `animation-iteration-count: infinite`) mean total duration is
  not always known, so this can't always pre-render a frame list the same
  way (see §6).

## 4. Scene graph: parse once, keep the raw structure animatable

`usvg`'s tree is unsuitable as the live document, both because it drops
animation elements and because its own scope statement ("static SVG
subset") means relying on its internal representation for a moving target
is fighting the library, not using it. Instead:

- Parse the raw SVG source with `roxmltree` (already a dependency, already
  used by `animation.rs` and `vd_parser.rs`) into Watermelon's own small
  scene-graph type — call it `AnimSvgDocument` — that mirrors the subset of
  SVG this repo already understands structurally (the same shape
  `svg_parser.rs` already walks for the static SVG→VD path: `<svg>`,
  `<g>`, `<path>`/`<rect>`/`<circle>`/`<ellipse>`/`<line>`/`<polygon>`/
  `<polyline>`, fill/stroke/transform attributes), but where every
  animatable attribute is stored as `Animatable<T>` — either a fixed value
  or a fixed value plus a list of active animations targeting it (SMIL
  elements found as children of that node, or CSS animations matched via
  class/id/tag selector from the parsed `<style>` block).
- At sample time (§5), each `Animatable<T>` resolves to a concrete `T`, and
  the whole tree at that instant can be lowered into the SAME
  `NormalizedSvg`/`VdPath`/`VdGroup` model `svg_parser.rs` already
  produces for a static file, then handed to the EXISTING resvg render
  path unchanged. This is the same "convert to the existing intermediate
  model, reuse the existing renderer" strategy `image_export.rs` already
  uses for the VD-preview path (`vd_to_svg` → `render_svg_string`) — no
  new rasterizer, only a new "what values do these nodes have right now"
  step in front of the existing one.
- CSS selector matching only needs to support what real exported SVGs
  actually use for animation targeting — class (`.name`), id (`#name`),
  and tag (`rect`) selectors, evaluated once per animated element at parse
  time (selectors don't change at runtime; only the property values along
  the timeline do), not a general CSS selector engine.

## 5. Two independent timeline/sampler modules

SMIL and CSS have genuinely different timing/interpolation models and get
separate modules (`smil.rs`, `css_animation.rs`, both under
`svg-converter-core/src/`), sharing only the `Animatable<T>` scene-graph
plumbing above and a common `Sampler` trait:

```rust
trait AnimationSampler {
    /// Value of every animated attribute at time `t` (ms since the
    /// animation's own timeline start — see `TimelineExtent` below).
    fn sample(&self, t_ms: u64) -> ResolvedFrame;
    fn extent(&self) -> TimelineExtent;
}

enum TimelineExtent {
    Finite(u64),   // total duration in ms — safe to pre-render
    Indefinite,    // repeatCount/animation-iteration-count: indefinite
}
```

**SMIL (`smil.rs`)** — implement first; the spec is old, stable, and more
common in real hand-authored/tool-exported animated SVGs than CSS
animation. Required surface, all per animate/animateTransform/
animateMotion/animateColor/set element:
- `begin` / `dur` / `end` (offset-value and, at minimum, indefinite/`end`
  sync-base forms — event-based and syncbase-based begin values like
  `other.end` are real SMIL but rare in exported files; support the
  numeric-offset form fully and treat unsupported begin syntax as
  `begin="0s"` with a debug-only warning rather than failing the whole
  file, matching this codebase's general "degrade, don't crash on the
  uncommon case" posture — e.g. `svg_parser.rs`'s existing handling of
  unrecognized attributes).
- `repeatCount` (numeric and `indefinite`) / `repeatDur`.
- `fill="freeze"|"remove"` (post-animation value: hold at the last
  computed value, or revert to the base attribute value).
- `calcMode` — `linear` (default), `discrete`, `paced`, `spline` — with
  `keyTimes`/`keySplines` for the `spline` case. `paced` requires
  distance-based pacing between values, which for non-path/non-numeric
  attributes has no defined distance metric — fall back to `linear` for
  those per the SMIL spec's own guidance, don't invent one.
- Multiple simultaneous `<animate>` targeting the same attribute on the
  same element: SMIL's own additive/compositing model
  (`additive="sum"|"replace"`, `accumulate`) — implement `replace`
  (default, and the common case) fully; `additive="sum"`/`accumulate` can
  degrade to "last one wins" initially with a tracked follow-up, since
  getting `replace` right for the overwhelming majority of real files
  matters more than perfect coverage of a rarely-used compositing mode —
  flag this explicitly to the user as a known initial gap rather than
  silently shipping it as "done."
- `<animateMotion>` needs path-following (a `path`/`mpath` attribute plus
  `keyPoints`/`keyTimes`) — this is its own small sub-problem (sampling a
  point + tangent angle along an SVG path at a given progress fraction);
  the existing `arc.rs`/path-segment code in this crate (used for
  arc-to-cubic conversion) is the right starting point for "walk a path
  and get a point at parameter t," not a new path-math implementation.

**CSS (`css_animation.rs`)** — second pass, after SMIL is solid. Required
surface:
- `@keyframes` block parsing: percentage/`from`/`to` selectors, one or
  more property declarations per keyframe.
- `animation-name`, `animation-duration`, `animation-timing-function`,
  `animation-iteration-count` (numeric and `infinite`),
  `animation-direction` (`normal`/`reverse`/`alternate`/
  `alternate-reverse`), `animation-fill-mode`, `animation-delay` — either
  the shorthand `animation:` property or the longhands; real exported SVGs
  use both.
- Easing: keyword easings (`ease`, `ease-in`, `ease-in-out`, `linear`) map
  to fixed `cubic-bezier` control points per the CSS Easing spec;
  `cubic-bezier(x1,y1,x2,y2)` and `steps(n[, jump-term])` need actual
  numeric evaluation, not an approximation — a small self-contained
  cubic-bezier solver (Newton-Raphson or bisection on the bezier's x(t) to
  invert for t at a given progress) is a well-known, boundable amount of
  code; do not add a new crate dependency for this without checking size
  and audit exposure against the project's existing dependency discipline
  first (`cargo audit` is a hard CI gate — see `.github/workflows/
  quality.yml`).
- Only properties SVG actually animates meaningfully need support:
  `opacity`, `fill`, `stroke`, `stroke-width`, and `transform` (translate/
  scale/rotate — matching what `Animatable<T>` exposes from the scene
  graph in §4). Arbitrary CSS properties with no SVG rendering meaning
  (e.g. `color`, which affects `currentColor` — support this one, real
  files use it) can be scoped explicitly rather than guessed at.

## 6. Pre-render vs. live evaluation

Per user confirmation of "flawless" playback: **live evaluation**, not a
fixed pre-rendered frame list — for indefinite-duration animations
(`repeatCount="indefinite"` in SMIL, `infinite` in CSS), there is no total
duration to pre-render against, and even for finite ones, evaluating on
demand rather than committing to a fixed sample count/timestep up front
avoids picking an arbitrary resolution that might visibly stutter on a
`calcMode="spline"` or fast easing curve. Concretely:

- New `ViewState::AnimatedSvg { document: AnimSvgDocument, samplers: Vec<Box<dyn AnimationSampler>>, started_at: Instant, cached_frame: (u64, image::Handle), zoom: f32, offset: Vector }` variant, parallel to `Avd`, not a reuse of it (the AVD variant's `handles: Vec<image::Handle>` model assumes a finite pre-rendered set; this doesn't fit).
- Driven by the SAME 16ms `Message::Tick` subscription pattern already used for AVD (`ViewState::Avd` and this new variant both register the tick subscription; `subscription()`'s existing `matches!` check just grows to cover both).
- On each tick: compute elapsed time since `started_at`, sample every active animation via §5's samplers, lower the resolved scene graph to `NormalizedSvg` (§4), and render via the EXISTING `image_export`-style resvg call — but ONLY if the elapsed time has meaningfully changed the resolved frame vs. `cached_frame` (a cheap resolved-value comparison, or simply re-rendering every tick at 16ms/~60fps, which is what AVD playback already does unconditionally — matching that existing cost profile rather than introducing new dirty-tracking complexity is the simpler starting point; revisit only if profiling shows it's actually a problem).
- Pan/zoom reuse the exact same `Vector offset` / `f32 zoom` fields and the same `PannableImage` canvas-based renderer built for item 1 (desktop viewer pan) — nothing new needed there, since by the time this reaches the canvas it's just another `image::Handle` for a given instant.
- `repeatCount`/`animation-iteration-count` "indefinite" simply means `started_at` never needs resetting and the sampler's own modulo-by-duration logic (per §5) handles the visual loop — no special-casing needed in the viewer beyond "don't stop ticking."

## 7. Sequence of implementation (not to be done in one pass)

1. This design note, reviewed/confirmed by the user before any
   implementation code.
2. `Animatable<T>` scene graph + raw-SVG parse pass (§4) — no timeline
   evaluation yet, just parsing animate/style elements into the tree
   alongside the existing static attribute values. Testable in isolation:
   parse a file, assert the animation metadata is captured correctly.
3. SMIL sampler (§5) — timing model, interpolation modes, `fill` behavior,
   full test coverage against hand-written fixtures covering each
   `calcMode`, `repeatCount`/`repeatDur`, and `fill="freeze"|"remove"`.
4. Wire SMIL into `viewer.rs` (§6) end to end — this is the first point
   real animated files can actually play, and worth landing/validating
   before starting CSS.
5. CSS sampler (§5) as a second, separate pass — same testing rigor.
6. Wire CSS into `viewer.rs` (extends the same `AnimatedSvg` state/sampler
   plumbing SMIL already established).
7. Test against real, varied files — hand-written AND tool-exported (from
   Illustrator, Figma, Lottie-to-SVG converters) — not just synthetic
   fixtures, since real export tools produce animation markup shapes unit
   tests are unlikely to think to cover on their own.

## 8. Error handling posture

Matches this codebase's existing C-4/C-5 posture: a file that uses SMIL/
CSS features genuinely outside this engine's supported surface (e.g. SMIL
event-based `begin` values, or a CSS selector combinator beyond class/id/
tag) should degrade to the best static approximation it can produce (e.g.
render the base/first-keyframe values as a static image) with a visible,
honest "playback partially unsupported" notice — not silently wrong
output, and not a hard failure that regresses back to today's
"Unsupported" screen for files that are mostly playable. Complete parse/
sample failure (malformed SMIL/CSS the engine can't make sense of at all)
falls through to today's `ViewState::Error`, same as any other unreadable
file.

## 9. Explicitly out of scope for this note

- Reinstating any WebView/browser-engine dependency (see §1) — this note
  is written specifically to avoid that path; if implementation ever
  seems to be heading toward "just embed a browser," stop and ask rather
  than doing it.
- SVG filters, masks, gradients-as-animation-targets, and other static-SVG
  features usvg doesn't render — those are pre-existing rendering gaps
  unrelated to animation and out of scope here.
- Editing/authoring animated SVGs — this is playback only, matching the
  viewer's existing read-only nature for every other format it shows.
