<script>
  import { createEventDispatcher } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  const dispatch = createEventDispatcher();
  /** "svg" | "xml" — which loose-file extension this zone accepts, plus ZIPs. */
  export let accept = "svg";

  $: extWord = accept === "xml" ? "XML" : "SVG";
  let choosing = false;

  function isZip(name) { return name.toLowerCase().endsWith(".zip"); }
  function matchesOwnExt(name) { return name.toLowerCase().endsWith(`.${accept}`); }

  /**
   * The Rust backend displays the native dialog, validates the selected files,
   * and returns bounded byte buffers. Paths never cross the IPC boundary.
   */
  async function chooseAndDispatch() {
    if (choosing) return;
    choosing = true;
    try {
      const picked = await invoke("pick_input_files", { kind: accept });
      const zips = picked.filter((file) => isZip(file.name));
      const loose = picked.filter((file) => !isZip(file.name) && matchesOwnExt(file.name));

      for (const file of zips) {
        dispatch("batch-zip", { bytes: new Uint8Array(file.bytes), name: file.name });
      }
      if (loose.length === 1) {
        const file = loose[0];
        dispatch("single-file", { bytes: new Uint8Array(file.bytes), name: file.name });
      } else if (loose.length > 1) {
        dispatch("batch-loose", {
          files: loose.map((file) => ({ bytes: new Uint8Array(file.bytes), name: file.name })),
        });
      }
    } catch (error) {
      dispatch("error", { message: error?.message ?? String(error) });
    } finally {
      choosing = false;
    }
  }
</script>

<div
  class="dropzone"
  class:choosing
  on:click={chooseAndDispatch}
  role="button"
  tabindex="0"
  on:keydown={(event) => event.key === "Enter" && chooseAndDispatch()}
  aria-label="Choose files to convert"
>
  <span class="drop-icon">📂</span>
  <p class="drop-label">Click to choose {extWord} file(s) or ZIP(s)</p>
  <p class="drop-hint">Native file selection keeps filesystem paths outside the webview</p>
</div>

<style>
  .dropzone {
    border: 2px dashed var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    padding: 32px 20px;
    text-align: center;
    margin-bottom: 14px;
    transition: border-color .2s, background .2s;
    cursor: pointer;
  }

  .dropzone:hover,
  .dropzone.choosing {
    border-color: var(--fresh-teal);
    background: color-mix(in srgb, var(--fresh-teal) 8%, var(--surface));
  }

  .drop-icon { font-size: 32px; display: block; margin-bottom: 10px; }
  .drop-label { font-size: 14px; font-weight: 600; color: var(--text-main); }
  .drop-hint { font-size: 11px; color: var(--text-sub); margin-top: 4px; }
</style>
