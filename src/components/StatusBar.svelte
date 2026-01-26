<script lang="ts">
  import { PreviewState, shell } from "$lib/stores";
  import { XCircle, CircleNotch } from "$lib/icons";
</script>

<div
  class="flex flex-row items-center h-7 border-t border-neutral-700 text-neutral-200 font-medium text-sm"
>
  <div class="flex-1"></div>
  {#if $shell.previewState === PreviewState.CompileError}
    <div class="flex flex-row items-center px-2 bg-red-500 h-full gap-2">
      <XCircle class="w-4 h-4" />
      Compile error
    </div>
  {:else if $shell.previewState === PreviewState.Compiling}
    <div class="flex flex-row items-center px-2 h-full gap-2">
      <CircleNotch class="w-4 h-4 animate-spin" />
      Compiling...
    </div>
  {/if}
</div>
