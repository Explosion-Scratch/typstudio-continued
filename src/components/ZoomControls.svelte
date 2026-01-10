<script lang="ts">
  export let zoom: number;
  export let onZoomIn: () => void;
  export let onZoomOut: () => void;
  export let onZoomReset: () => void;

  $: zoomPercent = Math.round(zoom * 100);
</script>

<div class="zoom-controls">
  <button
    class="zoom-button"
    on:click={onZoomOut}
    title="Zoom out (Cmd+-)"
    aria-label="Zoom out"
  >
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  </button>
  <button
    class="zoom-percent"
    on:click={onZoomReset}
    title="Reset to 100% (Cmd+0)"
    aria-label="Reset zoom"
  >
    {zoomPercent}%
  </button>
  <button
    class="zoom-button"
    on:click={onZoomIn}
    title="Zoom in (Cmd++)"
    aria-label="Zoom in"
  >
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  </button>
</div>

<style>
  .zoom-controls {
    position: fixed;
    bottom: var(--space-lg);
    right: var(--space-lg);
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    padding: 2px;
    opacity: 0.7;
    transition: opacity var(--transition-normal);
    z-index: 100;
  }

  .zoom-controls:hover {
    opacity: 1;
  }

  .zoom-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    transition: all var(--transition-fast);
  }

  .zoom-button:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .zoom-button:active {
    background: var(--color-bg-active);
  }

  .zoom-percent {
    min-width: 48px;
    height: 28px;
    padding: 0 var(--space-sm);
    font-size: 12px;
    font-weight: 500;
    color: var(--color-text-secondary);
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }

  .zoom-percent:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .zoom-percent:active {
    background: var(--color-bg-active);
  }
</style>
