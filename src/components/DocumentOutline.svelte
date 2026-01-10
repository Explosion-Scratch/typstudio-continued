<script lang="ts">
  import { shell, type OutlineItem } from "$lib/stores";
  import { TextH, ImageIcon, Table, List } from "$lib/icons";
  import { appWindow } from "@tauri-apps/api/window";

  export let outline: OutlineItem[] = [];

  const getIcon = (type: OutlineItem["type"]) => {
    switch (type) {
      case "heading":
        return TextH;
      case "figure":
        return ImageIcon;
      case "table":
        return Table;
      case "list":
        return List;
      default:
        return TextH;
    }
  };

  const handleItemClick = (line: number) => {
    appWindow.emit("jump_to_line", { line });
  };
</script>

<div class="document-outline">
  {#if !$shell.selectedFile}
    <div class="empty-state">
      <span class="empty-text">No file open</span>
    </div>
  {:else if outline.length === 0}
    <div class="empty-state">
      <span class="empty-text">No outline available</span>
      <span class="empty-hint">Add headings with = Title</span>
    </div>
  {:else}
    <div class="outline-list">
      {#each outline as item}
        <button
          class="outline-item"
          style="padding-left: {12 + (item.level - 1) * 12}px"
          on:click={() => handleItemClick(item.line)}
        >
          <svelte:component
            this={getIcon(item.type)}
            size={14}
            weight="duotone"
            class="item-icon"
          />
          <span class="item-title">{item.title}</span>
          <span class="item-line">{item.line}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .document-outline {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 120px;
    gap: var(--space-xs);
  }

  .empty-text {
    color: var(--color-text-tertiary);
    font-size: 13px;
  }

  .empty-hint {
    color: var(--color-text-placeholder);
    font-size: 11px;
  }

  .outline-list {
    display: flex;
    flex-direction: column;
    padding: var(--space-sm);
  }

  .outline-item {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
    text-align: left;
    min-height: 28px;
  }

  .outline-item:hover {
    background: var(--color-bg-hover);
  }

  .outline-item :global(.item-icon) {
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }

  .item-title {
    flex: 1;
    font-size: 13px;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item-line {
    font-size: 11px;
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }
</style>
