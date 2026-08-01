<script>
  export let label = "Working…";
  export let indeterminate = false;
  export let value = 0;
</script>

<div class="progress-wrap">
  <p class="progress-label">{label}</p>
  <div class="track">
    {#if indeterminate}
      <div class="bar indeterminate"></div>
    {:else}
      <div class="bar" style="width: {value}%"></div>
    {/if}
  </div>
</div>

<style>
  .progress-wrap { padding: 28px 0; }
  .progress-label { color: var(--text-sub); font-size: 13px; margin-bottom: 12px; }
  .track { height: 5px; background: var(--border); border-radius: 99px; overflow: hidden; }
  .bar { height: 100%; background: var(--fresh-teal); border-radius: 99px; transition: width .3s; }
  .bar.indeterminate {
    width: 40%;
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    0%   { transform: translateX(-100%); }
    100% { transform: translateX(350%); }
  }
</style>
