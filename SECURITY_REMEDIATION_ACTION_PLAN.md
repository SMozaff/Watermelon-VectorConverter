# Security Remediation Action Plan

**Repository:** `So-Muzaff/Watermelon-VectorConverter`  
**Prepared:** 16 August 2026  
**Status:** Approved for implementation planning  
**Basis:** `TECHNICAL_AUDIT_2026-08-16.md`

## Objective

This plan removes unnecessary Tauri filesystem privileges and closes the material security and release-quality risks identified by the technical audit. The target state is an offline converter and viewer that handle untrusted SVG, VectorDrawable XML, and ZIP input within explicit resource budgets, expose only necessary native capabilities, and ship only after automated verification passes.

> **Release gate:** No public desktop release should be cut until the critical filesystem, input-limiting, test-baseline, and frontend-compatibility workstreams have passed their acceptance criteria.

## Target security posture

The converter should process user-selected content as **bytes supplied from the frontend**, rather than granting the webview arbitrary filesystem access. The viewer may read a user-selected or OS-associated file through a narrowly owned Rust command, but it must apply size and path checks before parsing or rendering. Neither app should permit remote resources to load from a previewed SVG.

| Security boundary | Current state | Target state |
|---|---|---|
| Converter filesystem access | `fs:read-all` and `fs:write-all` granted to the main window. | No filesystem plugin access. Conversion continues over in-memory IPC data; saving uses the dialog-selected path only. |
| Viewer file access | Native command accepts arbitrary string paths. | Path is obtained from dialog/association flow, canonicalized where feasible, has a size limit, and is read only by narrowly scoped Rust code. |
| ZIP batch processing | Full archive entries are loaded and retained without aggregate limits. | Archive count, compressed/uncompressed size, ratio, path, nesting, and output-size budgets are enforced before work begins. |
| XML/SVG handling | Parser and preview paths have no explicit input/node/render limits. | A shared limits policy is enforced before UTF-8 decode, XML parsing, image rendering, and animation-frame generation. |
| Animated SVG preview | Sandboxed `srcdoc` frame allows scripts but has no explicit network policy. | Sandboxed frame receives a restrictive CSP that denies network, navigation, form, and plugin access. |
| Build release quality | Full test baseline is red; frontend ranges are not locked. | Tests, lint, audit, build, and WebView smoke tests are required and reproducible. |

Tauri’s capability model is intended to connect explicit permissions and scopes to particular windows/webviews, and file-system access can be narrowed to dialog-authorized paths rather than global access.[1] The plan uses this model by removing the filesystem plugin from the converter unless a future feature introduces a justified, scoped need.

## Implementation workstreams

### Workstream A — Restore the verification baseline

This workstream must be completed first. It prevents security fixes from being merged without a reliable signal that they preserve conversion behavior.

| Step | Change | Primary files | Acceptance criteria |
|---:|---|---|---|
| A1 | Correct the raw-string delimiters that collide with `#RRGGBB` XML values. Use a delimiter such as `r##"…"##` in `analyze_vd_tests.rs`. | `svg-converter-core/tests/analyze_vd_tests.rs` | `cargo test --workspace --all-targets` compiles and executes. |
| A2 | Fix all production Clippy findings or document a narrow exception where an API shape is intentionally clearer than the lint’s suggestion. | `svg-converter-core/src/**`, `desktop/src/**` | `cargo clippy --workspace -- -D warnings` passes. |
| A3 | Pin the Rust toolchain in `rust-toolchain.toml` and include the components used in CI. | New `rust-toolchain.toml` | Local and CI checks use the same supported compiler. |
| A4 | Commit npm lockfiles for each independent frontend and pin a compatible Svelte/SvelteKit/plugin/Vite set. | `tauri/package.json`, `tauri/package-lock.json`, `viewer/package.json`, `viewer/package-lock.json` | Clean install and production bundle have no missing-export warnings; a WebView smoke test loads each app. |

The branch should not proceed to packaging while any item in this workstream is red. The immediate owner should be the application maintainer, with a second reviewer validating the lockfile resolution before merging.

### Workstream B — Remove global Tauri filesystem access

The audited converter path reads selected files in the frontend and supplies raw bytes to the Rust conversion commands. It does not need the Tauri filesystem plugin for the reviewed conversion path. The secure default is therefore **plugin removal**, not a broad capability rewrite.

