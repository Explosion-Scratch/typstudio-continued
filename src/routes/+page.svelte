<script lang="ts">
  import Editor from "../components/Editor.svelte";
  import Preview from "../components/Preview.svelte";
  import { project, shell, recentProjects } from "../lib/stores";
  import type { ProjectChangeEvent, TypstJump, TypstCompileEvent } from "../lib/ipc";
  import { listDir, revealPath, renameFile, getDocumentSources } from "../lib/ipc";
  import WelcomeScreen from "../components/WelcomeScreen.svelte";
  import LoadingScreen from "../components/LoadingScreen.svelte";
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { debounce } from "../lib/fn";

  const appWindow = getCurrentWindow();
  import SidePanel from "../components/SidePanel.svelte";
  import Modals from "../components/ShellModal.svelte";
  import { fade } from "svelte/transition";
  import Resizer from "../components/Resizer.svelte";
  import FileViewer from "../components/FileViewer.svelte";
  import ContextMenu, { type ContextMenuItem } from "../components/ContextMenu.svelte";
  import {
    Eye,
    Pencil,
    CircleNotch,
    FileTextDuotone,
    FolderOpenDuotone,
    FolderDuotone,
    Export,
    GitDiff,
    ArrowLeft,
  } from "$lib/icons";
  import { diffStats, showDiffEditor } from "$lib/diff";

  let containerWidth = 0;
  let containerRef: HTMLDivElement;
  let sidebarContentWidth = 240;
  let editorWidth = 0;
  let contentAreaRef: HTMLDivElement;
  let contentAreaWidth = 0;

  let exportStatus: string | null = null;
  let isResizing = false;

  let showTitleMenu = false;
  let titleMenuX = 0;
  let titleMenuY = 0;
  let titleMenuItems: ContextMenuItem[] = [];

  const MIN_SIDEBAR_CONTENT = 150;
  const COLLAPSE_THRESHOLD = 120;
  const MIN_PANE_WIDTH = 500;
  const ICON_BAR_WIDTH = 48;

  const TYPST_EXTENSIONS = [".typ"];
  const EDITABLE_EXTENSIONS = [".typ", ".bib", ".md", ".txt"];

  $: selectedExtension = $shell.selectedFile?.toLowerCase().split(".").pop() || "";
  $: isTypstFile = TYPST_EXTENSIONS.some((ext) => $shell.selectedFile?.toLowerCase().endsWith(ext));
  $: isPreviewable = TYPST_EXTENSIONS.some((ext) => $shell.previewFile?.toLowerCase().endsWith(ext));
  $: isEditableFile = EDITABLE_EXTENSIONS.some((ext) =>
    $shell.selectedFile?.toLowerCase().endsWith(ext),
  );
  $: isSelectedInDocument = $shell.documentSourceFiles.includes($shell.selectedFile || "");

  $: canShowSidebarContent = containerWidth >= 500;
  $: showSidebarContent = $shell.sidebarVisible && canShowSidebarContent;

  $: projectName = $project?.root?.split("/").pop() || "";
  $: fileName = $shell.selectedFile?.split("/").pop() || "";
  $: titleDisplay =
    projectName && fileName ? `${projectName} — ${fileName}` : fileName || projectName || "";

  $: hasDiff = $diffStats.added > 0 || $diffStats.removed > 0;

  $: availableContentWidth = contentAreaWidth;
  $: canShowBothPanes = availableContentWidth >= MIN_PANE_WIDTH * 2;

  $: if (canShowBothPanes && $shell.viewMode !== "both") {
    shell.setViewMode("both");
  } else if (!canShowBothPanes && $shell.viewMode === "both") {
    shell.setViewMode("editor");
  }

  $: showEditor = $shell.viewMode === "both" || $shell.viewMode === "editor";
  $: showPreview = ($shell.viewMode === "both" || $shell.viewMode === "preview") && isPreviewable;
  $: showViewToggle = !canShowBothPanes && (isTypstFile || isPreviewable);

  $: if (editorWidth === 0 && contentAreaWidth > 0) {
    editorWidth = Math.floor(contentAreaWidth / 2);
  }

  // Handle recompile on preview show/hide
  let lastShowPreview: boolean | undefined = undefined;
  $: if (showPreview !== lastShowPreview) {
    lastShowPreview = showPreview;
    appWindow.emit("trigger_compile");
  }

  // Ensure layout is recalculated when content area appears
  $: if (contentAreaRef) {
    handleWindowResize();
  }

  const handleSidebarContentResize = (newSize: number, isDragging: boolean = false) => {
    isResizing = isDragging;

    if (newSize < COLLAPSE_THRESHOLD && isDragging) {
      shell.setSidebarVisible(false);
      sidebarContentWidth = MIN_SIDEBAR_CONTENT;
      return;
    }

    sidebarContentWidth = Math.max(MIN_SIDEBAR_CONTENT, newSize);
    if (containerWidth > 0) {
      shell.setSidebarWidthPercent((sidebarContentWidth / containerWidth) * 100);
    }
  };

  const handleSidebarResizeEnd = () => {
    isResizing = false;
  };

  const handleEditorResize = (newSize: number) => {
    editorWidth = Math.max(MIN_PANE_WIDTH, Math.min(newSize, contentAreaWidth - MIN_PANE_WIDTH));
    if (contentAreaWidth > 0) {
      shell.setEditorWidthPercent((editorWidth / contentAreaWidth) * 100);
    }
  };

  const handleWindowResize = () => {
    if (containerRef) {
      containerWidth = containerRef.offsetWidth;
    }
    if (contentAreaRef) {
      const newContentWidth = contentAreaRef.offsetWidth;
      if (contentAreaWidth > 0 && newContentWidth !== contentAreaWidth) {
        const ratio = editorWidth / contentAreaWidth;
        editorWidth = Math.floor(newContentWidth * ratio);
      }
      contentAreaWidth = newContentWidth;
    }
  };

  const handleTitleClick = async (event: MouseEvent) => {
    if (event.ctrlKey) return;
    handleShowInFinder();
  };

  const handleOpenFolderDialog = async () => {
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");

      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Open Folder",
      });

      if (selected && typeof selected === "string") {
        shell.setIsOpeningProject(true);
        shell.setLoadingStage(`Opening ${selected.split("/").pop() || "folder"}...`, 0);
        await invoke("open_project", { path: selected });
      }
    } catch (e) {
      console.error("Failed to open folder dialog:", e);
      shell.setIsOpeningProject(false);
    }
  };

  const handlePrintPdf = async () => {
    await handleExport("pdf");
  };

  const handleExport = async (type: "pdf" | "svg" | "png", filePath?: string) => {
    try {
      exportStatus = `Preparing ${type.toUpperCase()} export...`;
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");

      const defaultName = filePath ? filePath.split("/").pop()?.replace(".typ", "") : "export";
      const filters = {
        pdf: [{ name: "PDF", extensions: ["pdf"] }],
        svg: [{ name: "SVG Zip", extensions: ["zip"] }],
        png: [{ name: "PNG Zip", extensions: ["zip"] }],
      };

      const savePath = await save({
        title: `Export ${type.toUpperCase()}`,
        defaultPath: `${defaultName}.${type === "pdf" ? "pdf" : "zip"}`,
        filters: filters[type],
      });

      if (savePath) {
        exportStatus = `Exporting ${type.toUpperCase()}...`;
        await invoke(`export_${type}`, {
          path: savePath,
        });
      }
      exportStatus = null;
    } catch (e) {
      console.error(`Failed to export ${type}:`, e);
      exportStatus = null;
    }
  };

  let compileTimer: any;
  $: if ($shell.previewState === 1) {
    // Compiling
    if (!compileTimer) {
      compileTimer = setTimeout(() => {
        shell.setIsCompilingLong(true);
      }, 1000);
    }
  } else {
    if (compileTimer) {
      clearTimeout(compileTimer);
      compileTimer = null;
    }
    shell.setIsCompilingLong(false);
  }

  const handleRename = () => {
    if (!$shell.selectedFile) return;
    const currentPath = $shell.selectedFile;
    const currentName = currentPath.split("/").pop() || "";

    shell.createModal({
      type: "input",
      title: "Rename File",
      placeholder: currentName,
      callback: async (newName) => {
        if (newName && newName !== currentName) {
          try {
            const parentDir = currentPath.substring(0, currentPath.lastIndexOf("/"));
            const newPath = `${parentDir}/${newName}`;

            await renameFile(currentPath, newPath);
            shell.selectFile(newPath);
          } catch (e) {
            console.error("Failed to rename file:", e);
          }
        }
      },
    });
  };

  const handleShowInFinder = async () => {
    if ($shell.selectedFile) {
      try {
        await revealPath($shell.selectedFile);
      } catch (e) {
        console.error("Failed to reveal path:", e);
      }
    }
  };

  const handleExportPdf = () => {
    handlePrintPdf();
  };

  const handleTitleContextMenu = (event: MouseEvent) => {
    event.preventDefault();
    titleMenuX = event.clientX;
    titleMenuY = event.clientY;

    titleMenuItems = [];

    if ($shell.selectedFile) {
      titleMenuItems = [
        {
          label: "Rename",
          icon: Pencil,
          action: handleRename,
        },
        {
          label: "Reveal in Finder",
          icon: FileTextDuotone,
          action: handleShowInFinder,
        },
        {
          label: "Reveal Folder",
          icon: FolderDuotone,
          action: async () => {
            if ($shell.selectedFile) {
              try {
                const parentDir =
                  $shell.selectedFile.substring(0, $shell.selectedFile.lastIndexOf("/")) || "/";
                await revealPath(parentDir);
              } catch (e) {
                console.error(e);
              }
            }
          },
        },
        {
          label: "",
          action: () => {},
          divider: true,
        },
        {
          label: "Export PDF",
          icon: Export,
          action: handleExportPdf,
          disabled: !isTypstFile,
        },
        {
          label: $showDiffEditor ? "Hide Diff" : "Show Diff",
          icon: GitDiff,
          action: () => showDiffEditor.update((v) => !v),
        },
        {
          label: "",
          action: () => {},
          divider: true,
        },
      ];
    }

    titleMenuItems.push({
      label: "Open Folder...",
      icon: FolderOpenDuotone,
      action: handleOpenFolderDialog,
    });

    showTitleMenu = true;
  };

  onMount(() => {
    let cleanup: (() => void)[] = [];

    handleWindowResize();
    window.addEventListener("resize", handleWindowResize);
    cleanup.push(() => window.removeEventListener("resize", handleWindowResize));

    const resizeObserver = new ResizeObserver(() => {
      handleWindowResize();
    });

    const observeContentArea = () => {
      if (contentAreaRef) {
        resizeObserver.observe(contentAreaRef);
      }
    };

    observeContentArea();
    cleanup.push(() => resizeObserver.disconnect());

    appWindow
      .listen<ProjectChangeEvent>("project_changed", async ({ payload }) => {
        shell.setIsOpeningProject(false);
        shell.selectFile(undefined);
        shell.setPreviewFile(undefined);
        shell.setDocumentSourceFiles([]);
        project.set(payload.project);

        if (payload.project) {
          recentProjects.addProject(payload.project.root);

          try {
            const files = await listDir("/");
            const mainFile = files.find((f) => f.name === "main.typ");
            const firstTyp = files.find((f) => f.name.endsWith(".typ"));
            
            let previewPath: string | undefined;
            if (mainFile) {
              shell.selectFile("/main.typ");
              shell.setPreviewFile("/main.typ");
              previewPath = "/main.typ";
            } else if (firstTyp) {
              shell.selectFile("/" + firstTyp.name);
              shell.setPreviewFile("/" + firstTyp.name);
              previewPath = "/" + firstTyp.name;
            }

            if (previewPath) {
              setTimeout(() => {
                appWindow.emit("trigger_compile", { previewFile: previewPath });
              }, 100);
            }
          } catch (e) {
            console.error("Failed to list files:", e);
          }
        }
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });
    
    const fetchDocumentSourcesDebounced = debounce(async () => {
      try {
        const sources = await getDocumentSources();
        shell.setDocumentSourceFiles(sources);
      } catch (e) {
        console.error("Failed to get document sources:", e);
      }
    }, 500);
    
    cleanup.push(() => fetchDocumentSourcesDebounced.cancel());

    appWindow
      .listen<TypstCompileEvent>("typst_compile", () => {
        fetchDocumentSourcesDebounced();
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen<{ path: string }>("preview_document", async ({ payload }) => {
        shell.setPreviewFile(payload.path);
        shell.selectFile(payload.path);
        shell.setDocumentSourceFiles([]);
        setTimeout(() => {
          appWindow.emit("trigger_compile", { previewFile: payload.path });
        }, 100);
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen<TypstJump>("editor_goto_location", ({ payload }) => {
        // If editor is not showing, switch to editor view
        if ($shell.viewMode === "preview") {
          shell.setViewMode("editor");
        }

        if (payload.filepath !== $shell.selectedFile) {
          shell.selectFile(payload.filepath);
          setTimeout(() => {
            if (payload.start) {
              appWindow.emit("jump_to_position", {
                line: payload.start[0],
                column: payload.start[1],
              });
            }
          }, 150);
        } else {
          if (payload.start) {
            appWindow.emit("jump_to_position", {
              line: payload.start[0],
              column: payload.start[1],
            });
          }
        }
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen("toggle_sidebar", () => {
        shell.toggleSidebar();
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen("toggle_preview", () => {
        shell.toggleViewMode();
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen<{ path: string }>("export_file_as_pdf", ({ payload }) => {
        handleExport("pdf", payload.path);
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen<{ path: string }>("export_file_as_svg", ({ payload }) => {
        handleExport("svg", payload.path);
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen<{ path: string }>("export_file_as_png", ({ payload }) => {
        handleExport("png", payload.path);
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen("menu_export_pdf", () => {
        handleExport("pdf");
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen("menu_export_svg", () => {
        handleExport("svg");
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen<{ stage: string; progress: number; message?: string }>(
        "loading_progress",
        ({ payload }) => {
          shell.setLoadingStage(payload.message || payload.stage, payload.progress);
        },
      )
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen("menu_export_png", () => {
        handleExport("png");
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    appWindow
      .listen("menu_view_diff", () => {
        showDiffEditor.update((v) => !v);
      })
      .then((unlisten) => {
        cleanup.push(unlisten);
      });

    shell.setLoadingStage("Ready", 100);
    shell.setInitializing(false);

    return () => {
      cleanup.forEach((fn) => fn());
    };
  });
</script>

<div bind:this={containerRef} class="app-container" data-tauri-drag-region>
  <Modals />

  <div class="title-bar" data-tauri-drag-region>
    <button
      class="title-button"
      on:click={handleTitleClick}
      on:contextmenu={handleTitleContextMenu}
      title="Reveal in Finder (Right click to open folder)"
    >
      <span class="title-text">{titleDisplay}</span>
    </button>
    {#if hasDiff && $shell.selectedFile}
      <button
        class="diff-button"
        on:click={() => showDiffEditor.update((v) => !v)}
        on:contextmenu={handleTitleContextMenu}
        title={$showDiffEditor ? "Hide Diff" : "Show Diff"}
      >
        <div class="diff-stats">
          {#if $diffStats.added > 0}
            <span class="diff-added">+{ $diffStats.added }</span>
          {/if}
          {#if $diffStats.removed > 0}
            <span class="diff-removed">-{ $diffStats.removed }</span>
          {/if}
        </div>
      </button>
    {/if}
  </div>

  {#if showTitleMenu}
    <ContextMenu
      bind:x={titleMenuX}
      bind:y={titleMenuY}
      items={titleMenuItems}
      on:close={() => (showTitleMenu = false)}
    />
  {/if}

  {#if exportStatus || $shell.isCompilingLong}
    <div class="export-overlay" transition:fade={{ duration: 150 }}>
      <CircleNotch size={24} class="spinner" weight="bold" />
      <span>{exportStatus || "Compiling..."}</span>
    </div>
  {/if}

  {#if $shell.isInitializing || $shell.isOpeningProject}
    <LoadingScreen stage={$shell.loadingStage} progress={$shell.loadingProgress} />
  {:else if !$project}
    <WelcomeScreen />
  {:else}
    <div class="editor-layout" in:fade={{ duration: 150 }}>
      <div
        class="sidebar-wrapper"
        style={showSidebarContent ? `width: ${sidebarContentWidth + ICON_BAR_WIDTH}px` : ""}
      >
        <SidePanel showContent={showSidebarContent} />
      </div>

      {#if showSidebarContent}
        <Resizer
          direction="horizontal"
          bind:size={sidebarContentWidth}
          minSize={0}
          maxSize={Math.min(400, containerWidth * 0.3)}
          on:resize={(e) => handleSidebarContentResize(e.detail, true)}
          on:resizeend={handleSidebarResizeEnd}
        />
      {/if}

      <div bind:this={contentAreaRef} class="content-area">
        {#if $shell.selectedFile}
          {#if isEditableFile}
            <div
              class="editor-container"
              class:hidden={!showEditor}
              style={$shell.viewMode === "both" && isPreviewable
                ? `width: ${editorWidth}px`
                : showEditor
                  ? "flex: 1"
                  : ""}
            >
              <Editor class="editor-pane" path={$shell.selectedFile} isVisible={showEditor} />
            </div>
            {#if isPreviewable}
              {#if $shell.viewMode === "both"}
                <Resizer
                  direction="horizontal"
                  bind:size={editorWidth}
                  minSize={MIN_PANE_WIDTH}
                  maxSize={contentAreaWidth - MIN_PANE_WIDTH}
                  on:resize={(e) => handleEditorResize(e.detail)}
                />
              {/if}
              {#if showPreview}
                <div class="preview-container" style="flex: 1">
                  <Preview />
                </div>
              {/if}
            {/if}
          {:else}
            <FileViewer path={$shell.selectedFile} />
          {/if}
        {:else}
          <div class="empty-state bg-vibrant">
            <span class="empty-text">Select a file to start editing</span>
          </div>
        {/if}

        {#if showViewToggle}
          <button
            class="view-toggle"
            on:click={() => shell.toggleViewMode()}
            title={$shell.viewMode === "editor" ? "Show Preview" : "Show Editor"}
          >
            <svelte:component
              this={$shell.viewMode === "editor" ? Eye : Pencil}
              size={16}
              weight="duotone"
            />
          </button>
        {/if}

        {#if $showDiffEditor}
          <button
              class="diff-back-button"
              on:click={() => showDiffEditor.set(false)}
              title="Back to Editor"
              style={showViewToggle ? "top: 48px;" : "top: 8px;"}
          >
               <ArrowLeft size={16} />
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: transparent;
    overflow: hidden;
    position: relative;
  }

  .title-bar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    user-select: none;
    border-bottom: 1px solid var(--color-border);
  }

  .title-button {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px 12px;
    height: 22px;
    border-radius: var(--radius-md);
    background: transparent;
    border: 1px solid transparent;
    transition: all var(--transition-fast);
    cursor: default;
    max-width: 60%;
  }

  .title-button:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-border);
  }

  .title-button:active {
    background: var(--color-bg-active);
  }

  .diff-button {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px 8px;
    height: 22px;
    border-radius: var(--radius-md);
    background: transparent;
    border: 1px solid transparent;
    transition: all var(--transition-fast);
    cursor: default;
    margin-left: 4px;
  }

  .diff-button:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-border);
  }

  .diff-button:active {
    background: var(--color-bg-active);
  }

  .title-text {
    font-size: 12px;
    color: var(--color-text-secondary);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
  }

  .diff-stats {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 500;
  }

  .diff-added {
    color: var(--color-success);
  }

  .diff-removed {
    color: var(--color-error);
  }

  .export-overlay {
    position: fixed;
    top: 36px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-sm) var(--space-lg);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    z-index: 1000;
    box-shadow: var(--shadow-lg);
    font-size: 13px;
    color: var(--color-text-secondary);
  }

  .export-overlay :global(.spinner) {
    animation: spin 1s linear infinite;
    display: block;
    transform-origin: center;
    flex-shrink: 0;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .editor-layout {
    display: flex;
    flex: 1;
    min-height: 0;
    padding-top: 32px;
    position: relative;
  }

  .sidebar-wrapper {
    display: flex;
    flex-shrink: 0;
  }

  .content-area {
    display: flex;
    flex: 1;
    min-width: 0;
    position: relative;
  }

  .editor-container {
    display: flex;
    min-width: 200px;
    overflow: hidden;
  }

  .editor-container.hidden {
    position: absolute;
    left: -9999px;
    width: 1px;
    height: 1px;
    overflow: hidden;
  }

  .preview-container {
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
  }

  .empty-text {
    color: var(--color-text-tertiary);
    font-size: 14px;
  }

  .view-toggle {
    position: absolute;
    top: 8px;
    right: 12px;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-secondary);
    cursor: pointer;
    z-index: 100;
    box-shadow: var(--shadow-sm);
    transition: all var(--transition-fast);
  }

  .view-toggle:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .diff-back-button {
    position: absolute;
    right: 12px;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-secondary);
    cursor: pointer;
    z-index: 100;
    box-shadow: var(--shadow-sm);
    transition: all var(--transition-fast);
  }

  .diff-back-button:hover {
    background: var(--color-bg-hover);
    transform: translateY(-1px);
    color: var(--color-text-primary);
  }
</style>
