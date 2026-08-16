# Technical Audit: Watermelon Vector Converter

**Repository:** `So-Muzaff/Watermelon-VectorConverter`  
**Audited revision:** `ee2c586` on `main`  
**Audit date:** 16 August 2026  
**Author:** Manus AI

## Executive assessment

The repository has a promising separation between a Rust conversion core, desktop shells, and Tauri/Svelte frontends. The codebase also includes a substantial core test suite and cross-platform packaging workflow. However, it is **not ready for a production release** in its current state. The complete Rust test command is blocked by invalid test literals, the viewer frontend is resolved to an incompatible SvelteKit/Svelte combination that produces bundle warnings, and untrusted archive/XML/SVG inputs are processed without meaningful size or complexity limits.

The highest-priority work is to restore a green test baseline, constrain untrusted input processing, and reduce the Tauri capability and rendering attack surface. The audit found **three high-severity findings**, **four medium-severity findings**, and several lower-priority engineering concerns. No known Rust dependency CVE was reported by the vulnerability scan, but the resolved dependency graph has one unsoundness advisory and two unmaintained-crate advisories.

| Severity | Count | Release implication |
|---|---:|---|
| High | 3 | Must be remediated or explicitly risk-accepted before release. |
| Medium | 4 | Should be remediated in the next stabilization iteration. |
| Low / engineering | 5 | Schedule with normal maintenance work. |
| Informational | 2 | Track as validation and governance constraints. |

> **Overall release recommendation:** **Do not ship a new public release** until Findings H-01, H-02, and H-03 are closed and the frontend version matrix is pinned and runtime-tested.

## Scope and methodology

The audit covered the Rust workspace, both Tauri applications, Svelte frontend source, build configuration, GitHub Actions workflows, and dependency manifests/lockfiles. Review methods included static source inspection, permission/configuration review, targeted pattern searches, compilation/lint/build commands, dependency vulnerability scanning, and current-documentation comparison for Tauri v2 and `roxmltree`.

Current framework guidance was retrieved directly from the relevant documentation. Tauri v2 capabilities are intended to attach explicit permissions and scopes to individual windows/webviews, and file-system access can be constrained to named scopes rather than global access.[1] The `roxmltree` documentation recommends disabling DTDs, supplying no entity resolver, and setting a bounded node limit for untrusted XML.[2]

| Verification activity | Result | Interpretation |
|---|---|---|
| `cargo check --workspace` | **Passed**, with four dead-code warnings in the legacy Iced desktop target. | Production Rust source in the root workspace type-checks. |
| `cargo test --workspace --all-targets` | **Failed at compilation** in `analyze_vd_tests.rs`. | The repository has no green full-test baseline. |
| `cargo clippy --workspace -- -D warnings` | **Failed** with 11 lint errors. | Strict linting is not currently release-ready. |
| `npm run build` in `tauri/` | **Passed**. | Converter bundle builds, but dependency resolution needs pinning. |
| `npm run build` in `viewer/` | **Passed with runtime-export warnings**. | The bundle completes but may fail at runtime due to incompatible framework versions. |
| `cargo audit` | **0 vulnerabilities**, 3 warnings. | No CVE result, but advisory debt remains. |
| Production-only npm audit | **0 vulnerabilities**. | The 10 reported npm issues are in development/transitive tooling, not the production runtime set. |
| Tauri backend check | Blocked locally by missing `gdk-3.0` system headers. | This is an audit-host limitation; the repository CI installs required GTK packages. |

## Prioritized findings

### H-01 — Full test suite does not compile

**Evidence:** `svg-converter-core/tests/analyze_vd_tests.rs`, lines 16, 28, and 37–38 use `r#"…"#` raw strings containing XML color values such as `"#FFFF0000"`. The `"#` sequence ends an `r#` raw string, producing five Rust parser errors. `cargo test --workspace --all-targets` therefore terminates before tests execute.

