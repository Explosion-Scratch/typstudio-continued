<script lang="ts" context="module">
  export interface ContextMenuItem {
    label: string;
    icon?: any;
    action: () => void;
    disabled?: boolean;
    divider?: boolean;
  }
</script>

<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";

  export let items: ContextMenuItem[] = [];
  export let x = 0;
  export let y = 0;

  const dispatch = createEventDispatcher<{ close: void }>();

  let menuEl: HTMLDivElement;

  const handleClickOutside = (event: MouseEvent) => {
    if (menuEl && !menuEl.contains(event.target as Node)) {
      dispatch("close");
    }
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      dispatch("close");
    }
  };

  const handleItemClick = (item: ContextMenuItem) => {
    if (!item.disabled) {
      item.action();
      dispatch("close");
    }
  };

  onMount(() => {
    document.addEventListener("click", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);

    const rect = menuEl.getBoundingClientRect();
    if (rect.right > window.innerWidth) {
      x = window.innerWidth - rect.width - 8;
    }
    if (rect.bottom > window.innerHeight) {
      y = window.innerHeight - rect.height - 8;
    }

    return () => {
      document.removeEventListener("click", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  });
</script>

<div
  bind:this={menuEl}
  class="context-menu"
  style="left: {x}px; top: {y}px"
  role="menu"
>
  {#each items as item}
    {#if item.divider}
      <div class="divider"></div>
    {:else}
      <button
        class="menu-item"
        class:disabled={item.disabled}
        on:click={() => handleItemClick(item)}
        role="menuitem"
        disabled={item.disabled}
      >
        {#if item.icon}
          <svelte:component this={item.icon} size={14} />
        {/if}
        <span>{item.label}</span>
      </button>
    {/if}
  {/each}
</div>

<style>
  .context-menu {
    position: fixed;
    z-index: 1000;
    min-width: 160px;
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    padding: var(--space-xs);
    animation: fadeScaleIn var(--transition-fast) ease;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--color-text-primary);
    text-align: left;
    transition: background var(--transition-fast);
  }

  .menu-item:hover:not(.disabled) {
    background: var(--color-bg-hover);
  }

  .menu-item.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .divider {
    height: 1px;
    background: var(--color-border);
    margin: var(--space-xs) 0;
  }

  @keyframes fadeScaleIn {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
