<!-- Convert: two directions stacked vertically. Each has ONE smart drop
     zone: a single file converts immediately, multiple loose files or any
     ZIP run as a batch job. Multiple ZIPs each produce their own output. -->
<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { save } from "@tauri-apps/plugin-dialog";
  import { writeFile } from "@tauri-apps/plugin-fs";
  import { get } from "svelte/store";
  import { settings } from "../lib/settings";
  import { onMount, onDestroy } from "svelte";
  import FileDropZone from "../components/FileDropZone.svelte";
  import PreviewPane from "../components/PreviewPane.svelte";
  import WatermelonButton from "../components/WatermelonButton.svelte";
  import ProgressBar from "../components/ProgressBar.svelte";

  // ── Shared batch progress (only one batch job runs at a time) ──
  let progDone = 0, progTotal = 0, progName = "";
  let unlisten = null;
  onMount(async () => {
    unlisten = await listen("batch://progress", (e) => {
      const p = e.payload;
      progDone = p.done; progTotal = p.total; progName = p.current_name;
    });
  });
  onDestroy(() => { if (unlisten) unlisten(); });
  $: progPct = progTotal > 0 ? Math.round((progDone / progTotal) * 100) : 0;

  /**
   * Run one batch job (already-zipped bytes) through the given command,
   * saving the result under a name derived from the source. Each call is
   * independent — used to run N separate jobs when N zips are selected.
   */
  async function runBatchJob(command, bytes, sourceName, outSuffix) {
    progDone = 0; progTotal = 0; progName = "";
    const result = await invoke(command, bytes); // raw body — see commands.rs
    const outBytes = new Uint8Array(result);
    const defaultName = sourceName.replace(/\.zip$/i, outSuffix);
    const path = await save({ defaultPath: defaultName, filters: [{ name: "ZIP", extensions: ["zip"] }] });
    if (path) await writeFile(path, outBytes);
    return outBytes;
  }

  // ── Section 1: SVG -> XML ──
  let sState = "idle"; // idle | working | done | error
  let svgBytes = null, svgPreviewPng = null, vdPreviewPng = null, vdXml = "", sourceName = "", sError = "";
  $: vdSizeKb = vdXml ? (new TextEncoder().encode(vdXml).length / 1024).toFixed(1) : "0";
  $: vdLines  = vdXml ? vdXml.split("\n").length : 0;

  let fwdBatchState = "idle"; // idle | working | done | error
  let fwdBatchCount = 0;
  let fwdBatchError = "";

  async function onFwdSingle(e) {
    const { bytes, name } = e.detail;
    svgBytes = bytes; sourceName = name;
    sState = "working"; svgPreviewPng = null; vdPreviewPng = null; vdXml = "";
    try {
      vdXml = await invoke("convert_svg", { svg: Array.from(bytes) });
      const px = get(settings).previewSize;
      const [sp, vp] = await Promise.all([
        invoke("render_svg_preview", { svg: Array.from(bytes), px }),
        invoke("render_vd_preview", { vdXml, px }),
      ]);
      svgPreviewPng = sp; vdPreviewPng = vp; sState = "done";
    } catch (err) { sError = err?.message ?? String(err); sState = "error"; }
  }

  async function onFwdBatchZip(e) {
    const { bytes, name } = e.detail;
    fwdBatchState = "working";
    try {
      await runBatchJob("convert_zip", bytes, name, "_vectors.zip");
      fwdBatchState = "done"; fwdBatchCount += 1;
    } catch (err) { fwdBatchError = err?.message ?? String(err); fwdBatchState = "error"; }
  }

  async function onFwdBatchLoose(e) {
    fwdBatchState = "working";
    try {
      const files = e.detail.files.map((f) => ({ name: f.name, bytes: Array.from(f.bytes) }));
      const zipBytes = new Uint8Array(await invoke("zip_loose_files", { files }));
      await runBatchJob("convert_zip", zipBytes, "batch.zip", "_vectors.zip");
      fwdBatchState = "done"; fwdBatchCount += 1;
    } catch (err) { fwdBatchError = err?.message ?? String(err); fwdBatchState = "error"; }
  }

  async function exportSvgSingle() {
    if (!vdXml) return;
    const path = await save({ defaultPath: sourceName.replace(/\.svg$/i, ".xml"), filters: [{ name: "VectorDrawable", extensions: ["xml"] }] });
    if (!path) return;
    await writeFile(path, new TextEncoder().encode(vdXml));
  }

  function resetFwd() {
    sState = "idle"; svgBytes = null; svgPreviewPng = null; vdPreviewPng = null; vdXml = ""; sourceName = ""; sError = "";
    fwdBatchState = "idle"; fwdBatchCount = 0; fwdBatchError = "";
  }

  // ── Section 2: XML -> SVG ──
  let rState = "idle";
  let vdBytes = null, rVdPreviewPng = null, rSvgPreviewPng = null, svgOut = "", rSourceName = "", rError = "";
  $: svgSizeKb = svgOut ? (new TextEncoder().encode(svgOut).length / 1024).toFixed(1) : "0";
  $: svgLines  = svgOut ? svgOut.split("\n").length : 0;

  let revBatchState = "idle";
  let revBatchCount = 0;
  let revBatchError = "";

  async function onRevSingle(e) {
    const { bytes, name } = e.detail;
    vdBytes = bytes; rSourceName = name;
    rState = "working"; rVdPreviewPng = null; rSvgPreviewPng = null; svgOut = "";
    try {
      svgOut = await invoke("convert_vd", { vdXml: Array.from(bytes) });
      const px = get(settings).previewSize;
      const [vp, sp] = await Promise.all([
        invoke("render_vd_preview", { vdXml: new TextDecoder().decode(bytes), px }),
        invoke("render_svg_preview", { svg: Array.from(new TextEncoder().encode(svgOut)), px }),
      ]);
      rVdPreviewPng = vp; rSvgPreviewPng = sp; rState = "done";
    } catch (err) { rError = err?.message ?? String(err); rState = "error"; }
  }

  async function onRevBatchZip(e) {
    const { bytes, name } = e.detail;
    revBatchState = "working";
    try {
      await runBatchJob("convert_vd_zip", bytes, name, "_svgs.zip");
      revBatchState = "done"; revBatchCount += 1;
    } catch (err) { revBatchError = err?.message ?? String(err); revBatchState = "error"; }
  }

  async function onRevBatchLoose(e) {
    revBatchState = "working";
    try {
      const files = e.detail.files.map((f) => ({ name: f.name, bytes: Array.from(f.bytes) }));
      const zipBytes = new Uint8Array(await invoke("zip_loose_files", { files }));
      await runBatchJob("convert_vd_zip", zipBytes, "batch.zip", "_svgs.zip");
      revBatchState = "done"; revBatchCount += 1;
    } catch (err) { revBatchError = err?.message ?? String(err); revBatchState = "error"; }
  }

  async function exportSvgFromVd() {
    if (!svgOut) return;
    const path = await save({ defaultPath: rSourceName.replace(/\.xml$/i, ".svg"), filters: [{ name: "SVG", extensions: ["svg"] }] });
    if (!path) return;
    await writeFile(path, new TextEncoder().encode(svgOut));
  }

  function resetRev() {
    rState = "idle"; vdBytes = null; rVdPreviewPng = null; rSvgPreviewPng = null; svgOut = ""; rSourceName = ""; rError = "";
    revBatchState = "idle"; revBatchCount = 0; revBatchError = "";
  }

  function onDropZoneError(e) { console.error(e.detail.message); }
