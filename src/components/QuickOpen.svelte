<script lang="ts">
  import { onMount } from "svelte";
  import { shell, project } from "../lib/stores";
  import { searchFiles } from "$lib/ipc";
  import fuzzysort from "fuzzysort";
  import { FileText, MagnifyingGlass } from "phosphor-svelte";
  import { fade } from "svelte/transition";

  let query = "";
  let files: string[] = [];
  let results: fuzzysort.Result[] = [];
  let selectedIndex = 0;
  let inputElement: HTMLInputElement;

  $: {
    if (query.trim() === "") {
      results = files.slice(0, 10).map((f) => ({ target: f, score: 0 } as any));
    } else {
      results = fuzzysort.go(query, files, { limit: 10 });
    }
    selectedIndex = 0;
  }

  onMount(async () => {
    if ($project) {
      files = await searchFiles();
    }
    inputElement.focus();
  });

  const close = () => {
    shell.popModal();
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (results.length === 0) return;

    if (e.key === "Escape") {
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % results.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + results.length) % results.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (results[selectedIndex]) {
        openFile(results[selectedIndex].target);
      }
    }
  };

  const openFile = (filePath: string) => {
    shell.openFile(filePath);
    close();
  };
</script>

<div
  class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] bg-black/5 backdrop-blur-[1px]"
  on:mousedown|self={close}
  transition:fade={{ duration: 100 }}
>
  <div
    class="w-[600px] max-w-[90vw] card flex flex-col overflow-hidden shadow-lg animate-fade-scale-in"
    on:mousedown|stopPropagation
  >
    <div class="flex items-center px-4 py-3 gap-3 border-b border-border">
      <MagnifyingGlass size={18} class="text-text-tertiary" />
      <input
        bind:this={inputElement}
        bind:value={query}
        on:keydown={handleKeydown}
        placeholder="Search files..."
        class="flex-1 bg-transparent text-[14px] outline-none placeholder:text-text-placeholder"
      />
      <div class="text-[10px] font-medium text-text-tertiary uppercase tracking-wider px-1.5 py-0.5 rounded border border-border/50">
        Esc to close
      </div>
    </div>

    <div class="max-h-[360px] overflow-y-auto py-1">
      {#if results.length === 0}
        <div class="px-4 py-8 text-center text-text-tertiary text-sm">
          No files found
        </div>
      {:else}
        {#each results as result, i}
          <button
            class="w-full flex items-center gap-3 px-4 py-2 text-left transition-colors duration-75 {i === selectedIndex ? 'bg-[var(--color-bg-selected)]' : 'hover:bg-[var(--color-bg-hover)]'}"
            on:click={() => openFile(result.target)}
            on:mouseenter={() => (selectedIndex = i)}
          >
            <FileText size={16} class="text-text-secondary" />
            <div class="flex flex-col min-w-0">
              <span class="text-sm truncate font-medium text-text-primary">
                {result.target.split('/').pop()}
              </span>
              <span class="text-[11px] truncate text-text-tertiary">
                {result.target}
              </span>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .animate-fade-scale-in {
    animation: fadeScaleIn 0.15s ease-out;
  }

  @keyframes fadeScaleIn {
    from {
      opacity: 0;
      transform: scale(0.98) translateY(-4px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }
</style>