| Risk | Impact | Recommendation |
|---|---|---|
| The main verification command is red. CI or local releases can proceed without a trustworthy test result if the test step is absent or bypassed. | Regressions in the converter core cannot be reliably detected; the failure also obscures failures in other targets. | Use a delimiter that cannot collide with attribute contents, such as `r##"…"##`, in the affected literals. Add `cargo test --workspace --all-targets` as a required CI check. |

This is a **high-severity delivery blocker**, rather than a runtime vulnerability. Fixing it is low effort and should occur first because all subsequent changes require a working baseline.

### H-02 — Untrusted ZIP/XML inputs have no resource limits

**Evidence:** `batch_processor.rs` reads every matching archive entry in full with `Vec::with_capacity(entry.size() as usize)` and `read_to_end`, then retains all selected files in memory before Rayon-parallel conversion. It does not limit compressed size, uncompressed size, entry count, aggregate size, compression ratio, path depth, or concurrent work. Both SVG and VectorDrawable parsers call `roxmltree::Document::parse` directly without an explicit node limit.

| Risk | Impact | Recommendation |
|---|---|---|
| A crafted ZIP bomb, many-entry archive, or structurally complex XML can consume memory and CPU, freezing or terminating the desktop application. | A user opening a malicious or accidentally huge archive can cause denial of service; parallel conversion amplifies peak memory use. | Enforce byte, entry-count, cumulative-uncompressed-size, compression-ratio, XML-node, render-pixel, frame-count, and timeout/cancellation limits at every public conversion entry point. Stream archive entries where possible rather than retaining all input and output in memory. |

`roxmltree` defaults DTD parsing off, which is a good baseline, but that does not replace an application-level node and byte budget. The documented safe pattern is `parse_with_options` with `allow_dtd: false`, `entity_resolver: None`, and a bounded `nodes_limit`.[2] Introduce one shared `InputLimits` policy and apply it before UTF-8 conversion, XML parsing, preview rendering, ZIP writing, and AVD frame production.

### H-03 — Tauri converter grants global filesystem access

**Evidence:** `tauri/src-tauri/tauri.conf.json` assigns the main window `fs:read-all` and `fs:write-all` in `main-capability`. The application’s primary conversion flow already transfers file bytes over IPC; broad plugin permissions therefore appear unnecessary for the reviewed paths.

| Risk | Impact | Recommendation |
|---|---|---|
| Any future frontend injection, compromised webview content, or unintended IPC surface can read and overwrite arbitrary user-accessible files. | High-impact confidentiality and integrity loss from what should be an offline conversion utility. | Remove `fs:read-all` and `fs:write-all`. Prefer no filesystem-plugin permissions where raw IPC bytes suffice. If paths are essential, use dialog-originated, runtime-scoped directories and the narrowest read/write permissions. |

Tauri’s security model is designed around narrowly scoped permissions and capabilities tied to specific windows or webviews; its filesystem examples demonstrate explicit allow/deny scopes rather than global read/write access.[1] This configuration should be treated as a release blocker because it is a preventable privilege-expansion risk.

### M-01 — The viewer claims network isolation but does not enforce it

**Evidence:** `viewer/src/routes/+page.svelte` injects raw animated SVG into `iframe[srcdoc]` with `sandbox="allow-scripts"`. The iframe prevents the SVG from accessing the parent DOM or Tauri bridge, which is valuable, but the surrounding HTML contains no restrictive Content Security Policy. Sandboxing alone does not forbid remote image/font/media loads or outbound requests initiated by untrusted SVG content.

| Risk | Impact | Recommendation |
|---|---|---|
| An opened SVG can contact remote endpoints, undermining the comment’s claim that no remote resource is reachable. | Privacy leakage, local-network probing potential, and a larger attack surface for the embedded renderer. | Add an iframe-local CSP before the SVG content, for example a policy that starts with `default-src 'none'`, permits only required inline styling/animation, and disallows network schemes. Consider SVG sanitization or a dedicated renderer for untrusted animated SVG. Test SVG `<image>`, CSS `url()`, SMIL, scripts, and form/navigation attempts. |

