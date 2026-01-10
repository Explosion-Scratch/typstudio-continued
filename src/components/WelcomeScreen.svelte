<script lang="ts">
  import { FolderPlus, Clock, FolderDuotone, ArrowRight, CircleNotch } from "$lib/icons";
  import { recentProjects, shell } from "$lib/stores";
  import { open } from "@tauri-apps/api/dialog";
  import { invoke } from "@tauri-apps/api";
  import { fade } from "svelte/transition";

  let isLoading = false;
  let loadingMessage = "Opening project...";
  let loadingProgress = 0;

  const simulateFontLoading = () => {
    loadingProgress = 0;
    loadingMessage = "Loading fonts...";
    
    const steps = [
      { progress: 15, message: "Loading fonts..." },
      { progress: 35, message: "Loading fonts..." },
      { progress: 55, message: "Loading fonts..." },
      { progress: 75, message: "Loading fonts..." },
      { progress: 90, message: "Finalizing..." },
      { progress: 100, message: "Ready" },
    ];
    
    let stepIndex = 0;
    const interval = setInterval(() => {
      if (stepIndex < steps.length) {
        loadingProgress = steps[stepIndex].progress;
        loadingMessage = steps[stepIndex].message;
        stepIndex++;
      } else {
        clearInterval(interval);
      }
    }, 200);
    
    return () => clearInterval(interval);
  };

  const handleNewProject = async () => {
    const path = await open({
      directory: true,
      multiple: false,
      title: "Select project folder",
    });

    if (path && typeof path === "string") {
      isLoading = true;
      loadingMessage = "Creating project...";
      loadingProgress = 0;
      const cleanup = simulateFontLoading();
      try {
        await invoke("open_project", { path });
      } finally {
        cleanup();
      }
    }
  };

  const handleOpenProject = async () => {
    const path = await open({
      directory: true,
      multiple: false,
      title: "Open project folder",
    });

    if (path && typeof path === "string") {
      isLoading = true;
      loadingMessage = "Opening project...";
      loadingProgress = 0;
      const cleanup = simulateFontLoading();
      try {
        await invoke("open_project", { path });
      } finally {
        cleanup();
      }
    }
  };

  const handleOpenRecent = async (path: string) => {
    isLoading = true;
    loadingMessage = "Opening project...";
    loadingProgress = 0;
    const cleanup = simulateFontLoading();
    try {
      await invoke("open_project", { path });
    } catch (e) {
      console.error("Failed to open project:", e);
      recentProjects.removeProject(path);
      isLoading = false;
    } finally {
      cleanup();
    }
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) return "Today";
    if (days === 1) return "Yesterday";
    if (days < 7) return `${days} days ago`;
    return date.toLocaleDateString();
  };
</script>

<div class="welcome-screen" in:fade={{ duration: 150 }}>
  <div class="welcome-content">
    <div class="welcome-header">
      <h1 class="welcome-title">Typstudio</h1>
      <p class="welcome-subtitle">A modern editor for Typst documents</p>
    </div>

    <div class="actions">
      <button class="action-button" on:click={handleNewProject}>
        <div class="action-icon">
          <FolderPlus size={24} weight="duotone" />
        </div>
        <div class="action-text">
          <span class="action-title">New Project</span>
          <span class="action-description">Create a new Typst project</span>
        </div>
        <ArrowRight size={16} class="action-arrow" />
      </button>

      <button class="action-button" on:click={handleOpenProject}>
        <div class="action-icon">
          <FolderDuotone size={24} weight="duotone" />
        </div>
        <div class="action-text">
          <span class="action-title">Open Project</span>
          <span class="action-description">Open an existing project folder</span>
        </div>
        <ArrowRight size={16} class="action-arrow" />
      </button>
    </div>

    {#if $recentProjects.length > 0}
      <div class="recent-section">
        <div class="section-header">
          <Clock size={16} weight="bold" />
          <span>Recent Projects</span>
        </div>

        <div class="recent-list">
          {#each $recentProjects as project}
            <button
              class="recent-item"
              on:click={() => handleOpenRecent(project.path)}
            >
              <FolderDuotone size={18} weight="duotone" class="recent-icon" />
              <div class="recent-details">
                <span class="recent-name">{project.name}</span>
                <span class="recent-path">{project.path}</span>
              </div>
              <span class="recent-date">{formatDate(project.lastOpened)}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .welcome-screen {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-secondary);
    padding: var(--space-xl);
  }

  .welcome-content {
    max-width: 480px;
    width: 100%;
  }

  .welcome-header {
    text-align: center;
    margin-bottom: var(--space-2xl);
  }

  .welcome-title {
    font-size: 28px;
    font-weight: 700;
    color: var(--color-text-primary);
    margin: 0 0 var(--space-sm);
  }

  .welcome-subtitle {
    font-size: 14px;
    color: var(--color-text-secondary);
    margin: 0;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    margin-bottom: var(--space-2xl);
  }

  .action-button {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    padding: var(--space-md) var(--space-lg);
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    text-align: left;
    transition: all var(--transition-fast);
    cursor: pointer;
  }

  .action-button:hover {
    background: var(--color-bg-primary);
    border-color: var(--color-border-strong);
    box-shadow: var(--shadow-sm);
  }

  .action-button:hover :global(.action-arrow) {
    opacity: 1;
    transform: translateX(2px);
  }

  .action-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    background: var(--color-bg-secondary);
    border-radius: var(--radius-md);
    color: var(--color-text-secondary);
  }

  .action-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .action-title {
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .action-description {
    font-size: 12px;
    color: var(--color-text-tertiary);
  }

  .action-button :global(.action-arrow) {
    color: var(--color-text-tertiary);
    opacity: 0;
    transition: all var(--transition-fast);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: var(--space-md);
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .recent-item {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    width: 100%;
    padding: var(--space-md) var(--space-lg);
    text-align: left;
    transition: background var(--transition-fast);
    cursor: pointer;
  }

  .recent-item:hover {
    background: var(--color-bg-hover);
  }

  .recent-item :global(.recent-icon) {
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }

  .recent-details {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent-name {
    font-weight: 500;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .recent-path {
    font-size: 11px;
    color: var(--color-text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .recent-date {
    font-size: 11px;
    color: var(--color-text-tertiary);
    flex-shrink: 0;
  }
</style>
