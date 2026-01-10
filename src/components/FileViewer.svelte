<script lang="ts">
  import { readFileBinary, readFileText } from "$lib/ipc";
  import ZoomControls from "./ZoomControls.svelte";

  export let path: string;

  let content: string | null = null;
  let binaryUrl: string | null = null;
  let isLoading = true;
  let error: string | null = null;

  let zoom = 1.0;
  let panX = 0;
  let panY = 0;
  let isDragging = false;
  let dragStart = { x: 0, y: 0 };
  let imageContainer: HTMLDivElement;

  const minZoom = 0.25;
  const maxZoom = 4.0;

  const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "svg", "ico"];
  const VIDEO_EXTENSIONS = ["mp4", "webm", "mov"];
  const AUDIO_EXTENSIONS = ["mp3", "wav", "ogg", "m4a"];
  const PDF_EXTENSIONS = ["pdf"];
  const TEXT_EXTENSIONS = ["txt", "json", "xml", "csv", "log"];

  $: extension = path.toLowerCase().split(".").pop() || "";
  $: fileType = getFileType(extension);

  type FileType = "image" | "video" | "audio" | "pdf" | "text" | "unsupported";

  function getFileType(ext: string): FileType {
    if (IMAGE_EXTENSIONS.includes(ext)) return "image";
    if (VIDEO_EXTENSIONS.includes(ext)) return "video";
    if (AUDIO_EXTENSIONS.includes(ext)) return "audio";
    if (PDF_EXTENSIONS.includes(ext)) return "pdf";
    if (TEXT_EXTENSIONS.includes(ext)) return "text";
    return "unsupported";
  }

  async function loadFile(filePath: string) {
    isLoading = true;
    error = null;
    content = null;
    zoom = 1.0;
    panX = 0;
    panY = 0;
    if (binaryUrl) {
      URL.revokeObjectURL(binaryUrl);
      binaryUrl = null;
    }

    try {
      const ext = filePath.toLowerCase().split(".").pop() || "";
      const type = getFileType(ext);

      if (type === "text") {
        content = await readFileText(filePath);
      } else if (type === "image" || type === "video" || type === "audio" || type === "pdf") {
        const bytes = await readFileBinary(filePath);
        const mimeTypes: Record<string, string> = {
          png: "image/png",
          jpg: "image/jpeg",
          jpeg: "image/jpeg",
          gif: "image/gif",
          webp: "image/webp",
          svg: "image/svg+xml",
          ico: "image/x-icon",
          mp4: "video/mp4",
          webm: "video/webm",
          mov: "video/quicktime",
          mp3: "audio/mpeg",
          wav: "audio/wav",
          ogg: "audio/ogg",
          m4a: "audio/mp4",
          pdf: "application/pdf",
        };
        const mimeType = mimeTypes[ext] || "application/octet-stream";
        const blob = new Blob([new Uint8Array(bytes)], { type: mimeType });
        binaryUrl = URL.createObjectURL(blob);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to load file";
    } finally {
      isLoading = false;
    }
  }

  const handleZoomIn = () => {
    zoom = Math.min(maxZoom, zoom * 1.2);
  };

  const handleZoomOut = () => {
    zoom = Math.max(minZoom, zoom / 1.2);
  };

  const handleZoomReset = () => {
    zoom = 1.0;
    panX = 0;
    panY = 0;
  };

  const handleWheel = (event: WheelEvent) => {
    if (event.ctrlKey || event.metaKey) {
      event.preventDefault();
      const delta = event.deltaY > 0 ? 0.9 : 1.1;
      zoom = Math.max(minZoom, Math.min(maxZoom, zoom * delta));
    }
  };

  const handleMouseDown = (event: MouseEvent) => {
    if (zoom > 1) {
      isDragging = true;
      dragStart = { x: event.clientX - panX, y: event.clientY - panY };
    }
  };

  const handleMouseMove = (event: MouseEvent) => {
    if (isDragging) {
      panX = event.clientX - dragStart.x;
      panY = event.clientY - dragStart.y;
    }
  };

  const handleMouseUp = () => {
    isDragging = false;
  };

  $: loadFile(path);
</script>

<div class="file-viewer">
  {#if isLoading}
    <div class="loading-state">
      <span class="spinner"></span>
      <span>Loading file...</span>
    </div>
  {:else if error}
    <div class="error-state">
      <span class="error-icon">⚠</span>
      <span>{error}</span>
    </div>
  {:else if fileType === "image" && binaryUrl}
    <div 
      bind:this={imageContainer}
      class="image-container"
      class:dragging={isDragging}
      class:zoomable={zoom > 1}
      on:wheel={handleWheel}
      on:mousedown={handleMouseDown}
      on:mousemove={handleMouseMove}
      on:mouseup={handleMouseUp}
      on:mouseleave={handleMouseUp}
      role="img"
      tabindex="0"
    >
      <img 
        src={binaryUrl} 
        alt={path.split("/").pop()} 
        style="transform: scale({zoom}) translate({panX / zoom}px, {panY / zoom}px)"
        draggable="false"
      />
      <ZoomControls
        {zoom}
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onZoomReset={handleZoomReset}
      />
    </div>
  {:else if fileType === "video" && binaryUrl}
    <div class="media-container">
      <video controls src={binaryUrl}>
        <track kind="captions" />
      </video>
    </div>
  {:else if fileType === "audio" && binaryUrl}
    <div class="media-container audio">
      <div class="audio-icon">🎵</div>
      <audio controls src={binaryUrl}></audio>
    </div>
  {:else if fileType === "pdf" && binaryUrl}
    <iframe src={binaryUrl} title="PDF viewer" class="pdf-viewer"></iframe>
  {:else if fileType === "text" && content !== null}
    <div class="text-content">
      <pre>{content}</pre>
    </div>
  {:else}
    <div class="unsupported-state">
      <div class="unsupported-icon">📄</div>
      <h3>Cannot Preview This File</h3>
      <p>The Typst editor can't render this file type.</p>
      <span class="file-extension">.{extension}</span>
    </div>
  {/if}
</div>

<style>
  .file-viewer {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-secondary);
    overflow: hidden;
    position: relative;
  }

  .loading-state,
  .error-state,
  .unsupported-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-md);
    color: var(--color-text-tertiary);
    text-align: center;
    padding: var(--space-xl);
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-state {
    color: var(--color-error);
  }

  .error-icon {
    font-size: 32px;
  }

  .unsupported-state h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .unsupported-state p {
    margin: 0;
    font-size: 14px;
  }

  .unsupported-icon {
    font-size: 48px;
    opacity: 0.5;
  }

  .file-extension {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--color-bg-hover);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
  }

  .image-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-xl);
    overflow: hidden;
    position: relative;
  }

  .image-container.zoomable {
    cursor: grab;
  }

  .image-container.dragging {
    cursor: grabbing;
  }

  .image-container img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    transition: transform 0.1s ease;
    user-select: none;
  }

  .media-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-xl);
  }

  .media-container video {
    max-width: 100%;
    max-height: 100%;
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }

  .media-container.audio {
    flex-direction: column;
    gap: var(--space-lg);
  }

  .audio-icon {
    font-size: 64px;
    opacity: 0.5;
  }

  .media-container audio {
    width: 100%;
    max-width: 400px;
  }

  .pdf-viewer {
    flex: 1;
    width: 100%;
    border: none;
  }

  .text-content {
    flex: 1;
    padding: var(--space-lg);
    overflow: auto;
  }

  .text-content pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-wrap: break-word;
  }
</style>
