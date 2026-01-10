<script lang="ts">
  import { shell, type OutlineItem } from "$lib/stores";
  import { TextH, ImageIcon, Table, List, CaretRight, CaretDown, MagnifyingGlass } from "$lib/icons";
  import { appWindow } from "@tauri-apps/api/window";

  export let outline: OutlineItem[] = [];

  let searchQuery = "";
  let collapsedLevels: Set<number> = new Set();

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
    appWindow.emit("jump_to_position", { line, column: 1 });
  };

  const toggleLevel = (level: number) => {
    if (collapsedLevels.has(level)) {
      collapsedLevels.delete(level);
    } else {
      collapsedLevels.add(level);
    }
    collapsedLevels = collapsedLevels;
  };

  $: filteredOutline = searchQuery.trim()
    ? outline.filter(item =>
        item.title.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : outline;

  $: visibleOutline = filteredOutline.filter((item, index, arr) => {
    for (let i = index - 1; i >= 0; i--) {
      if (arr[i].level < item.level && collapsedLevels.has(arr[i].level)) {
        return false;
      }
    }
    return true;
  });

  const hasChildren = (index: number, items: OutlineItem[]) => {
    const item = items[index];
    for (let i = index + 1; i < items.length; i++) {
      if (items[i].level <= item.level) break;
      if (items[i].level > item.level) return true;
    }
    return false;
  };
</script>

<div class="document-outline">
  <div class="search-bar">
    <svelte:component this={MagnifyingGlass} size={14} class="search-icon" />
    <input
      type="text"
      class="search-input"
      placeholder="Search outline..."
      bind:value={searchQuery}
    />
  </div>

  {#if !$shell.selectedFile}
    <div class="empty-state">
      <span class="empty-text">No file open</span>
    </div>
  {:else if outline.length === 0}
    <div class="empty-state">
      <span class="empty-text">No outline available</span>
      <span class="empty-hint">Add headings with = Title</span>
    </div>
  {:else if filteredOutline.length === 0}
    <div class="empty-state">
      <span class="empty-text">No matches found</span>
    </div>
  {:else}
    <div class="outline-list">
      {#each visibleOutline as item, i}
        <button
          class="outline-item"
          style="padding-left: {12 + (item.level - 1) * 12}px"
          on:click={() => handleItemClick(item.line)}
        >
          {#if item.type === "heading" && hasChildren(filteredOutline.indexOf(item), filteredOutline)}
            <button
              class="collapse-toggle"
              on:click|stopPropagation={() => toggleLevel(item.level)}
            >
              <svelte:component
                this={collapsedLevels.has(item.level) ? CaretRight : CaretDown}
                size={12}
              />
            </button>
          {:else}
            <span class="collapse-placeholder"></span>
          {/if}
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
    display: flex;
    flex-direction: column;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-border);
  }

  .search-bar :global(.search-icon) {
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    font-size: 12px;
    padding: var(--space-xs) 0;
    background: transparent;
    border: none;
    color: var(--color-text-primary);
  }

  .search-input::placeholder {
    color: var(--color-text-placeholder);
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
    flex: 1;
    overflow-y: auto;
  }

  .outline-item {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
    text-align: left;
    min-height: 28px;
  }

  .outline-item:hover {
    background: var(--color-bg-hover);
  }

  .collapse-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: var(--radius-sm);
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }

  .collapse-toggle:hover {
    background: var(--color-bg-active);
  }

  .collapse-placeholder {
    width: 16px;
    flex-shrink: 0;
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

