<script lang="ts">
  import type { FileItem, FileType, FSRefreshEvent } from "../lib/ipc";
  import { project, shell } from "../lib/stores";
  import { listDir, deleteFile, renameFile } from "../lib/ipc";
  import { onMount } from "svelte";
  import {
    CaretRight,
    CaretDown,
    FileDuotone,
    FolderDuotone,
    FolderOpenDuotone,
    Trash,
    Pencil,
  } from "$lib/icons";
  import { appWindow } from "@tauri-apps/api/window";
  import ContextMenu, { type ContextMenuItem } from "./ContextMenu.svelte";

  export let type: FileType;
  export let path: string;

  let expanded = path === "/";
  let files: FileItem[] = [];
  let contextMenu: { x: number; y: number } | null = null;

  const handleClick = () => {
    if (type === "directory") {
      expanded = !expanded;
    } else {
      shell.selectFile(path);
    }
  };

  const update = async () => {
    files = await listDir(path);
  };

  const handleContextMenu = (event: MouseEvent) => {
    if (path === "/") return;
    event.preventDefault();
    event.stopPropagation();
    contextMenu = { x: event.clientX, y: event.clientY };
  };

  const handleDelete = async () => {
    if (!confirm(`Delete "${fileName}"?`)) return;
    try {
      await deleteFile(path);
      const parentPath = path.substring(0, path.lastIndexOf("/")) || "/";
      appWindow.emit("fs_refresh", { path: parentPath.substring(1) });
      if ($shell.selectedFile === path) {
        shell.selectFile(undefined);
      }
    } catch (e) {
      console.error("Failed to delete file:", e);
    }
  };

  const handleRename = async () => {
    const newName = prompt("Enter new name:", fileName);
    if (!newName || newName === fileName) return;
    try {
      const parentPath = path.substring(0, path.lastIndexOf("/")) || "/";
      const newPath = parentPath === "/" ? `/${newName}` : `${parentPath}/${newName}`;
      await renameFile(path, newPath);
      appWindow.emit("fs_refresh", { path: parentPath.substring(1) });
      if ($shell.selectedFile === path) {
        shell.selectFile(newPath);
      }
    } catch (e) {
      console.error("Failed to rename file:", e);
    }
  };

  const getContextMenuItems = (): ContextMenuItem[] => [
    {
      label: "Rename",
      icon: Pencil,
      action: handleRename,
    },
    { label: "", action: () => {}, divider: true },
    {
      label: "Delete",
      icon: Trash,
      action: handleDelete,
    },
  ];

  onMount(() => {
    appWindow.listen<FSRefreshEvent>("fs_refresh", ({ payload }) => {
      if (`/${payload.path}` === path) update();
    });
  });

  $: {
    if (expanded) {
      update();
    }
  }

  if (path === "/") {
    onMount(() => project.subscribe(update));
  }

  $: isSelected = $shell.selectedFile === path;
  $: fileName = path === "/" ? "root" : path.slice(path.lastIndexOf("/") + 1);
  $: depth = path.split("/").length - 2;
</script>

{#if contextMenu}
  <ContextMenu
    items={getContextMenuItems()}
    x={contextMenu.x}
    y={contextMenu.y}
    on:close={() => contextMenu = null}
  />
{/if}

{#if path !== "/"}
  <button
    class="explorer-node"
    class:selected={isSelected}
    style="padding-left: {8 + depth * 16}px"
    on:click={handleClick}
    on:contextmenu={handleContextMenu}
  >
    {#if type === "directory"}
      <span class="caret">
        <svelte:component
          this={expanded ? CaretDown : CaretRight}
          size={12}
          weight="bold"
        />
      </span>
      <svelte:component
        this={expanded ? FolderOpenDuotone : FolderDuotone}
        size={16}
        weight="duotone"
        class="node-icon folder"
      />
    {:else}
      <span class="caret-placeholder"></span>
      <FileDuotone size={16} weight="duotone" class="node-icon file" />
    {/if}
    <span class="node-label">{fileName}</span>
  </button>
{/if}

{#if expanded}
  {#each files as file}
    <svelte:self
      type={file.type}
      path={path === "/" ? `${path}${file.name}` : `${path}/${file.name}`}
    />
  {/each}
{/if}

<style>
  .explorer-node {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    width: 100%;
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    text-align: left;
    transition: background var(--transition-fast);
    min-height: 26px;
  }

  .explorer-node:hover {
    background: var(--color-bg-hover);
  }

  .explorer-node.selected {
    background: var(--color-bg-selected);
  }

  .caret {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }

  .caret-placeholder {
    width: 14px;
    flex-shrink: 0;
  }

  .explorer-node :global(.node-icon) {
    flex-shrink: 0;
  }

  .explorer-node :global(.node-icon.folder) {
    color: #e8a951;
  }

  .explorer-node :global(.node-icon.file) {
    color: var(--color-text-tertiary);
  }

  .node-label {
    flex: 1;
    font-size: 13px;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
