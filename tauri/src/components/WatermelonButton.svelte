<script>
  import { createEventDispatcher } from "svelte";
  export let label = "Button";
  export let variant = "primary";
  export let disabled = false;
  const dispatch = createEventDispatcher();
</script>

<button
  class="wm-btn {variant}"
  {disabled}
  on:click={() => dispatch("click")}
>
  {label}
</button>

<style>
  .wm-btn {
    padding: 9px 20px;
    border-radius: 50px;
    font-size: 13px;
    font-weight: 600;
    transition: opacity .15s, box-shadow .15s;
    letter-spacing: .01em;
    white-space: nowrap;
  }

  .wm-btn:disabled { opacity: .45; cursor: not-allowed; }
  .wm-btn:not(:disabled):hover { box-shadow: 0 3px 10px rgba(0,0,0,.18); }

  .primary { background: var(--watermelon-red); color: #fff; }
  .teal    { background: var(--fresh-teal);     color: #fff; }
  .outline {
    background: transparent;
    color: var(--text-main);
    border: 1.5px solid var(--border);
  }
  .outline:hover { border-color: var(--fresh-teal); color: var(--fresh-teal); }
</style>