| Step | Change | Primary files | Verification |
|---:|---|---|---|
| B1 | Remove `tauri_plugin_fs::init()` from the converter builder. | `tauri/src-tauri/src/lib.rs` | Converter starts and all conversion/save flows remain functional. |
| B2 | Remove `tauri-plugin-fs` from Rust and frontend dependencies. | `tauri/src-tauri/Cargo.toml`, `tauri/package.json`, lockfiles | `cargo check` and `npm run build` pass without the plugin. |
| B3 | Delete `fs:read-all` and `fs:write-all` from `main-capability`. Retain only dialog permissions actually required by the frontend. | `tauri/src-tauri/tauri.conf.json` | Generated capability schema accepts configuration; inspection confirms no `fs:*` permission remains. |
| B4 | Add a capability regression test that parses configuration and fails when global filesystem permissions or an active fs plugin are reintroduced. | New `tauri/src-tauri/tests/security_config.rs` or a repository script | CI rejects `fs:read-all`, `fs:write-all`, and unreviewed filesystem plugin registration. |
| B5 | Review all command registration entries. Limit each command to a typed, validated contract and avoid commands that accept arbitrary shell-like strings. | `tauri/src-tauri/src/commands.rs`, `tauri/src-tauri/src/lib.rs` | Command inventory is documented; every registered command has a test or a reviewed rationale. |

If a later feature truly requires direct path I/O, it must use the following exception process: first acquire a path through a native dialog; then scope access only to that returned file or directory; enforce a non-symlink/canonical-path policy where supported; and record the justification in the capability configuration. Do not restore a global wildcard permission. Tauri’s own documentation provides a pattern for dynamically allowing a directory returned by a dialog, with scope limited to the permitted directory.[1]

### Workstream C — Establish one input-limits policy

Create an `InputLimits` structure in the core crate and require all public parse, conversion, preview, ZIP, and animation methods to receive it or use a safe default. Keep the initial production defaults conservative and configurable through a future, reviewed settings interface rather than unbounded user-controlled IPC parameters.

| Limit | Initial safe default | Enforcement point | Failure behavior |
|---|---:|---|---|
| Single loose SVG/XML input | 10 MiB | Before UTF-8 decode / parser call | Typed `InputTooLarge` error. |
| ZIP compressed input | 50 MiB | Before archive open | Typed `InputTooLarge` error. |
| ZIP selected entries | 1,000 | Entry enumeration | Typed `TooManyEntries` error. |
| ZIP aggregate uncompressed input | 100 MiB | Before each entry allocation/read | Typed `ArchiveTooLarge` error. |
| ZIP compression ratio | 100:1 | Per-entry metadata check | Typed `SuspiciousCompressionRatio` error. |
| ZIP pathname depth | 16 components | Entry-name validation | Typed `UnsafeArchivePath` error. |
| XML node count | 100,000 | XML parse options | Typed `XmlComplexityLimit` error. |
| Preview dimension | 4,096 pixels per edge | Before rasterization | Typed `RenderDimensionLimit` error. |
| Preview pixel area | 16 megapixels | Before rasterization | Typed `RenderDimensionLimit` error. |
| AVD frames | 180 | Animation sampling | Typed `FrameLimit` error. |
| AVD duration | 60 seconds | Animation metadata parse | Typed `AnimationDurationLimit` error. |
| Aggregate batch output | 200 MiB | ZIP writing | Typed `OutputTooLarge` error. |

The values above are starting points, not a substitute for product requirements. They should be measured against representative artwork and changed only alongside an updated performance/security test. The key requirement is that every allocation and expensive iteration has an enforced upper bound.

#### Required core-code changes