Tauri can apply a configured production CSP to local HTML assets, but neither Tauri configuration currently defines a CSP.[3] Because the viewer uses `srcdoc`, it needs its own deliberate policy rather than relying on the parent application configuration.

### M-02 — Frontend dependency ranges resolve to an incompatible framework set

**Evidence:** Both `tauri/package.json` and `viewer/package.json` permit broad caret ranges. A clean install resolved `@sveltejs/kit@2.70.2` with `svelte@4.2.20`. The viewer production build emitted warnings that `untrack`, `fork`, and `settled` are not exported by Svelte’s runtime but are imported by the resolved SvelteKit client runtime.

| Risk | Impact | Recommendation |
|---|---|---|
| A successful build may still ship a bundle that errors when the viewer WebView loads. Reproducibility is also lost because no committed npm lockfile is present. | Viewer availability and release reliability are at risk; future installs can resolve differently without source changes. | Add and commit separate lockfiles for `tauri/` and `viewer/`. Pin a verified compatible Svelte/SvelteKit/plugin/Vite matrix; either keep SvelteKit at a version supporting Svelte 4 or upgrade the application and tests to Svelte 5. Add a WebView smoke test after production bundling. |

The converter frontend built without this warning, but both applications resolve the same version ranges and should be locked together deliberately.

### M-03 — Batch conversion can preserve hostile archive paths

**Evidence:** `convert_zip` and `convert_vd_zip` accept `entry.name()` and pass a derived name to `ZipWriter::start_file` through `swap_ext`. An entry named `../../payload.svg` will yield `../../payload.xml` in the output archive. The application does not extract it itself, so this is not direct local file overwrite; however, it can produce an archive that is dangerous when extracted by another tool.

| Risk | Impact | Recommendation |
|---|---|---|
| Converted output can retain traversal paths, absolute-path-like names, duplicate names, or platform-confusing separators. | Downstream extraction tools may be exposed to Zip Slip-style overwrite risks; users can unknowingly redistribute malicious output. | Reject any entry whose path is non-relative, contains `..`, has an empty/invalid normal component, exceeds a depth limit, or collides after normalization. Normalize separators and generate deterministic safe output names. Add negative tests for traversal and duplicate names. |

### M-04 — Dependency advisory debt should be reduced

**Evidence:** `cargo audit` found no vulnerability advisories in 530 locked Rust dependencies, but it reported `paste@1.0.15` as unmaintained, `ttf-parser@0.25.1` as unmaintained, and `lru@0.16.4` with an unsoundness advisory involving panic safety in `LruCache::pop()`.[4] The complete npm tree reported 10 non-production vulnerabilities; production-only audit returned zero.

| Risk | Impact | Recommendation |
|---|---|---|
| Indirect dependencies can become unsupported or contain reachable unsound behavior over time. | Maintenance and security response cost grows; a future compiler or dependency update may surface latent issues. | Run `cargo tree -i` for each advisory, update direct parents where a compatible release exists, and document any accepted transitive risk. Add `cargo audit` and `npm audit --omit=dev` to scheduled CI. Keep development dependency remediation separate from runtime risk reporting. |

The `lru` report is significant but its exploitability depends on whether the affected method and panic path are reachable in the resolved transitive graph. Treat it as **medium** pending dependency-path confirmation rather than a confirmed application vulnerability.

## Lower-priority engineering findings

