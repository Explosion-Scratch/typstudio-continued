<script lang="ts">
  import Editor from "../components/Editor.svelte";
  import Preview from "../components/Preview.svelte";
  import { project, shell, recentProjects } from "../lib/stores";
  import type { ProjectChangeEvent, TypstJump } from "../lib/ipc";
  import { listDir } from "../lib/ipc";
  import WelcomeScreen from "../components/WelcomeScreen.svelte";
  import LoadingScreen from "../components/LoadingScreen.svelte";
  import { onMount } from "svelte";
  import { appWindow } from "@tauri-apps/api/window";
  import SidePanel from "../components/SidePanel.svelte";
  import Modals from "../components/ShellModal.svelte";
  import { fade } from "svelte/transition";
  import Resizer from "../components/Resizer.svelte";
  import FileViewer from "../components/FileViewer.svelte";

  let sidebarWidth = 280;
  let editorWidth = 50;

  const TYPST_EXTENSIONS = [".typ"];
  const EDITABLE_EXTENSIONS = [".typ", ".bib", ".md", ".txt"];

  $: selectedExtension = $shell.selectedFile?.toLowerCase().split(".").pop() || "";
  $: isTypstFile = TYPST_EXTENSIONS.some(ext => $shell.selectedFile?.toLowerCase().endsWith(ext));
  $: isEditableFile = EDITABLE_EXTENSIONS.some(ext => $shell.selectedFile?.toLowerCase().endsWith(ext));

  onMount(() => {
    let cleanup: (() => void)[] = [];

    appWindow.listen<ProjectChangeEvent>("project_changed", async ({ payload }) => {
      shell.selectFile(undefined);
      project.set(payload.project);

      if (payload.project) {
        recentProjects.addProject(payload.project.root);

        try {
          const files = await listDir("/");
          const mainFile = files.find(f => f.name === "main.typ");
          if (mainFile) {
            shell.selectFile("/main.typ");
          } else {
            const firstTyp = files.find(f => f.name.endsWith(".typ"));
            if (firstTyp) {
              shell.selectFile("/" + firstTyp.name);
            }
          }
        } catch (e) {
          console.error("Failed to list files:", e);
        }
      }
    }).then((unlisten) => {
      cleanup.push(unlisten);
    });

    appWindow.listen<TypstJump>("editor_goto_location", ({ payload }) => {
      if (payload.filepath !== $shell.selectedFile) {
        shell.selectFile(payload.filepath);
        setTimeout(() => {
          if (payload.start) {
            appWindow.emit("jump_to_position", { line: payload.start[0], column: payload.start[1] });
          }
        }, 150);
      } else {
        if (payload.start) {
          appWindow.emit("jump_to_position", { line: payload.start[0], column: payload.start[1] });
        }
      }
    }).then((unlisten) => {
      cleanup.push(unlisten);
    });

    setTimeout(() => {
      shell.setInitializing(false);
    }, 500);

    return () => {
      cleanup.forEach(fn => fn());
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
        <div class="sidebar-container" style="width: {sidebarWidth}px">
          <SidePanel />
        </div>
        <Resizer
          direction="horizontal"
          bind:size={sidebarWidth}
          minSize={200}
          maxSize={500}
        />
      {/if}

      {#if $shell.selectedFile}
        {#if isEditableFile}
          <div class="editor-container" style="flex: {editorWidth}">
            <Editor class="editor-pane" path={$shell.selectedFile} />
          </div>
          {#if isTypstFile}
            <Resizer
              direction="horizontal"
              bind:size={editorWidth}
              minSize={20}
              maxSize={80}
            />
            <Preview />
          {/if}
        {:else}
          <FileViewer path={$shell.selectedFile} />
        {/if}
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
    padding-top: 28px;
  }

  .sidebar-container {
    display: flex;
    flex-shrink: 0;
    height: 100%;
    overflow: hidden;
  }

  .editor-container {
    display: flex;
    min-width: 200px;
    overflow: hidden;
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

