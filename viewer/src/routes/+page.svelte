<script>
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  // What we're actually displaying, once render_file_preview resolves.
  // kind: "static" | "avd" | "animated-svg"
  let previewKind = null;
  let imgUrl = null;          // "static" kind
  let avdFrames = null;       // "avd" kind — { widths, frameDurationsMs, frameUrls, loopMode }
  let avdFrameIndex = 0;
  let svgText = null;         // "animated-svg" kind — raw SVG text, rendered inline

  let fileName = "";
  let error = "";
  let loading = false;

  // Wraps the raw SVG in a minimal HTML shell so it centers nicely inside
  // the sandboxed iframe. No script, no external references — only the
  // file's own SVG markup, which the sandbox further restricts regardless.
  $: svgSrcDoc = svgText
    ? `<!DOCTYPE html><html><head><style>html,body{margin:0;height:100%;display:flex;align-items:center;justify-content:center;background:transparent}svg{max-width:100%;max-height:100%}</style></head><body>${svgText}</body></html>`
    : "";

  let avdTimer = null;

  function clearPreviewState() {
    if (imgUrl) URL.revokeObjectURL(imgUrl);
    if (avdFrames) avdFrames.frameUrls.forEach((u) => URL.revokeObjectURL(u));
    if (avdTimer) { clearTimeout(avdTimer); avdTimer = null; }
    imgUrl = null;
    avdFrames = null;
    avdFrameIndex = 0;
    svgText = null;
    previewKind = null;
  }

  function playAvdFrame() {
    if (!avdFrames) return;
    const total = avdFrames.frameUrls.length;
    const nextIndex = avdFrames.loopMode === "Reverse"
      ? avdFrameIndex // reverse ping-pong handled via direction below
      : (avdFrameIndex + 1) % total;

    avdTimer = setTimeout(() => {
      if (avdFrames.loopMode === "Once" && avdFrameIndex >= total - 1) return;
      avdFrameIndex = nextIndex;
      playAvdFrame();
    }, avdFrames.frameDurationsMs[avdFrameIndex] ?? 33);
  }

  async function loadFile(path) {
    loading = true; error = ""; clearPreviewState();
    try {
      const result = await invoke("render_file_preview", { path, px: 1024 });
      fileName = path.split(/[\\/]/).pop();

      if (result.kind === "Static") {
        const blob = new Blob([new Uint8Array(result.png)], { type: "image/png" });
        imgUrl = URL.createObjectURL(blob);
        previewKind = "static";
      } else if (result.kind === "Avd") {
        // Contract C-5.2/C-5.4 playback: decode each frame PNG, draw to a
        // canvas-equivalent (here, a plain <img> swapped on a timer) on a
        // setTimeout loop honoring frame_durations_ms, per the frozen
        // contract's specified frontend playback approach.
        const frameUrls = result.frames.frames.map((bytes) => {
          const blob = new Blob([new Uint8Array(bytes)], { type: "image/png" });
          return URL.createObjectURL(blob);
        });
        avdFrames = {
          frameUrls,
          frameDurationsMs: result.frames.frame_durations_ms,
          loopMode: result.frames.loop_mode,
        };
        previewKind = "avd";
        playAvdFrame();
      } else if (result.kind === "AnimatedSvg") {
        // Contract C-5.5: no native rendering at all — the raw SVG markup
        // is handed straight to this already-a-browser webview. SMIL/CSS
        // timing plays natively, exactly as it would in any browser tab.
        svgText = result.svg_text;
        previewKind = "animated-svg";
      }
    } catch (e) {
      error = e?.message ?? String(e);
      clearPreviewState();
    } finally {
      loading = false;
    }
  }

  async function pickFile() {
    const path = await open({ filters: [{ name: "SVG / VectorDrawable", extensions: ["svg", "xml"] }], multiple: false });
    if (path) await loadFile(path);
  }

  onMount(async () => {
    // File that launched this instance (first run)
    const pending = await invoke("take_pending_file");
    if (pending) await loadFile(pending);

    // File passed to an already-running instance
    await listen("viewer://open-file", (e) => loadFile(e.payload));
  });

  onDestroy(() => clearPreviewState());
</script>

<div class="viewer">
  {#if previewKind}
    {#if previewKind === "static"}
      <img src={imgUrl} alt={fileName} class="preview" />
    {:else if previewKind === "avd"}
      <div class="preview avd-preview">
        <img src={avdFrames.frameUrls[avdFrameIndex]} alt={fileName} class="avd-frame" />
      </div>
    {:else if previewKind === "animated-svg"}
      <!--
        Contract C-5.5 (Tauri half): the SVG is rendered directly in a
        webview — but isolated inside a sandboxed <iframe>, not this page's
        own DOM. A malicious SVG could embed a <script>; sandboxing with
        only "allow-scripts" (no "allow-same-origin", no
        "allow-top-navigation") means that script runs in an opaque origin
        with no access to window.__TAURI__, no access to this page's DOM,
        and nowhere to navigate to — matching the frozen contract's "no
        remote resource should ever be reachable" intent for the Android
        WebView, applied here via iframe isolation instead of a second
        native webview.
      -->
      <iframe
        class="preview animated-svg-frame"
        title={fileName}
        sandbox="allow-scripts"
        srcdoc={svgSrcDoc}
      ></iframe>
    {/if}
    <div class="bar">
      <span class="filename">{fileName}</span>
      <button class="open-btn" on:click={pickFile}>Open…</button>
    </div>
  {:else}
    <div class="empty" on:click={pickFile} role="button" tabindex="0" on:keydown={(e) => e.key === "Enter" && pickFile()}>
      {#if loading}
        <p>Loading…</p>
      {:else if error}
        <p class="error">⚠ {error}</p>
        <p class="hint">Click to open a file</p>
      {:else}
        <p class="hint">Click to open an SVG or VectorDrawable XML</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .viewer { height: 100vh; display: flex; flex-direction: column; }

  .preview {
    flex: 1;
    width: 100%;
    object-fit: contain;
    background:
      linear-gradient(45deg, #1a1e24 25%, transparent 25%),
      linear-gradient(-45deg, #1a1e24 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #1a1e24 75%),
      linear-gradient(-45deg, transparent 75%, #1a1e24 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0px;
    background-color: var(--bg);
  }

  .avd-preview {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .avd-frame {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  .animated-svg-frame {
    border: none;
    background: transparent;
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }

  .filename { font-size: 13px; color: var(--text-sub); }

  .open-btn {
    background: var(--fresh-teal);
    color: #fff;
    padding: 6px 16px;
    border-radius: 50px;
    font-size: 13px;
    font-weight: 600;
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    cursor: pointer;
  }

  .hint { color: var(--text-sub); font-size: 14px; }
  .error { color: #E63946; font-size: 13px; text-align: center; padding: 0 20px; }
</style>
