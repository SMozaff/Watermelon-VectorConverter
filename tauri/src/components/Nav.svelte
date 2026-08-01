<script>
  import { page } from "$app/stores";
</script>

<nav class="sidebar">
  <ul class="nav-links">
    <li class:active={$page.url.pathname === "/"}>
      <a href="/">
        <span class="icon">⚡</span>
        <span>Convert</span>
      </a>
    </li>
    <li class:active={$page.url.pathname.startsWith("/settings")}>
      <a href="/settings">
        <span class="icon">⚙️</span>
        <span>Settings</span>
      </a>
    </li>
    <li class:active={$page.url.pathname.startsWith("/about")}>
      <a href="/about">
        <span class="icon">ℹ️</span>
        <span>About</span>
      </a>
    </li>
  </ul>

  <div class="sidebar-footer">
    <span class="version">v0.1.0</span>
    <span class="studio">Digital Raven</span>
  </div>
</nav>

<style>
  .sidebar {
    width: 180px;
    min-width: 180px;
    background: var(--deep-navy);
    display: flex;
    flex-direction: column;
    padding-top: 8px; /* aligns with banner height */
    height: 100vh;
  }

  .nav-links { list-style: none; flex: 1; padding-top: 8px; }

  .nav-links li a {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 20px;
    color: rgba(255,255,255,.6);
    text-decoration: none;
    font-size: 14px;
    font-weight: 500;
    transition: background .15s, color .15s;
    border-radius: 6px;
    margin: 2px 8px;
  }

  .nav-links li.active a,
  .nav-links li a:hover {
    background: rgba(255,255,255,.1);
    color: #fff;
  }

  .nav-links li.active a { color: #5CE0D0; }
  .icon { font-size: 15px; width: 20px; text-align: center; }

  .sidebar-footer {
    padding: 14px 20px;
    border-top: 1px solid rgba(255,255,255,.1);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .version { color: rgba(255,255,255,.3); font-size: 11px; }
  .studio  { color: rgba(255,255,255,.4); font-size: 12px; font-weight: 500; }
</style>
