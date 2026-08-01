<script>
  import "../app.css";
  import Nav from "../components/Nav.svelte";
  import AssocNoticeModal from "../components/AssocNoticeModal.svelte";
  import { settings } from "../lib/settings";
</script>

<div class="shell" class:dark={$settings.darkMode}>
  <Nav />
  <div class="main-wrap">
    <!-- Top banner: replace src with your PNG asset path -->
    <div class="top-banner">
      <img src="/banner.png" alt="" class="banner-img" onerror="this.style.display='none'" />
      <span class="banner-title">Watermelon Vector Converter</span>
    </div>
    <main class="content">
      <slot />
    </main>
  </div>
  <AssocNoticeModal />
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
    background: var(--off-white);
  }

  .main-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Narrow top banner — 48px tall. Put your PNG there. */
  .top-banner {
    height: 48px;
    min-height: 48px;
    background: var(--deep-navy);
    display: flex;
    align-items: center;
    padding: 0 20px;
    gap: 12px;
    overflow: hidden;
    position: relative;
  }

  /* When banner.png loads it fills the strip */
  .banner-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: left center;
    opacity: 0.35;
    pointer-events: none;
  }

  .banner-title {
    position: relative;
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: .04em;
    text-transform: uppercase;
    opacity: 0.9;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 28px;
    background: var(--off-white);
    color: var(--text-main);
  }

  /* Dark mode — banner stays dark navy regardless */
  :global(.dark) .top-banner {
    background: #0D1117;
  }
</style>
