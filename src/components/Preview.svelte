<script lang="ts">
  import PreviewPage from "./PreviewPage.svelte";
  import CompileErrorDisplay from "./CompileErrorDisplay.svelte";
  import ZoomControls from "./ZoomControls.svelte";
  import { onMount } from "svelte";

  import type { TypstCompileEvent, TypstSourceDiagnostic } from "../lib/ipc";
  import { jump } from "../lib/ipc";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  const appWindow = getCurrentWindow();
  import { shell, PreviewState } from "$lib/stores";


  let container: HTMLDivElement;
  let pagesContainer: HTMLDivElement;
  let previousEvent: MouseEvent | undefined;
  let flashMarker: { x: number; y: number } | null = null;

  let isDragging = false;
  let mouseDownPosition: { x: number; y: number; time: number } | null = null;
  let zoom = 1.0;
  let minZoom = 0.25;
  let maxZoom = 4.0;

  const CLICK_THRESHOLD_DISTANCE = 5;
  const CLICK_THRESHOLD_TIME = 200;

  let pages = 0;
  let hash: string | null = null;
  let width: number = 0;
  let height: number = 0;
  let containerWidth: number = 0;
  let currentErrors: TypstSourceDiagnostic[] = [];

  $: padding = 48; // var(--space-xl) * 2 roughly
  $: effectiveScale = width > 0 && containerWidth > 0 
    ? ((containerWidth - padding) / width) * zoom 
    : zoom;

  let isVisible: boolean = true;

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

  const handlePreviewClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement;
    const pageElement = target.closest(".preview-page");
    if (!pageElement) return;

    const pageIndex = parseInt(pageElement.getAttribute("data-page") || "0", 10);
    const rect = pageElement.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    const ptX = x / effectiveScale;
    const ptY = y / effectiveScale;

    const result = await jump(pageIndex, ptX, ptY);
    if (result && result.start) {
      console.log("Jump Target Received (Preview -> Editor):", {
        file: result.filepath,
        line: result.start[0],
        column: result.start[1],
        offset: result.offset,
        kind: result.node_kind,
        context: result.text?.trim()
      });
      appWindow.emit("editor_goto_location", result);
    }
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
          console.log("[Preview] typst_compile event received", payload);

          const { document, diagnostics } = payload;
          console.log("[Preview] Destructured:", { document, diagnostics });

          currentErrors = diagnostics || [];
          shell.setCurrentErrors(currentErrors);

          if (document) {
            pages = document.pages;
            hash = document.hash;
            width = document.width;
            height = document.height;
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

      const unsubscribeScrollToPos = await appWindow.listen<{ page: number; x: number; y: number; flash?: boolean }>(
        "scroll_to_position_in_preview",
        ({ payload }) => {
          if (!container) return;
          const pageElement = container.querySelector(`.preview-page[data-page="${payload.page}"]`);
          if (pageElement) {
            const pageRect = pageElement.getBoundingClientRect();
            const containerRect = container.getBoundingClientRect();

            const pixelX = payload.x * effectiveScale;
            const pixelY = payload.y * effectiveScale;

            const absolutePageTop = pageRect.top - containerRect.top + container.scrollTop;
            const absolutePageLeft = pageRect.left - containerRect.left + container.scrollLeft;

            const targetTop = absolutePageTop + pixelY - container.clientHeight / 2;
            const targetLeft = absolutePageLeft + pixelX - container.clientWidth / 2;

            container.scrollTo({ top: targetTop, left: targetLeft, behavior: "smooth" });

            if (pagesContainer && payload.flash !== false) {
              const pagesRect = pagesContainer.getBoundingClientRect();
              
              flashMarker = {
                x: pageRect.left - pagesRect.left + pixelX,
                y: pageRect.top - pagesRect.top + pixelY,
              };
            }
          }
        }
      );
      cleanup.push(unsubscribeScrollToPos);
    })();

    const resizeObserver = new ResizeObserver((entries) => {
      for (let entry of entries) {
        containerWidth = entry.contentRect.width;
      }
    });

    if (container) {
      resizeObserver.observe(container);
    }

    return () => {
      cleanup.forEach((fn) => fn());
      window.removeEventListener("keydown", handleKeyDown);
      resizeObserver.disconnect();
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
      >
        {#each Array(pages) as _, i}
          {#if hash}
            <PreviewPage
              page={i}
              hash={hash}
              width={Math.floor(width * effectiveScale)}
              height={Math.floor(height * effectiveScale)}
              scale={effectiveScale}
            />
          {/if}
        {/each}
        {#if flashMarker}
          {#key flashMarker}
            <div
              class="flash-marker"
              style="top: {flashMarker.y}px; left: {flashMarker.x}px;"
              on:animationend={() => {
                flashMarker = null;
              }}
            ></div>
          {/key}
        {/if}
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
    position: relative;
  }

  .flash-marker {
    position: absolute;
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-bottom: 9px solid var(--color-accent);
    transform: rotate(-45deg);
    transform-origin: top left;
    
    pointer-events: none;
    animation: arrow-soft-fade 0.8s ease-out forwards;
    z-index: 100;
  }

  @keyframes arrow-soft-fade {
    0% {
      opacity: 0;
      transform: rotate(-45deg) translate(5px, 5px);
    }
    20% {
      opacity: 0.6;
      transform: rotate(-45deg) translate(0, 0);
    }
    80% {
      opacity: 0.6;
      transform: rotate(-45deg) translate(0, 0);
    }
    100% {
      opacity: 0;
      transform: rotate(-45deg) translate(0, 0);
    }
  }
</style>
