import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const converterRoot = resolve(import.meta.dirname, "..");
const repositoryRoot = resolve(converterRoot, "..");
const viewerManifest = join(repositoryRoot, "viewer", "src-tauri", "Cargo.toml");
const sidecarDirectory = join(converterRoot, "src-tauri", "binaries");
const executableName = process.platform === "win32" ? "wvgc-viewer.exe" : "wvgc-viewer";
const explicitTarget = process.env.TAURI_TARGET_TRIPLE ?? process.env.TAURI_ENV_TARGET_TRIPLE;
const targetTriple = explicitTarget ?? execFileSync("rustc", ["--print", "host-tuple"], { encoding: "utf8" }).trim();

if (!targetTriple) {
  throw new Error("Could not determine the Rust target triple for the viewer sidecar.");
}

const cargoArguments = ["build", "--release", "--manifest-path", viewerManifest];
if (explicitTarget) {
  cargoArguments.splice(2, 0, "--target", explicitTarget);
}

console.log(`Building viewer sidecar for ${targetTriple}...`);
execFileSync("cargo", cargoArguments, { cwd: repositoryRoot, stdio: "inherit" });

const targetDirectory = explicitTarget
  ? join(repositoryRoot, "viewer", "src-tauri", "target", explicitTarget, "release")
  : join(repositoryRoot, "viewer", "src-tauri", "target", "release");
const sourceBinary = join(targetDirectory, executableName);
const sidecarBinary = join(sidecarDirectory, `wvgc-viewer-${targetTriple}${process.platform === "win32" ? ".exe" : ""}`);

if (!existsSync(sourceBinary)) {
  throw new Error(`Viewer sidecar was not produced at ${sourceBinary}.`);
}

mkdirSync(sidecarDirectory, { recursive: true });
if (existsSync(sidecarBinary)) {
  rmSync(sidecarBinary);
}
copyFileSync(sourceBinary, sidecarBinary);
console.log(`Prepared Tauri sidecar: ${sidecarBinary}`);
