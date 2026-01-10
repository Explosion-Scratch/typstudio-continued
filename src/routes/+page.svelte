<script lang="ts">
  import Editor from "../components/Editor.svelte";
  import Preview from "../components/Preview.svelte";
  import { project, shell, recentProjects } from "../lib/stores";
  import type { ProjectChangeEvent } from "../lib/ipc";
  import WelcomeScreen from "../components/WelcomeScreen.svelte";
  import LoadingScreen from "../components/LoadingScreen.svelte";
  import { onMount } from "svelte";
  import { appWindow } from "@tauri-apps/api/window";
  import SidePanel from "../components/SidePanel.svelte";
  import Modals from "../components/ShellModal.svelte";
  import { fade } from "svelte/transition";

  onMount(() => {
    let cleanup: (() => void) | undefined;

    appWindow.listen<ProjectChangeEvent>("project_changed", ({ payload }) => {
      shell.selectFile(undefined);
      project.set(payload.project);

      if (payload.project) {
        recentProjects.addProject(payload.project.root);

        setTimeout(() => {
          shell.selectFile("/main.typ");
        }, 100);
      }
    }).then((unlisten) => {
      cleanup = unlisten;
    });

    setTimeout(() => {
      shell.setInitializing(false);
    }, 500);

    return () => {
      if (cleanup) cleanup();
    };
  });
</script>

<div class="app-container" data-tauri-drag-region>
  <Modals />

  {#if $shell.isInitializing}
    <LoadingScreen />
  {:else if !$project}
    <WelcomeScreen />
  {:else}
    <div class="editor-layout" in:fade={{ duration: 150 }}>
      {#if $shell.sidebarVisible}
        <SidePanel />
      {/if}

      {#if $shell.selectedFile}
        <Editor class="editor-pane" path={$shell.selectedFile} />
        <Preview />
      {:else}
        <div class="empty-state">
          <span class="empty-text">Select a file to start editing</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: var(--color-bg-primary);
    overflow: hidden;
  }

  .editor-layout {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  :global(.editor-pane) {
    flex: 1;
    min-width: 0;
    border-right: 1px solid var(--color-border);
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-secondary);
  }

  .empty-text {
    color: var(--color-text-tertiary);
    font-size: 14px;
  }
</style>
