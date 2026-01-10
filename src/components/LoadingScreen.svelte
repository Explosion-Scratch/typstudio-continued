<script lang="ts">
  import { CircleNotch } from "$lib/icons";
  import { fade } from "svelte/transition";
</script>

<div class="loading-screen" transition:fade={{ duration: 200 }}>
  <div class="loading-content">
    <CircleNotch size={32} class="spinner" weight="bold" />
    <span class="loading-text">Starting Typstudio...</span>
  </div>
</div>

<style>
  .loading-screen {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-primary);
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
</style>
