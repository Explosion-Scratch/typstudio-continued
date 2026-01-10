<script lang="ts">
  import { shell, type OutlineItem } from "$lib/stores";
  import { TextH, ImageIcon, Table, List, CaretRight, CaretDown, MagnifyingGlass } from "$lib/icons";
  import { appWindow } from "@tauri-apps/api/window";

  export let outline: OutlineItem[] = [];

  let searchQuery = "";
  let collapsedItems: Set<string> = new Set();

  const getItemId = (item: OutlineItem) => `${item.line}-${item.level}-${item.title}`;

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

  const toggleItem = (item: OutlineItem) => {
    const id = getItemId(item);
    if (collapsedItems.has(id)) {
      collapsedItems.delete(id);
    } else {
      collapsedItems.add(id);
    }
    collapsedItems = collapsedItems;
  };

  const isItemCollapsed = (item: OutlineItem) => collapsedItems.has(getItemId(item));

  const getParentHeadings = (index: number, items: OutlineItem[]): OutlineItem[] => {
    const item = items[index];
    const parents: OutlineItem[] = [];
    for (let i = index - 1; i >= 0; i--) {
      if (items[i].level < item.level) {
        parents.unshift(items[i]);
        if (items[i].level === 1) break;
      }
    }
    return parents;
  };

  $: matchedItems = searchQuery.trim()
    ? outline.filter(item =>
        item.title.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : [];

  $: displayItems = (() => {
    if (!searchQuery.trim()) {
      const visible: { item: OutlineItem; isParent: boolean }[] = [];
      for (let i = 0; i < outline.length; i++) {
        const item = outline[i];
        let hidden = false;
        for (let j = i - 1; j >= 0; j--) {
          if (outline[j].level < item.level && isItemCollapsed(outline[j])) {
            hidden = true;
            break;
          }
          if (outline[j].level < item.level) break;
        }
        if (!hidden) {
          visible.push({ item, isParent: false });
        }
      }
      return visible;
    }

    const result: { item: OutlineItem; isParent: boolean }[] = [];
    const addedIds = new Set<string>();

    for (const matched of matchedItems) {
      const idx = outline.indexOf(matched);
      const parents = getParentHeadings(idx, outline);
      
      for (const parent of parents) {
        const id = getItemId(parent);
        if (!addedIds.has(id)) {
          addedIds.add(id);
          result.push({ item: parent, isParent: true });
        }
      }
      
      const matchedId = getItemId(matched);
      if (!addedIds.has(matchedId)) {
        addedIds.add(matchedId);
        result.push({ item: matched, isParent: false });
      }
    }
    
    return result.sort((a, b) => a.item.line - b.item.line);
  })();

  const hasChildren = (item: OutlineItem, items: OutlineItem[]) => {
    const idx = items.indexOf(item);
    for (let i = idx + 1; i < items.length; i++) {
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
  {:else if displayItems.length === 0}
    <div class="empty-state">
      <span class="empty-text">No matches found</span>
    </div>
  {:else}
    <div class="outline-list">
      {#each displayItems as { item, isParent }}
        <button
          class="outline-item"
          class:parent-item={isParent}
          style="padding-left: {12 + (item.level - 1) * 12}px"
          on:click={() => handleItemClick(item.line)}
        >
          {#if item.type === "heading" && hasChildren(item, outline) && !searchQuery.trim()}
            <span
              class="collapse-toggle"
              role="button"
              tabindex="0"
              on:click|stopPropagation={() => toggleItem(item)}
              on:keydown|stopPropagation={(e) => e.key === 'Enter' && toggleItem(item)}
            >
              <svelte:component
                this={isItemCollapsed(item) ? CaretRight : CaretDown}
                size={12}
              />
            </span>
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

  .outline-item.parent-item {
    opacity: 0.6;
  }

  .outline-item.parent-item .item-title {
    color: var(--color-text-tertiary);
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
