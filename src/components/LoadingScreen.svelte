<script lang="ts">
  import { CircleNotch } from "$lib/icons";
  import { fade } from "svelte/transition";

  export let stage: string = "Starting Typstudio...";
  export let progress: number = 0;
</script>

<div class="loading-screen bg-vibrant" transition:fade={{ duration: 200 }}>
  <div class="loading-content">
    <CircleNotch size={32} class="spinner" weight="bold" />
    <span class="loading-text">{stage}</span>
    {#if progress > 0}
      <div class="progress-bar">
        <div class="progress-fill" style="width: {progress}%"></div>
      </div>
    {/if}
  </div>
</div>

<style>
  .loading-screen {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .loading-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-lg);
  }

  .loading-content :global(.spinner) {
    color: var(--color-text-tertiary);
    animation: spin 1s linear infinite;
    display: block;
    transform-origin: center;
  }

  .loading-text {
    color: var(--color-text-tertiary);
    font-size: 14px;
    font-weight: 500;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .progress-bar {
    width: 200px;
    height: 4px;
    background: var(--color-bg-hover);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-accent);
    border-radius: 2px;
    transition: width 0.3s ease;
  }
</style>
