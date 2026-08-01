<!-- About screen -->
<script>
  import { invoke } from "@tauri-apps/api/core";

  async function openUrl(url) {
    try {
      await invoke("open_url", { url });
    } catch {
      // fallback: try window.open (may not work in Tauri but graceful)
      window.open(url, "_blank");
    }
  }
</script>

<section class="about">

  <!-- Logo strip -->
  <div class="logo-strip">
    <!-- Watermelon logo: place watermelon_gen.png in tauri/public/ -->
    <img src="/watermelon_gen.png" alt="Watermelon" class="logo-wm"
         onerror="this.style.display='none'" />
    <!-- Raven logo: place digital_raven_hq.png in tauri/public/ -->
    <img src="/digital_raven_hq.png" alt="Digital Raven" class="logo-raven"
         onerror="this.style.display='none'" />
  </div>

  <h1 class="app-name">Watermelon Vector Converter</h1>
  <p class="version-tag">v0.1.0 — Desktop</p>

  <p class="desc">
    Offline SVG → Android VectorDrawable converter.<br />
    Rust-powered. No internet required. No data leaves your device.
  </p>

  <div class="divider"></div>

  <div class="links">
    <button class="link-row" on:click={() => openUrl("https://github.com/muzaff-beep/Watermelon-VectorConverter")}>
      <span class="link-icon">⭐</span>
      <div class="link-text">
        <span class="link-label">GitHub Repository</span>
        <span class="link-sub">muzaff-beep/Watermelon-VectorConverter</span>
      </div>
    </button>

    <button class="link-row" on:click={() => openUrl("mailto:so.muzaff@gmail.com")}>
      <span class="link-icon">✉️</span>
      <div class="link-text">
        <span class="link-label">Contact</span>
        <span class="link-sub">so.muzaff@gmail.com</span>
      </div>
    </button>
  </div>

  <div class="divider"></div>

  <div class="meta">
    <span class="meta-item">Built by <strong>Suhail Muzaffari</strong></span>
    <span class="meta-dot">·</span>
    <span class="meta-item">Digital Raven Studio</span>
    <span class="meta-dot">·</span>
    <span class="meta-item">© 2026</span>
  </div>

</section>

<style>
  .about {
    max-width: 480px;
    margin: 0 auto;
    padding: 8px 0;
  }

  .logo-strip {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-bottom: 24px;
  }

  .logo-wm {
    width: 72px;
    height: 72px;
    object-fit: contain;
    border-radius: 16px;
    background: var(--surface);
    padding: 4px;
  }

  .logo-raven {
    width: 120px;
    height: 48px;
    object-fit: contain;
  }

  .app-name {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-main);
    margin-bottom: 4px;
  }

  .version-tag {
    font-size: 12px;
    color: var(--text-sub);
    margin-bottom: 16px;
    font-family: monospace;
  }

  .desc {
    font-size: 14px;
    color: var(--text-sub);
    line-height: 1.6;
    margin-bottom: 24px;
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 20px 0;
  }

  .links { display: flex; flex-direction: column; gap: 4px; }

  .link-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
    transition: background .15s, border-color .15s;
    cursor: pointer;
    width: 100%;
  }

  .link-row:hover {
    background: color-mix(in srgb, var(--fresh-teal) 8%, var(--surface));
    border-color: var(--fresh-teal);
  }

  .link-icon { font-size: 20px; flex-shrink: 0; }

  .link-text { display: flex; flex-direction: column; gap: 2px; }
  .link-label { font-size: 14px; font-weight: 600; color: var(--text-main); }
  .link-sub { font-size: 12px; color: var(--text-sub); }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .meta-item { font-size: 13px; color: var(--text-sub); }
  .meta-item strong { color: var(--text-main); }
  .meta-dot { color: var(--border); }
</style>
