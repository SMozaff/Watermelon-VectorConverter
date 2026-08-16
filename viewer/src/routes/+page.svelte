<script>
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  let previewKind = null;
  let imgUrl = null;
  let avdFrames = null;
  let avdFrameIndex = 0;
  let svgText = null;
  let fileName = "";
  let error = "";
  let loading = false;

  // The preview is an opaque-origin, script-disabled document. Its CSP permits
  // only inline SVG/CSS animation and data images; it blocks every network,
  // form, navigation, and plugin capability.
  $: svgSrcDoc = svgText
    ? `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; media-src 'none'; font-src 'none'; connect-src 'none'; form-action 'none'; base-uri 'none'; object-src 'none'"><style>html,body{margin:0;height:100%;display:flex;align-items:center;justify-content:center;background:transparent}svg{max-width:100%;max-height:100%}</style></head><body>${svgText}</body></html>`
    : "";

  let avdTimer = null;

  function clearPreviewState() {
    if (imgUrl) URL.revokeObjectURL(imgUrl);
    if (avdFrames) avdFrames.frameUrls.forEach((url) => URL.revokeObjectURL(url));
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
    const nextIndex = (avdFrameIndex + 1) % total;
    avdTimer = setTimeout(() => {
      if (avdFrames.loopMode === "Once" && avdFrameIndex >= total - 1) return;
      avdFrameIndex = nextIndex;
      playAvdFrame();
    }, avdFrames.frameDurationsMs[avdFrameIndex] ?? 33);
  }

  function applyPreview(response) {
    clearPreviewState();
    fileName = response.name;
    const result = response.preview;
    if (result.kind === "Static") {
      const blob = new Blob([new Uint8Array(result.png)], { type: "image/png" });
      imgUrl = URL.createObjectURL(blob);
      previewKind = "static";
    } else if (result.kind === "Avd") {
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
      svgText = result.svg_text;
      previewKind = "animated-svg";
    }
  }

  async function requestPreview(command) {
    loading = true;
    error = "";
    try {
      const response = await invoke(command, { px: 1024 });
      if (response) applyPreview(response);
    } catch (cause) {
      clearPreviewState();
      error = cause?.message ?? String(cause);
    } finally {
      loading = false;
    }
  }

  async function pickFile() {
    await requestPreview("pick_file_preview");
  }

  onMount(async () => {
    await requestPreview("take_pending_file_preview");
    await listen("viewer://file-ready", () => requestPreview("take_pending_file_preview"));
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
      <iframe
        class="preview animated-svg-frame"
        title={fileName}
        sandbox=""
        srcdoc={svgSrcDoc}
      ></iframe>
    {/if}
    <div class="bar">
      <span class="filename">{fileName}</span>
      <button class="open-btn" on:click={pickFile}>Open…</button>
    </div>
  {:else}
    <div class="empty" on:click={pickFile} role="button" tabindex="0" on:keydown={(event) => event.key === "Enter" && pickFile()}>
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
    background-position: 0 0, 0 10px, 10px -10px, -10px 0;
    background-color: var(--bg);
  }
  .avd-preview { display: flex; align-items: center; justify-content: center; }
  .avd-frame { max-width: 100%; max-height: 100%; object-fit: contain; }
  .animated-svg-frame { border: none; background: transparent; }
  .bar { display: flex; align-items: center; justify-content: space-between; padding: 10px 16px; background: var(--surface); border-top: 1px solid var(--border); }
  .filename { font-size: 13px; color: var(--text-sub); }
  .open-btn { background: var(--fresh-teal); color: #fff; padding: 6px 16px; border-radius: 50px; font-size: 13px; font-weight: 600; }
  .empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; cursor: pointer; }
  .hint { color: var(--text-sub); font-size: 14px; }
  .error { color: #E63946; font-size: 13px; text-align: center; padding: 0 20px; }
</style>
