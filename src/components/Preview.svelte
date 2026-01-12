<script lang="ts">
  import PreviewPage from "./PreviewPage.svelte";
  import CompileErrorDisplay from "./CompileErrorDisplay.svelte";
  import ZoomControls from "./ZoomControls.svelte";
  import { calculatePreviewScrollCenter, getPreviewToEditorTarget, getPreview3Positions } from "$lib/scroll";
  import { onMount, tick } from "svelte";
  import type { TypstCompileEvent, TypstSourceDiagnostic } from "../lib/ipc";
  import { jump } from "../lib/ipc";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { shell, PreviewState, pendingScroll } from "$lib/stores";
  import { debounce } from "lodash";

  const appWindow = getCurrentWindow();
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

  $: padding = 48;
  $: effectiveScale = width > 0 && containerWidth > 0 
    ? ((containerWidth - padding) / width) * zoom 
    : zoom;

  let isVisible: boolean = true;
  let hasRestoredInitialScroll = false;

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
  
  const handleScroll = debounce(async (event: Event) => {
    if (!container || !pagesContainer) return;

    const positions = getPreview3Positions(container, pagesContainer, effectiveScale);
    const centerPos = positions[1];

    if ($shell.viewMode === "preview") {
      const target = await getPreviewToEditorTarget(container, pagesContainer, effectiveScale);
      
      pendingScroll.update(p => ({
        ...p,
        source: 'preview',
        line: target?.line ?? p.line,
        filepath: target?.filepath ?? p.filepath,
        preview: centerPos ?? p.preview
      }));
    } else {
      pendingScroll.update(p => ({
        ...p,
        preview: centerPos ?? p.preview
      }));
    }
  }, 200);

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

  const handleZoomIn = () => zoom = Math.min(maxZoom, zoom * 1.2);
  const handleZoomOut = () => zoom = Math.max(minZoom, zoom / 1.2);
  const handleZoomReset = () => zoom = 1.0;

  const handleErrorClick = (error: TypstSourceDiagnostic) => {
    appWindow.emit("scroll_to_position", { position: error.range.start });
  };

  const scrollToPreviewPosition = (payload: { page: number; x: number; y: number; flash?: boolean }) => {
    if (!container || !pagesContainer) return;

    const pageElement = container.querySelector(`.preview-page[data-page="${payload.page}"]`) as HTMLElement;
    if (pageElement) {
      const target = calculatePreviewScrollCenter(container, pageElement, payload, effectiveScale);
      
      if (target) {
        container.scrollTo({ top: target.top, left: target.left, behavior: "smooth" });
        
        if (payload.flash !== false) {
          const pageRect = pageElement.getBoundingClientRect();
          const pagesRect = pagesContainer.getBoundingClientRect();
          
          flashMarker = {
            x: pageRect.left - pagesRect.left + (payload.x * effectiveScale),
            y: pageRect.top - pagesRect.top + (payload.y * effectiveScale),
          };
        }
      }
    }
  };

  $: if (isVisible && container && pages > 0) {
    (async () => {
      const pending = $pendingScroll;
      if (pending.source === 'editor' && pending.preview) {
        await tick();
        scrollToPreviewPosition({ ...pending.preview, flash: false });
        pendingScroll.update(p => ({ ...p, source: null }));
        hasRestoredInitialScroll = true;
      } else if (!hasRestoredInitialScroll && pending.preview) {
        await tick();
        scrollToPreviewPosition({ ...pending.preview, flash: false });
        hasRestoredInitialScroll = true;
      }
    })();
  }

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
        () => isVisible = !isVisible
      );
      cleanup.push(unsubscribeToggleVisibility);

      const unsubscribeScrollToPos = await appWindow.listen<{ page: number; x: number; y: number; flash?: boolean }>(
        "scroll_to_position_in_preview",
        ({ payload }) => scrollToPreviewPosition(payload)
      );
      cleanup.push(unsubscribeScrollToPos);
    })();

    const resizeObserver = new ResizeObserver((entries) => {
      for (let entry of entries) {
        containerWidth = entry.contentRect.width;
      }
    });

    if (container) resizeObserver.observe(container);

    return () => {
      cleanup.forEach((fn) => fn());
      window.removeEventListener("keydown", handleKeyDown);
      resizeObserver.disconnect();
      handleScroll.cancel();
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
      on:scroll={handleScroll}
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
    background: var(--color-bg-preview);
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
