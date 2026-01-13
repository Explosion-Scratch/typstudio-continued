<script lang="ts">
  import type { TypstRenderResponse } from "../lib/ipc";
  import { render } from "../lib/ipc";
  import { onMount } from "svelte";
  import { CircleNotch } from "../lib/icons";
  import { fade } from "svelte/transition";
  import { patchSvgToContainer } from "../lib/typst-patch";

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
  let loading = false;
  let showLoading = false;
  let loadingTimer: any;

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

  const decorateSvg = (svgEl: SVGElement) => {
    svgEl.style.width = "100%";
    svgEl.style.height = "100%";
    svgEl.style.display = "block";
  };

  const update = async (updateHash: string, updateScale: number) => {
    loading = true;
    clearTimeout(loadingTimer);
    loadingTimer = setTimeout(() => {
      if (loading) showLoading = true;
    }, 1000);

    try {
      const res: TypstRenderResponse = await render(page, updateScale, nonce++);

      if (res.nonce > lastNonce) {
        lastNonce = res.nonce;
        patchSvgToContainer(container, res.image, decorateSvg);
      }
    } finally {
      loading = false;
      showLoading = false;
      clearTimeout(loadingTimer);
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
>
  {#if showLoading}
    <div class="page-loading" transition:fade={{ duration: 150 }}>
      <CircleNotch size={24} class="spinner" weight="bold" />
    </div>
  {/if}
</div>

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
  
  .page-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.5);
    z-index: 5;
  }
  
  .page-loading :global(.spinner) {
    animation: spin 1s linear infinite;
    color: var(--color-text-secondary);
  }
  
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>