| Step | Change | Primary files | Acceptance criteria |
|---:|---|---|---|
| C1 | Add `InputLimits`, typed error variants, and utility validators for byte length, dimensions, archive paths, and aggregate budgets. | New `svg-converter-core/src/limits.rs`; `error.rs`; `lib.rs` | Error codes are stable, serializable, and covered by unit tests. |
| C2 | Replace `Document::parse` with `Document::parse_with_options` in SVG, VectorDrawable, animation detection, and AVD parsing. Configure `allow_dtd: false`, `entity_resolver: None`, and `nodes_limit`. | `svg_parser.rs`, `vd_parser.rs`, `animation.rs`, `animation_engine.rs`, `analysis.rs` | DTD/entity and node-exhaustion fixtures fail cleanly without an uncontrolled allocation. |
| C3 | Validate archive metadata before `Vec::with_capacity` and `read_to_end`. Read one vetted entry at a time, and avoid retaining all input buffers before conversion. | `batch_processor.rs` | ZIP-bomb, oversized-entry, excessive-entry, and aggregate-budget tests pass. |
| C4 | Normalize and reject unsafe archive entry names. Deny absolute paths, prefixes, `..`, empty normalized names, duplicate output names, and unsafe separators. | `batch_processor.rs` | Traversal, absolute, duplicate, and separator-confusion fixtures are rejected. |
| C5 | Bound `px`, fps, duration, and frame count in command wrappers before entering expensive render paths. | `tauri/src-tauri/src/commands.rs`, `viewer/src-tauri/src/commands.rs`, `image_export.rs`, `animation_engine.rs` | Fuzz/property tests demonstrate bounded output and cancellation behavior. |

`roxmltree` documents the safe untrusted-input pattern as an explicit parser configuration with DTD parsing disabled, no custom entity resolver, and a defined node limit.[2] The implementation should follow that documented control rather than depending only on defaults.

### Workstream D — Harden the viewer and desktop command boundary

The viewer correctly isolates animated SVG from the parent Tauri context with a sandboxed iframe, but it must prevent network access and make file-origin input validation explicit.

| Step | Change | Primary files | Acceptance criteria |
|---:|---|---|---|
| D1 | Add an iframe-local CSP in `svgSrcDoc`. Deny all network with `default-src 'none'`; allow only the narrowly required inline style/script behavior. | `viewer/src/routes/+page.svelte` | Tests prove that remote `<image>`, CSS `url()`, navigation, form, and external script URLs are blocked. |
| D2 | Re-evaluate whether `allow-scripts` is essential for animation. If retained, add a comment identifying the exact required animation features and security tests. | `viewer/src/routes/+page.svelte` | A security review records why the sandbox token is necessary. |
| D3 | Validate viewer file paths and metadata before `fs::read`; use bounded file reading and return a typed user-safe error. | `viewer/src-tauri/src/commands.rs` | Oversized, unreadable, directory, broken-symlink, and unsupported files fail predictably. |
| D4 | Restrict `open_url` to an allowlist of application-owned `https:` hosts and approved `mailto:` syntax, or replace OS-command fallback with a reviewed Tauri shell/open mechanism. | `tauri/src-tauri/src/commands.rs`, `about/+page.svelte` | Tests reject command separators, unsupported schemes, whitespace-control characters, and unapproved hosts. |
| D5 | Restrict `set_file_association` to a closed enum (`svg`, `xml`) at the Rust command boundary. | `tauri/src-tauri/src/commands.rs` | Invalid extension input cannot write registry/MIME state. |

Tauri can apply a CSP to production local assets, but the `srcdoc` viewer content needs its own explicit policy because it is dynamically constructed from untrusted SVG.[3]

### Workstream E — Dependency, CI, and release hardening

| Step | Change | Primary files | Acceptance criteria |
|---:|---|---|---|
| E1 | Trace `lru`, `ttf-parser`, and `paste` through the dependency graph; update direct parents or record an explicit temporary exception. | `Cargo.lock`, manifests, security notes | `cargo audit` has no unreviewed advisory; dependency decision is documented. |
| E2 | Add CI checks for formatting, full tests, strict Clippy, RustSec, npm production audit, frontend builds, and capability regression tests. Run them on pull requests and changes to `main`. | `.github/workflows/quality.yml`; existing workflow files | A package/release job depends on a green quality gate. |
| E3 | Use `npm ci` rather than mutable `npm install` in build/release jobs once lockfiles are committed. | `.github/workflows/desktop.yml`, `.github/workflows/android.yml` | CI resolves exactly the reviewed frontend dependency graph. |
| E4 | Split artifacts from releases. Upload test artifacts only from successful jobs and publish signed releases only after all matrix checks are green. | `.github/workflows/desktop.yml` | No release asset can be created from a failed or partial job. |
| E5 | Create a `SECURITY.md` describing private reporting, supported versions, and the maintained security properties of local-file processing. | New `SECURITY.md` | Security policy is discoverable and reviewed annually. |

