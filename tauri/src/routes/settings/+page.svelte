<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { settings } from "../../lib/settings";
  import WatermelonButton from "../../components/WatermelonButton.svelte";

  const SIZES = [256, 400, 512, 768];

  function setPreviewSize(px) { settings.update((s) => ({ ...s, previewSize: px })); }
  function toggleDark()       { settings.update((s) => ({ ...s, darkMode: !s.darkMode })); }

  // File association state — Windows only. null = not yet loaded / unsupported.
  let svgAssoc = null;
  let xmlAssoc = null;
  let assocError = "";

  onMount(async () => {
    try {
      svgAssoc = await invoke("get_file_association", { ext: "svg" });
      xmlAssoc = await invoke("get_file_association", { ext: "xml" });
    } catch {
      // Non-Windows platform — leave as null, section stays hidden.
    }
  });

  async function toggleAssoc(ext) {
    const current = ext === "svg" ? svgAssoc : xmlAssoc;
    const next = !current;
    try {
      await invoke("set_file_association", { ext, enabled: next });
      if (ext === "svg") svgAssoc = next; else xmlAssoc = next;
      assocError = "";
    } catch (e) {
      assocError = e?.message ?? String(e);
    }
  }
</script>

<section class="settings">
  <h1 class="page-title">Settings</h1>

  <div class="setting-group">
    <h2 class="group-title">Preview size</h2>
    <p class="group-sub">Resolution used when rendering conversion previews.</p>
    <div class="chips">
      {#each SIZES as px}
        <button
          class="chip"
          class:active={$settings.previewSize === px}
          on:click={() => setPreviewSize(px)}
        >{px}px</button>
      {/each}
    </div>
  </div>

  <div class="setting-group">
    <h2 class="group-title">Appearance</h2>
    <p class="group-sub">Toggle between light and dark interface.</p>
    <label class="toggle">
      <input type="checkbox" checked={$settings.darkMode} on:change={toggleDark} />
      <span class="toggle-track"><span class="toggle-thumb"></span></span>
      <span class="toggle-label">{$settings.darkMode ? "Dark mode" : "Light mode"}</span>
    </label>
  </div>

  {#if svgAssoc !== null || xmlAssoc !== null}
    <div class="setting-group">
      <h2 class="group-title">Default viewer (Windows)</h2>
      <p class="group-sub">Open these file types with Watermelon Vector Viewer.</p>

      <label class="toggle">
        <input type="checkbox" checked={svgAssoc} on:change={() => toggleAssoc("svg")} />
        <span class="toggle-track"><span class="toggle-thumb"></span></span>
        <span class="toggle-label">SVG files</span>
      </label>

      <label class="toggle" style="margin-top:12px">
        <input type="checkbox" checked={xmlAssoc} on:change={() => toggleAssoc("xml")} />
        <span class="toggle-track"><span class="toggle-thumb"></span></span>
        <span class="toggle-label">XML files (VectorDrawable)</span>
      </label>

      {#if assocError}
        <p class="assoc-error">{assocError}</p>
      {/if}
    </div>
  {/if}

  <div class="setting-group">
    <WatermelonButton on:click={() => settings.reset()} label="Reset to defaults" variant="outline" />
  </div>
</section>

<style>
  .settings { max-width: 560px; }
  .page-title { font-size: 20px; font-weight: 700; color: var(--text-main); margin-bottom: 28px; }

  .setting-group { margin-bottom: 30px; }
  .group-title { font-size: 15px; font-weight: 600; color: var(--text-main); margin-bottom: 4px; }
  .group-sub { font-size: 13px; color: var(--text-sub); margin-bottom: 14px; }

  .chips { display: flex; gap: 8px; flex-wrap: wrap; }
  .chip {
    padding: 7px 16px;
    border-radius: 50px;
    border: 1.5px solid var(--border);
    background: var(--card-bg);
    color: var(--text-main);
    font-size: 13px;
    font-weight: 500;
    transition: all .15s;
  }
  .chip.active { background: var(--fresh-teal); color: #fff; border-color: var(--fresh-teal); }
  .chip:not(.active):hover { border-color: var(--fresh-teal); color: var(--fresh-teal); }

  .toggle { display: flex; align-items: center; gap: 12px; cursor: pointer; }
  .toggle input { display: none; }
  .toggle-track {
    width: 44px; height: 24px; border-radius: 50px;
    background: var(--border); position: relative; transition: background .2s;
  }
  .toggle input:checked + .toggle-track { background: var(--fresh-teal); }
  .toggle-thumb {
    position: absolute; top: 3px; left: 3px;
    width: 18px; height: 18px; border-radius: 50%;
    background: #fff; transition: transform .2s; box-shadow: 0 1px 3px rgba(0,0,0,.25);
  }
  .toggle input:checked + .toggle-track .toggle-thumb { transform: translateX(20px); }
  .toggle-label { font-size: 14px; color: var(--text-main); font-weight: 500; }
  .assoc-error { color: #e63946; font-size: 12px; margin-top: 10px; }
</style>
