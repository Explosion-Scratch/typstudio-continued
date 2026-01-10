<script lang="ts">
  import ExplorerTree from "./ExplorerTree.svelte";
  import DocumentOutline from "./DocumentOutline.svelte";
  import PackagesPanel from "./PackagesPanel.svelte";
  import { FolderDuotone, ListBullets, Package } from "$lib/icons";
  import { shell } from "$lib/stores";

  const tabs = [
    { id: "files" as const, icon: FolderDuotone, label: "Files" },
    { id: "outline" as const, icon: ListBullets, label: "Outline" },
    { id: "packages" as const, icon: Package, label: "Packages" },
  ];
</script>

<div class="side-panel">
  <div class="tab-bar">
    {#each tabs as tab}
      <button
        class="tab-button"
        class:active={$shell.activeSidebarTab === tab.id}
        on:click={() => shell.setSidebarTab(tab.id)}
        title={tab.label}
      >
        <svelte:component
          this={tab.icon}
          size={18}
          weight={$shell.activeSidebarTab === tab.id ? "duotone" : "regular"}
        />
      </button>
    {/each}
  </div>

  <div class="panel-content">
    {#if $shell.activeSidebarTab === "files"}
      <ExplorerTree />
    {:else if $shell.activeSidebarTab === "outline"}
      <DocumentOutline outline={$shell.documentOutline} />
    {:else if $shell.activeSidebarTab === "packages"}
      <PackagesPanel />
    {/if}
  </div>
</div>

<style>
  .side-panel {
    display: flex;
    border-right: 1px solid var(--color-border);
    background: var(--color-bg-primary);
    height: 100%;
  }

  .tab-bar {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding: var(--space-sm);
    border-right: 1px solid var(--color-border);
    background: var(--color-bg-secondary);
  }

  .tab-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    color: var(--color-text-tertiary);
    transition: all var(--transition-fast);
  }

  .tab-button:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-secondary);
  }

  .tab-button.active {
    background: var(--color-bg-active);
    color: var(--color-text-primary);
  }

  .panel-content {
    display: flex;
    flex-direction: column;
    width: 240px;
    min-width: 200px;
    overflow: hidden;
  }
</style>
