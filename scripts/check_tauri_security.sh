#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for config in "$root/tauri/src-tauri/tauri.conf.json" "$root/viewer/src-tauri/tauri.conf.json"; do
  if grep -Eq '"fs:(read-all|write-all)"' "$config"; then
    echo "Broad filesystem permission found in $config" >&2
    exit 1
  fi
done

if grep -R -n -E 'tauri_plugin_fs|@tauri-apps/plugin-fs' \
  "$root/tauri/src-tauri/Cargo.toml" "$root/tauri/src-tauri/src" \
  "$root/tauri/package.json" "$root/tauri/src" 2>/dev/null; then
  echo "Converter filesystem plugin usage is not allowed" >&2
  exit 1
fi

if grep -R -n -E 'render_file_preview|take_pending_file\(' \
  "$root/viewer/src-tauri/src" 2>/dev/null; then
  echo "Viewer path-based preview IPC is not allowed" >&2
  exit 1
fi

echo "Tauri security configuration checks passed."