| ID | Finding | Evidence and recommended action |
|---|---|---|
| L-01 | Strict lint baseline is red. | `cargo clippy --workspace -- -D warnings` reports 11 errors, including manual checked arithmetic, explicit counters, unnecessary `map_or`, and an over-parameterized arc helper. Fix or narrowly justify each lint, then enforce Clippy in CI. |
| L-02 | Tauri devtools are compiled into both application backends. | `tauri/src-tauri/Cargo.toml` and `viewer/src-tauri/Cargo.toml` enable `tauri` feature `devtools`. Tauri documents this as a mechanism for enabling inspector tools in production builds.[3] Use a dedicated development feature and keep production release features minimal. |
| L-03 | The root Iced desktop target includes placeholder functionality and dead code. | `desktop/src/app/converter.rs` explicitly states that the converter UI is not implemented; `cargo check` additionally reports dead-code warnings in viewer/single-instance paths. Either complete this binary, remove it from the workspace, or document it as non-release experimental code. |
| L-04 | CI packages applications but does not validate the test/lint/audit baseline. | `.github/workflows/desktop.yml` is manual-dispatch only and builds/bundles across three platforms, but does not run workspace tests, strict Clippy, or dependency audits. Add those gates before packaging and trigger them on pull requests and main-branch changes. |
| L-05 | Platform-specific association logic has insufficient automated validation. | The macOS association module documents that it was not tested on a real Mac. The desktop workflow targets macOS, but no focused automated behavioral test covers the LaunchServices behavior. Add a macOS smoke test and document OS-version support. |

## Remediation sequence

The following order minimizes rework and establishes a trustworthy verification baseline before security-sensitive refactoring.

| Order | Work item | Completion criterion |
|---:|---|---|
| 1 | Repair raw-string delimiters in `analyze_vd_tests.rs`. | `cargo test --workspace --all-targets` compiles and passes. |
| 2 | Commit deterministic frontend lockfiles and correct the Svelte/SvelteKit compatibility matrix. | Clean install plus WebView smoke test has no runtime export warnings/errors. |
| 3 | Introduce a shared `InputLimits` configuration across ZIP, SVG, AVD, and preview APIs. | Automated tests reject oversize archives, traversal names, high node counts, and extreme render/frame requests predictably. |
| 4 | Replace broad Tauri filesystem permissions with dialog-scoped or no filesystem permissions. | Capability JSON grants only specific operations/paths justified by code. |
| 5 | Enforce a no-network viewer CSP and test hostile animated SVG cases. | External image/CSS/script navigation attempts are blocked and recorded in integration tests. |
| 6 | Add PR/main CI gates for tests, Clippy, RustSec, npm production audit, and package builds. | A release workflow cannot run after an unmet quality/security gate. |
| 7 | Address transitive dependency advisory paths and remove legacy/placeholder targets or promote them to maintained products. | Advisory decisions and supported targets are documented. |

## Suggested acceptance criteria

A release candidate should meet the following conditions: all Rust test targets pass; Clippy is clean or only has reviewed, narrow suppressions; both frontend builds use lockfile-pinned dependencies and pass a real WebView smoke test; the converter has no broad filesystem capability; hostile ZIP/XML/SVG fixture tests enforce documented limits; and the release workflow runs the complete quality gate before signing and publishing.

## Limitations

This was a source-and-build audit, not a penetration test. The Tauri native backends were not fully compiled on the audit host because the host lacked GTK development headers; this is not treated as a source defect because the existing Linux CI workflow installs the requisite system packages. No signed installer was executed, no macOS file-association behavior was exercised on hardware, and no performance profile was run against very large real-world artwork. These should be handled as follow-up validation tasks after the high-priority fixes.

## References

[1] [Tauri v2 — Permissions, capabilities, and filesystem scopes](https://v2.tauri.app/security/capabilities/)

[2] [roxmltree — Parsing untrusted XML safely](https://github.com/RazrFalcon/roxmltree/blob/master/_autodocs/configuration.md)

[3] [Tauri v2 — Debugging, production devtools, and CSP behavior](https://v2.tauri.app/develop/debug/)

[4] [RustSec Advisory Database — `lru` panic-safety unsoundness, RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html)

[5] [RustSec Advisory Database — `ttf-parser` unmaintained, RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html)

[6] [RustSec Advisory Database — `paste` unmaintained, RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)
