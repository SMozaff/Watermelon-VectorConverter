<script>
  import { createEventDispatcher, onMount, onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { readFile } from "@tauri-apps/plugin-fs";
  import { getCurrentWebview } from "@tauri-apps/api/webview";

  const dispatch = createEventDispatcher();
  /** "svg" | "xml" — which non-zip extension this zone accepts, plus zips of that type. */
  export let accept = "svg";

  $: extWord = accept === "xml" ? "XML" : "SVG";

  let dragOver = false;
  let unlisten = null;
  // Tauri's native onDragDropEvent has a known bug where a single drop can
  // fire twice with different event IDs (tauri-apps/tauri#14134). Guard
  // against acting on the same drop twice in quick succession.
  let lastHandledAt = 0;

  function isZip(path) { return path.toLowerCase().endsWith(".zip"); }
  function matchesOwnExt(path) { return path.toLowerCase().endsWith(`.${accept}`); }

  /**
   * Classify a batch of dropped/picked paths and dispatch the right event(s):
   * - each .zip fires its own "batch-zip" event (independent output per your spec)
   * - all matching loose files together fire one "batch-loose" (2+) or
   *   "single-file" (exactly 1) event
   * - files matching neither this zone's extension nor .zip are ignored
   */
  async function classifyAndDispatch(paths) {
    const zips = paths.filter(isZip);
    const loose = paths.filter((p) => !isZip(p) && matchesOwnExt(p));

    for (const zipPath of zips) {
      try {
        const bytes = await readFile(zipPath);
        dispatch("batch-zip", { bytes, name: zipPath.split(/[\\/]/).pop() });
      } catch (e) {
        dispatch("error", { message: e?.message ?? String(e) });
      }
    }

    if (loose.length === 1) {
      try {
        const bytes = await readFile(loose[0]);
        dispatch("single-file", { bytes, name: loose[0].split(/[\\/]/).pop() });
      } catch (e) {
        dispatch("error", { message: e?.message ?? String(e) });
      }
    } else if (loose.length > 1) {
      try {
        const files = await Promise.all(
          loose.map(async (p) => ({ bytes: await readFile(p), name: p.split(/[\\/]/).pop() }))
        );
        dispatch("batch-loose", { files });
      } catch (e) {
        dispatch("error", { message: e?.message ?? String(e) });
      }
    }
  }

  onMount(async () => {
    // Native OS file drag-and-drop: Tauri v2's webview does not deliver these
    // through the DOM's drop event (dragDropEnabled defaults to true, which
    // intercepts it before the page sees it) — must use this API instead.
    unlisten = await getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === "over") {
        dragOver = true;
      } else if (p.type === "drop") {
        dragOver = false;
        const now = Date.now();
        if (now - lastHandledAt < 300) return; // de-dupe rapid repeat events
        lastHandledAt = now;
        if (p.paths?.length) classifyAndDispatch(p.paths);
      } else {
        dragOver = false; // "leave" / cancelled
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function onClick() {
    const paths = await open({
      multiple: true,
      filters: [{ name: `${extWord} or ZIP`, extensions: [accept, "zip"] }],
    });
    if (!paths) return;
    const list = Array.isArray(paths) ? paths : [paths];
    await classifyAndDispatch(list);
  }
</script>

<div
  class="dropzone"
  class:drag-over={dragOver}
  on:click={onClick}
  role="button"
  tabindex="0"
  on:keydown={(e) => e.key === "Enter" && onClick()}
  aria-label="Drop or click to choose files"
>
  <span class="drop-icon">📂</span>
  <p class="drop-label">Drop {extWord} file(s) or ZIP(s) here, or click to browse</p>
  <p class="drop-hint">One file converts instantly · multiple files or a ZIP run as a batch</p>
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
  .dropzone.drag-over {
    border-color: var(--fresh-teal);
    background: color-mix(in srgb, var(--fresh-teal) 8%, var(--surface));
  }

  .drop-icon { font-size: 32px; display: block; margin-bottom: 10px; }
  .drop-label { font-size: 14px; font-weight: 600; color: var(--text-main); }
  .drop-hint { font-size: 11px; color: var(--text-sub); margin-top: 4px; }
</style>
