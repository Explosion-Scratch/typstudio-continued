<script lang="ts">
  import ExplorerNode from "./ExplorerNode.svelte";
  import { project, shell } from "$lib/stores";
  import { createFile } from "$lib/ipc";
  import { Plus, ArrowClockwise } from "$lib/icons";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  const appWindow = getCurrentWindow();
</script>

<div class="explorer-tree">
  {#if $project}
    <div class="tree-header">
      <span class="tree-title">Files</span>
      <button
        class="icon-button"
        on:click={() => {
          appWindow.emit("fs_refresh", { path: "" });
        }}
        title="Refresh"
      >
        <ArrowClockwise size={14} weight="bold" />
      </button>
      <button
        class="icon-button"
        on:click={() => {
          shell.createModal({
            type: "input",
            title: "Create file",
            placeholder: "filename.typ",
            callback: (path) => {
              if (path) createFile(path);
            },
          });
        }}
        title="New file"
      >
        <Plus size={14} weight="bold" />
      </button>
    </div>
    <div class="tree-content">
      <ExplorerNode type="directory" path="/" />
    </div>
  {/if}
</div>

<style>
  .explorer-tree {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    user-select: none;
  }

  .tree-header {
    display: flex;
    align-items: center;
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-border);
  }

  .tree-title {
    flex: 1;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .tree-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-sm);
  }
</style>