`cargo audit` previously reported no vulnerability advisory but did report an unsound `lru` advisory and unmaintained `ttf-parser` and `paste` transitive dependencies.[4] This workstream manages that advisory debt and prevents it from silently accumulating.

## Testing strategy

The security work must be validated through deterministic fixtures, not only manual interaction. New fixtures must remain small enough for version control and must assert a typed error outcome rather than process termination.

| Test class | Minimum coverage | Required command or environment |
|---|---|---|
| Core unit tests | Size checks, node budget, safe error codes, dimension/frame limits. | `cargo test -p svg-converter-core` |
| Archive adversarial tests | ZIP bomb metadata, 1,001 entries, oversized aggregate, `../` and absolute names, duplicate normalized names. | `cargo test -p svg-converter-core --test batch` |
| XML/SVG adversarial tests | DTD/entity cases, excessive nesting/nodes, remote URLs, malformed UTF-8, extreme path data. | Dedicated parser/preview tests |
| Tauri command tests | Reject invalid paths, limits, URL schemes, and file-association extensions. | `cargo test` in both Tauri backend packages |
| Capability regression | Config contains no `fs:read-all` / `fs:write-all`; no unnecessary filesystem plugin is registered. | Repository script or backend integration test |
| Frontend smoke tests | Converter and viewer WebViews load; animated SVG cannot reach network; UI reports typed failures. | WebDriver/WebView-capable CI or OS-specific smoke job |
| Cross-platform packaging | Windows, Linux, and macOS packages build only after security gate passes. | Existing matrix workflow, refactored with dependencies |

## Delivery sequence and ownership

The sequence below assumes one primary maintainer and one independent reviewer. The elapsed-time labels describe relative complexity, not an implementation promise.

| Phase | Workstreams | Primary owner | Independent reviewer | Exit condition |
|---|---|---|---|---|
| 0 — Baseline | A | Maintainer | Reviewer | Tests and strict lint pass; dependency versions are reproducible. |
| 1 — Privilege reduction | B | Maintainer | Security reviewer | Converter runs with no filesystem plugin and no global fs capabilities. |
| 2 — Input safety | C | Maintainer | Security reviewer | All input-limit and archive-negative tests pass. |
| 3 — Viewer hardening | D | Maintainer | Frontend/security reviewer | Network isolation and command-boundary tests pass. |
| 4 — Release controls | E | Maintainer | Release reviewer | Required CI gates are enforced; signed package matrix is green. |

## Rollback and compatibility approach

Security changes should be released in one minor version because legitimate oversized inputs may now receive a deliberate error. Each limit error must include a stable code, a clear user-facing explanation, and—where appropriate—a product-supported alternative such as converting a smaller batch. Do not introduce a hidden “unlimited” switch.

If a converter workflow regresses after filesystem-plugin removal, first inspect the specific caller and use a narrow dialog-derived scope only when in-memory IPC is demonstrably insufficient. If a renderer regression occurs after CSP enforcement, add only the directive required by a documented SVG feature and accompany it with a security regression test. A rollback must never restore `fs:read-all` or `fs:write-all` globally.

## Definition of done

The remediation is complete only when all conditions in the following table hold.

| Area | Required final state |
|---|---|
| Filesystem authority | Converter does not register the fs plugin and has no broad filesystem capability. Any future direct I/O is dialog-originated and narrowly scoped. |
| Input handling | All public conversion, preview, archive, and animation paths enforce documented byte, complexity, and output limits. |
| Archive safety | Path traversal, duplicates, oversize data, and suspicious compression are rejected before full allocation. |
| Viewer isolation | Previewed SVG cannot make network requests, navigate the top-level app, or access the Tauri bridge. |
| Release quality | Full tests, strict lint, audits, reproducible frontend builds, smoke tests, and package matrix are green. |
| Governance | CI quality gate precedes release; advisory exceptions and supported versions are documented. |

## References

[1] [Tauri v2 — Capabilities, permissions, and scoped filesystem access](https://v2.tauri.app/security/capabilities/)

[2] [roxmltree — Parsing untrusted XML safely](https://github.com/RazrFalcon/roxmltree/blob/master/_autodocs/configuration.md)

[3] [Tauri v2 — Debugging, production devtools, and Content Security Policy behavior](https://v2.tauri.app/develop/debug/)

[4] [RustSec Advisory Database — `lru` panic-safety unsoundness, RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html)
