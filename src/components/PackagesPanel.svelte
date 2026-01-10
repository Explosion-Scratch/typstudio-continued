<script lang="ts">
  import { Package, Trash, Plus, CircleNotch } from "$lib/icons";
  import { onMount } from "svelte";

  interface InstalledPackage {
    namespace: string;
    name: string;
    version: string;
  }

  let packages: InstalledPackage[] = [];
  let isLoading = true;
  let searchQuery = "";
  let isInstalling = false;

  const groupedPackages: Record<string, InstalledPackage[]> = {};

  $: {
    Object.keys(groupedPackages).forEach(key => delete groupedPackages[key]);
    packages.forEach(pkg => {
      if (!groupedPackages[pkg.namespace]) {
        groupedPackages[pkg.namespace] = [];
      }
      groupedPackages[pkg.namespace].push(pkg);
    });
  }

  const loadPackages = async () => {
    isLoading = true;
    try {
      packages = [
        { namespace: "preview", name: "tablex", version: "0.0.8" },
        { namespace: "preview", name: "cetz", version: "0.3.1" },
        { namespace: "preview", name: "fletcher", version: "0.5.3" },
        { namespace: "local", name: "my-template", version: "1.0.0" },
      ];
    } finally {
      isLoading = false;
    }
  };

  const handleRemovePackage = async (pkg: InstalledPackage) => {
    console.log("Remove package:", pkg);
  };

  const handleInstallPackage = async () => {
    if (!searchQuery.trim()) return;
    isInstalling = true;
    try {
      console.log("Install package:", searchQuery);
    } finally {
      isInstalling = false;
      searchQuery = "";
    }
  };

  onMount(() => {
    loadPackages();
  });
</script>

<div class="packages-panel">
  <div class="panel-header">
    <h3 class="panel-title">
      <svelte:component this={Package} size={16} />
      Packages
    </h3>
    <span class="package-count">{packages.length}</span>
  </div>

  <div class="install-bar">
    <input
      type="text"
      class="install-input"
      placeholder="Package name to install..."
      bind:value={searchQuery}
      on:keydown={(e) => e.key === "Enter" && handleInstallPackage()}
    />
    <button
      class="install-button"
      on:click={handleInstallPackage}
      disabled={isInstalling || !searchQuery.trim()}
      title="Install package"
    >
      {#if isInstalling}
        <svelte:component this={CircleNotch} size={14} class="spinning" />
      {:else}
        <svelte:component this={Plus} size={14} />
      {/if}
    </button>
  </div>

  <div class="packages-list">
    {#if isLoading}
      <div class="loading-state">
        <svelte:component this={CircleNotch} size={20} class="spinning" />
        <span>Loading packages...</span>
      </div>
    {:else if packages.length === 0}
      <div class="empty-state">
        <span>No packages installed</span>
      </div>
    {:else}
      {#each Object.entries(groupedPackages) as [namespace, pkgs]}
        <div class="namespace-group">
          <div class="namespace-header">@{namespace}</div>
          {#each pkgs as pkg}
            <div class="package-item">
              <div class="package-info">
                <span class="package-name">{pkg.name}</span>
                <span class="package-version">{pkg.version}</span>
              </div>
              <button
                class="remove-button"
                on:click={() => handleRemovePackage(pkg)}
                title="Remove package"
              >
                <svelte:component this={Trash} size={14} />
              </button>
            </div>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .packages-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md) var(--space-md) var(--space-sm);
    border-bottom: 1px solid var(--color-border);
  }

  .panel-title {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-text-secondary);
    margin: 0;
  }

  .package-count {
    font-size: 11px;
    color: var(--color-text-tertiary);
    background: var(--color-bg-secondary);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }

  .install-bar {
    display: flex;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--color-border);
  }

  .install-input {
    flex: 1;
    padding: var(--space-sm);
    font-size: 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg-primary);
  }

  .install-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .install-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--color-accent);
    color: white;
    transition: all var(--transition-fast);
  }

  .install-button:hover:not(:disabled) {
    background: var(--color-accent-hover);
  }

  .install-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .packages-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-sm) 0;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-xl);
    color: var(--color-text-tertiary);
    font-size: 12px;
  }

  .namespace-group {
    margin-bottom: var(--space-sm);
  }

  .namespace-header {
    padding: var(--space-xs) var(--space-md);
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .package-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-sm) var(--space-md);
    transition: background var(--transition-fast);
  }

  .package-item:hover {
    background: var(--color-bg-hover);
  }

  .package-info {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
  }

  .package-name {
    font-size: 13px;
    color: var(--color-text-primary);
  }

  .package-version {
    font-size: 11px;
    color: var(--color-text-tertiary);
  }

  .remove-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    color: var(--color-text-tertiary);
    opacity: 0;
    transition: all var(--transition-fast);
  }

  .package-item:hover .remove-button {
    opacity: 1;
  }

  .remove-button:hover {
    background: var(--color-error-bg);
    color: var(--color-error);
  }

  :global(.spinning) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
