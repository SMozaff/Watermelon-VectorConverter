<script>
  import { invoke } from "@tauri-apps/api/core";
  import { settings } from "../lib/settings";

  let visible = !$settings.hasSeenAssocNotice;
  let busy = false;

  async function enable(ext) {
    busy = true;
    try {
      await invoke("set_file_association", { ext, enabled: true });
    } catch {
      // Non-Windows or failure — silently ignore, user can retry in Settings.
    }
    busy = false;
  }

  function dismiss() {
    settings.update((s) => ({ ...s, hasSeenAssocNotice: true }));
    visible = false;
  }

  async function enableAndDismiss(ext) {
    await enable(ext);
    dismiss();
  }
</script>

{#if visible}
  <div class="overlay">
    <div class="modal">
      <h2 class="title">Set as default viewer?</h2>
      <p class="body">
        Watermelon Vector Viewer can open SVG and VectorDrawable XML files
        directly. You can change this anytime in Settings.
      </p>
      <div class="actions">
        <button class="btn primary" disabled={busy} on:click={() => enableAndDismiss("svg")}>
          Set as default for SVG
        </button>
        <button class="btn outline" disabled={busy} on:click={() => enableAndDismiss("xml")}>
          Set as default for XML
        </button>
        <button class="btn text" disabled={busy} on:click={dismiss}>
          Not now
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(0,0,0,.5);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
  }
  .modal {
    background: var(--card-bg, #fff);
    color: var(--text-main, #1D3557);
    border-radius: 12px;
    padding: 24px;
    max-width: 380px;
    box-shadow: 0 8px 30px rgba(0,0,0,.25);
  }
  .title { font-size: 18px; font-weight: 700; margin-bottom: 10px; }
  .body { font-size: 14px; line-height: 1.5; margin-bottom: 20px; opacity: .85; }
  .actions { display: flex; flex-direction: column; gap: 8px; }
  .btn {
    padding: 10px 14px; border-radius: 8px; border: none;
    font-size: 14px; font-weight: 600; cursor: pointer;
  }
  .btn:disabled { opacity: .6; cursor: default; }
  .btn.primary { background: #2A9D8F; color: #fff; }
  .btn.outline { background: transparent; border: 1px solid #2A9D8F; color: #2A9D8F; }
  .btn.text { background: transparent; color: var(--text-sub, #6b7280); }
</style>
