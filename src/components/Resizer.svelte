<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let direction: "horizontal" | "vertical" = "horizontal";
  export let minSize = 150;
  export let maxSize = 600;

  const dispatch = createEventDispatcher<{ resize: number; resizeend: void }>();

  let isDragging = false;
  let startPosition = 0;
  let startSize = 0;

  export let size = 240;

  const handleMouseDown = (event: MouseEvent) => {
    isDragging = true;
    startPosition = direction === "horizontal" ? event.clientX : event.clientY;
    startSize = size;
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = direction === "horizontal" ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";
  };

  const handleMouseMove = (event: MouseEvent) => {
    if (!isDragging) return;
    const currentPosition = direction === "horizontal" ? event.clientX : event.clientY;
    const delta = currentPosition - startPosition;
    const newSize = Math.min(maxSize, Math.max(minSize, startSize + delta));
    size = newSize;
    dispatch("resize", newSize);
  };

  const handleMouseUp = () => {
    isDragging = false;
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    dispatch("resizeend");
  };
</script>

<button
  class="resizer"
  class:horizontal={direction === "horizontal"}
  class:vertical={direction === "vertical"}
  class:dragging={isDragging}
  on:mousedown={handleMouseDown}
  aria-label="Resize pane"
></button>

<style>
  .resizer {
    flex-shrink: 0;
    background: transparent;
    transition: background var(--transition-fast);
    position: relative;
    z-index: 1000;
  }

  .resizer::after {
    content: "";
    position: absolute;
    background: var(--color-accent);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .resizer.horizontal {
    width: 16px;
    cursor: col-resize;
    margin: 0 -8px;
  }

  .resizer.horizontal::after {
    top: 0;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 2px;
  }

  .resizer.vertical {
    height: 16px;
    cursor: row-resize;
    margin: -8px 0;
  }

  .resizer.vertical::after {
    left: 0;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 2px;
  }

  .resizer:hover::after,
  .resizer.dragging::after {
    opacity: 1;
  }

  .resizer.dragging {
    background: rgba(35, 131, 226, 0.1);
  }
</style>
