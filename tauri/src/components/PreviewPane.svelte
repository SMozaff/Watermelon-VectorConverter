<script>
  export let svgPreviewPng = null;
  export let vdPreviewPng  = null;
  export let vdXml = "";
  /** "forward" (SVG->XML, default) or "reverse" (XML->SVG) — only changes labels. */
  export let direction = "forward";

  $: leftLabel  = direction === "reverse" ? "Original VD"  : "Original SVG";
  $: rightLabel = direction === "reverse" ? "Generated SVG" : "Generated VD";
  $: codeTitle  = direction === "reverse" ? "SVG XML" : "VectorDrawable XML";

  let xmlExpanded = false;

  function toDataUrl(arr) {
    if (!arr) return null;
    const blob = new Blob([new Uint8Array(arr)], { type: "image/png" });
    return URL.createObjectURL(blob);
  }

  $: svgUrl = toDataUrl(svgPreviewPng);
  $: vdUrl  = toDataUrl(vdPreviewPng);
</script>

<div class="preview-row">
  <div class="preview-tile">
    <p class="tile-label">{leftLabel}</p>
    {#if svgUrl}
      <img src={svgUrl} alt="{leftLabel} preview" />
    {:else}
      <div class="placeholder">No preview</div>
    {/if}
  </div>
  <div class="preview-tile">
    <p class="tile-label">{rightLabel}</p>
    {#if vdUrl}
      <img src={vdUrl} alt="{rightLabel} preview" />
    {:else}
      <div class="placeholder">No preview</div>
    {/if}
  </div>
</div>

{#if vdXml}
  <div class="xml-card">
    <div class="xml-header">
      <span class="xml-title">{codeTitle}</span>
      <button class="expand-btn" on:click={() => xmlExpanded = !xmlExpanded}>
        {xmlExpanded ? "Collapse" : "Expand"}
      </button>
    </div>
    {#if xmlExpanded}
      <pre class="xml-body">{vdXml}</pre>
    {/if}
  </div>
{/if}

<style>
  .preview-row { display: flex; gap: 12px; margin-bottom: 16px; }

  .preview-tile {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
    background: var(--card-bg);
    text-align: center;
  }

  .tile-label {
    font-size: 11px;
    color: var(--text-sub);
    margin-bottom: 8px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: .04em;
  }

  img {
    max-width: 100%;
    max-height: 260px;
    object-fit: contain;
    border-radius: 6px;
    background: var(--surface);
  }

  .placeholder {
    height: 160px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-sub);
    font-size: 13px;
  }

  .xml-card {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--card-bg);
    overflow: hidden;
    margin-bottom: 16px;
  }

  .xml-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .xml-title { font-weight: 600; font-size: 13px; color: var(--text-main); }

  .expand-btn {
    background: none;
    color: var(--fresh-teal);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border: none;
  }

  .xml-body {
    padding: 14px;
    font-family: "Cascadia Code", "Consolas", monospace;
    font-size: 12px;
    color: var(--text-mono);
    overflow-x: auto;
    max-height: 320px;
    overflow-y: auto;
    white-space: pre;
    background: var(--card-bg);
  }
</style>