</script>

<div class="convert-stack">

  <!-- ── Section 1: SVG -> XML ── -->
  <section class="direction-section">
    <h2 class="section-title">Convert SVG to XML</h2>

    {#if sState === "idle" && fwdBatchState === "idle"}
      <FileDropZone
        accept="svg"
        on:single-file={onFwdSingle}
        on:batch-zip={onFwdBatchZip}
        on:batch-loose={onFwdBatchLoose}
        on:error={onDropZoneError}
      />
    {/if}

    {#if sState === "working"}
      <ProgressBar label="Converting {sourceName}…" indeterminate />
    {:else if sState === "done"}
      <div class="result-header">
        <span class="success-badge">✓ {sourceName}</span>
        <div class="row-gap">
          <WatermelonButton on:click={exportSvgSingle} label="Export .xml" variant="teal" />
          <WatermelonButton on:click={resetFwd} label="New" variant="outline" />
        </div>
      </div>
      <div class="report-card">
        <div class="report-item"><span class="report-value">{vdSizeKb} KB</span><span class="report-label">Output size</span></div>
        <div class="report-divider"></div>
        <div class="report-item"><span class="report-value">{vdLines}</span><span class="report-label">XML lines</span></div>
      </div>
      <PreviewPane {svgPreviewPng} {vdPreviewPng} vdXml={vdXml} direction="forward" />
    {:else if sState === "error"}
      <div class="error-box">
        <p class="error-title">⚠ Conversion failed</p>
        <p class="error-msg">{sError}</p>
        <WatermelonButton on:click={resetFwd} label="Try again" variant="outline" />
      </div>
    {/if}

    {#if fwdBatchState === "working"}
      {#if progTotal > 0}
        <ProgressBar label="{progDone} / {progTotal} — {progName}" value={progPct} />
      {:else}
        <ProgressBar label="Reading batch…" indeterminate />
      {/if}
    {:else if fwdBatchState === "done"}
      <div class="done-box">
        <p class="success-badge">✓ {fwdBatchCount} batch {fwdBatchCount === 1 ? "job" : "jobs"} exported</p>
        <div class="row-gap" style="margin-top:14px">
          <WatermelonButton on:click={resetFwd} label="New batch" variant="outline" />
        </div>
      </div>
    {:else if fwdBatchState === "error"}
      <div class="error-box">
        <p class="error-title">⚠ Batch failed</p>
        <p class="error-msg">{fwdBatchError}</p>
        <WatermelonButton on:click={resetFwd} label="Try again" variant="outline" />
      </div>
    {/if}
  </section>

  <div class="section-divider"></div>

  <!-- ── Section 2: XML -> SVG ── -->
  <section class="direction-section">
    <h2 class="section-title">Convert XML to SVG</h2>

    {#if rState === "idle" && revBatchState === "idle"}
      <FileDropZone
        accept="xml"
        on:single-file={onRevSingle}
        on:batch-zip={onRevBatchZip}
        on:batch-loose={onRevBatchLoose}
        on:error={onDropZoneError}
      />
    {/if}

    {#if rState === "working"}
      <ProgressBar label="Converting {rSourceName}…" indeterminate />
    {:else if rState === "done"}
      <div class="result-header">
        <span class="success-badge">✓ {rSourceName}</span>
        <div class="row-gap">
          <WatermelonButton on:click={exportSvgFromVd} label="Export .svg" variant="teal" />
          <WatermelonButton on:click={resetRev} label="New" variant="outline" />
        </div>
      </div>
      <div class="report-card">
        <div class="report-item"><span class="report-value">{svgSizeKb} KB</span><span class="report-label">Output size</span></div>
        <div class="report-divider"></div>
        <div class="report-item"><span class="report-value">{svgLines}</span><span class="report-label">SVG lines</span></div>
      </div>
      <PreviewPane svgPreviewPng={rSvgPreviewPng} vdPreviewPng={rVdPreviewPng} vdXml={svgOut} direction="reverse" />
    {:else if rState === "error"}
      <div class="error-box">
        <p class="error-title">⚠ Conversion failed</p>
        <p class="error-msg">{rError}</p>
        <WatermelonButton on:click={resetRev} label="Try again" variant="outline" />
      </div>
    {/if}

    {#if revBatchState === "working"}
      {#if progTotal > 0}
        <ProgressBar label="{progDone} / {progTotal} — {progName}" value={progPct} />
      {:else}
        <ProgressBar label="Reading batch…" indeterminate />
      {/if}
    {:else if revBatchState === "done"}
      <div class="done-box">
        <p class="success-badge">✓ {revBatchCount} batch {revBatchCount === 1 ? "job" : "jobs"} exported</p>
        <div class="row-gap" style="margin-top:14px">
          <WatermelonButton on:click={resetRev} label="New batch" variant="outline" />
        </div>
      </div>
    {:else if revBatchState === "error"}
      <div class="error-box">
        <p class="error-title">⚠ Batch failed</p>
        <p class="error-msg">{revBatchError}</p>
        <WatermelonButton on:click={resetRev} label="Try again" variant="outline" />
      </div>
    {/if}
  </section>

</div>

<style>
  .convert-stack {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 4px;
  }

  .direction-section { padding: 4px 0; }

  .section-divider {
    height: 1px;
    background: var(--border);
    margin: 20px 0;
    flex-shrink: 0;
  }

  .section-title {
    font-size: 17px;
    font-weight: 700;
    color: var(--text-main);
    margin-bottom: 14px;
  }

  .result-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
    flex-wrap: wrap;
    gap: 8px;
  }

  .success-badge { font-weight: 600; color: var(--fresh-teal); font-size: 14px; }

  .row-gap { display: flex; gap: 8px; }

  .report-card {
    display: flex;
    align-items: center;
    background: var(--card-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 22px;
    margin-bottom: 16px;
    gap: 22px;
  }
  .report-item { display: flex; flex-direction: column; gap: 2px; }
  .report-value { font-size: 18px; font-weight: 700; color: var(--text-main); }
  .report-label { font-size: 11px; color: var(--text-sub); text-transform: uppercase; letter-spacing: .04em; }
  .report-divider { width: 1px; height: 32px; background: var(--border); }

  .done-box {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 24px;
  }

  .error-box {
    background: color-mix(in srgb, var(--watermelon-red) 8%, var(--card-bg));
    border: 1px solid color-mix(in srgb, var(--watermelon-red) 30%, transparent);
    border-radius: var(--radius);
    padding: 20px;
  }
  .error-title { color: var(--watermelon-red); font-weight: 600; margin-bottom: 6px; }
  .error-msg { font-family: monospace; font-size: 12px; color: var(--text-sub); margin-bottom: 14px; word-break: break-all; }
</style>
