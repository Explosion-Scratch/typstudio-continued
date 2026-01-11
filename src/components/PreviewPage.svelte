<script lang="ts">
  import type { TypstRenderResponse } from "../lib/ipc";
  import { render } from "../lib/ipc";
  import { onMount } from "svelte";

  export let page: number;
  export let hash: string;
  export let width: number;
  export let height: number;
  export let scale: number;

  let container: HTMLDivElement;
  let canRender = false;
  let isIntersecting = false;
  let nonce = 1;
  let lastNonce = 0;

  onMount(() => {
    const observer = new IntersectionObserver((entries) => {
      isIntersecting = entries[0].isIntersecting;
      if (isIntersecting) canRender = true;
    });
    observer.observe(container);
    return () => observer.disconnect();
  });

  const invalidateCanRender = (_hash: string, _scale: number) => {
    canRender = isIntersecting;
  };

  const update = async (updateHash: string, updateScale: number) => {
    const res: TypstRenderResponse = await render(page, updateScale, nonce++);

    if (res.nonce > lastNonce) {
      lastNonce = res.nonce;
      container.innerHTML = res.image;
      
      const svgEl = container.querySelector("svg");
      if (svgEl) {
        svgEl.style.width = "100%";
        svgEl.style.height = "100%";
        svgEl.style.display = "block";
      }
    }
  };

  $: invalidateCanRender(hash, scale);
  $: if (canRender) update(hash, scale);
</script>
<div
  class="preview-page"
  style="height: {height}px; min-height: {height}px; width: {width}px; min-width: {width}px; --height: {height}px;"
  bind:this={container}
  data-page={page}
></div>

<style>
  .preview-page {
    background: white;
    box-shadow: var(--shadow-md);
    border-radius: 2px;
    margin: 0 auto;
    box-sizing: border-box;
    overflow: hidden;
    content-visibility: auto;
    contain-intrinsic-size: auto var(--height);
  }

  .preview-page :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>