<script lang="ts">
  import PreviewPage from "./PreviewPage.svelte";
  import CompileErrorDisplay from "./CompileErrorDisplay.svelte";
  import ZoomControls from "./ZoomControls.svelte";
  import { onMount } from "svelte";
  import type { TypstCompileEvent, TypstSourceDiagnostic } from "../lib/ipc";
  import { appWindow } from "@tauri-apps/api/window";
  import { shell, PreviewState } from "$lib/stores";
  import { fade } from "svelte/transition";

  let container: HTMLDivElement;
  let pagesContainer: HTMLDivElement;
  let previousEvent: MouseEvent | undefined;

  let isDragging = false;
  let mouseDownPosition: { x: number; y: number; time: number } | null = null;
  let zoom = 1.0;
  let minZoom = 0.25;
  let maxZoom = 4.0;

  const CLICK_THRESHOLD_DISTANCE = 5;
  const CLICK_THRESHOLD_TIME = 200;

  let pages = 0;
  let hash: string | null = null;
  let previousHash: string | null = null;
  let width: number;
  let height: number;
  let currentErrors: TypstSourceDiagnostic[] = [];

  let isVisible: boolean = true;
  let isFading = false;

  const handleMouseDown = (event: MouseEvent) => {
    if (event.button === 0) {
      mouseDownPosition = {
        x: event.clientX,
        y: event.clientY,
        time: Date.now(),
      };
      isDragging = false;
    }
  };

  const handleMouseUp = (event: MouseEvent) => {
    if (mouseDownPosition) {
      const dx = event.clientX - mouseDownPosition.x;
      const dy = event.clientY - mouseDownPosition.y;
      const distance = Math.sqrt(dx * dx + dy * dy);
      const elapsed = Date.now() - mouseDownPosition.time;

      if (distance < CLICK_THRESHOLD_DISTANCE && elapsed < CLICK_THRESHOLD_TIME) {
        handlePreviewClick(event);
      }
    }
    mouseDownPosition = null;
    isDragging = false;
    previousEvent = undefined;
  };

  const handlePreviewClick = (event: MouseEvent) => {
    console.log("Preview clicked at", event.clientX, event.clientY);
  };

  const handleMove = (event: MouseEvent) => {
    if (mouseDownPosition) {
      const dx = event.clientX - mouseDownPosition.x;
      const dy = event.clientY - mouseDownPosition.y;
      const distance = Math.sqrt(dx * dx + dy * dy);

      if (distance >= CLICK_THRESHOLD_DISTANCE) {
        isDragging = true;
      }

      if (isDragging && previousEvent) {
        const deltaX = previousEvent.screenX - event.screenX;
        const deltaY = previousEvent.screenY - event.screenY;
        container.scrollBy({ left: deltaX, top: deltaY });
      }
    }
    previousEvent = event;
  };

  const handleWheel = (event: WheelEvent) => {
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
    }
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.metaKey || event.ctrlKey) {
      switch (event.key) {
        case "=":
        case "+":
          event.preventDefault();
          zoom = Math.min(maxZoom, zoom * 1.2);
          break;
        case "-":
          event.preventDefault();
          zoom = Math.max(minZoom, zoom / 1.2);
          break;
        case "0":
          event.preventDefault();
          zoom = 1.0;
          break;
      }
    }
  };

  const handleZoomIn = () => {
    zoom = Math.min(maxZoom, zoom * 1.2);
  };

  const handleZoomOut = () => {
    zoom = Math.max(minZoom, zoom / 1.2);
  };

  const handleZoomReset = () => {
    zoom = 1.0;
  };

  const handleErrorClick = (error: TypstSourceDiagnostic) => {
    appWindow.emit("scroll_to_position", { position: error.range.start });
  };

  onMount(() => {
    let cleanup: (() => void)[] = [];

    (async () => {
      const unsubscribeCompile = await appWindow.listen<TypstCompileEvent>(
        "typst_compile",
        ({ payload }) => {
          const { document, diagnostics } = payload;

          currentErrors = diagnostics || [];
          shell.setCurrentErrors(currentErrors);

          if (document) {
            previousHash = hash;
            pages = document.pages;
            hash = document.hash;
            width = document.width;
            height = document.height;

            if (previousHash && previousHash !== hash) {
              isFading = true;
              setTimeout(() => {
                isFading = false;
              }, 150);
            }
          }
        }
      );
      cleanup.push(unsubscribeCompile);

      const unsubscribeToggleVisibility = await appWindow.listen<never>(
        "toggle_preview_visibility",
        () => {
          isVisible = !isVisible;
        }
      );
      cleanup.push(unsubscribeToggleVisibility);
    })();

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      cleanup.forEach((fn) => fn());
      window.removeEventListener("keydown", handleKeyDown);
    };
  });
</script>

{#if isVisible}
  {#if $shell.previewState === PreviewState.CompileError && currentErrors.length > 0}
    <CompileErrorDisplay errors={currentErrors} onErrorClick={handleErrorClick} />
  {:else}
    <div
      bind:this={container}
      on:mousemove={handleMove}
      on:mousedown={handleMouseDown}
      on:mouseup={handleMouseUp}
      on:mouseleave={handleMouseUp}
      on:wheel={handleWheel}
      class="preview-container"
      class:dragging={isDragging}
      role="region"
      aria-label="Document preview"
    >
      <div
        bind:this={pagesContainer}
        class="pages-wrapper"
        class:fading={isFading}
      >
        {#each Array(pages) as _, i}
          {#if hash}
            <PreviewPage
              page={i}
              hash={hash}
              width={Math.floor(width * zoom)}
              height={Math.floor(height * zoom)}
              scale={zoom}
            />
          {/if}
        {/each}
      </div>
      <ZoomControls
        {zoom}
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onZoomReset={handleZoomReset}
      />
    </div>
  {/if}
{/if}

<style>
  .preview-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: auto;
    background: var(--color-bg-secondary);
    padding: var(--space-xl);
    cursor: default;
    position: relative;
    touch-action: pan-x pan-y;
  }

  .preview-container.dragging {
    cursor: grabbing;
  }

  .pages-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    transition: opacity 150ms ease;
  }

  .pages-wrapper.fading {
    opacity: 0.7;
  }
</style>
