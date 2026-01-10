<script lang="ts">
  import type { TypstSourceDiagnostic } from "$lib/ipc";
  import { WarningCircle } from "$lib/icons";

  export let errors: TypstSourceDiagnostic[] = [];
  export let onErrorClick: (error: TypstSourceDiagnostic) => void = () => {};
</script>

<div class="error-display">
  <div class="error-header">
    <WarningCircle size={20} weight="duotone" class="error-icon" />
    <span class="error-title">Compilation Error</span>
  </div>

  <div class="error-list">
    {#each errors as error}
      <button class="error-item" on:click={() => onErrorClick(error)}>
        <div class="error-severity" class:warning={error.severity === "warning"}>
          {error.severity}
        </div>
        <div class="error-content">
          <pre class="error-message">{error.message}</pre>
          {#if error.hints.length > 0}
            <div class="error-hints">
              {#each error.hints as hint}
                <span class="hint">Hint: {hint}</span>
              {/each}
            </div>
          {/if}
        </div>
      </button>
    {/each}
  </div>
</div>

<style>
  .error-display {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-secondary);
    overflow: hidden;
  }

  .error-header {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-primary);
  }

  .error-header :global(.error-icon) {
    color: var(--color-error);
  }

  .error-title {
    font-weight: 600;
    font-size: 14px;
    color: var(--color-text-primary);
  }

  .error-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .error-item {
    display: flex;
    gap: var(--space-md);
    padding: var(--space-md);
    background: var(--color-bg-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    text-align: left;
    transition: all var(--transition-fast);
    cursor: pointer;
  }

  .error-item:hover {
    border-color: var(--color-border-strong);
    box-shadow: var(--shadow-sm);
  }

  .error-severity {
    padding: 2px 8px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-radius: var(--radius-sm);
    background: var(--color-error-bg);
    color: var(--color-error);
    flex-shrink: 0;
    height: fit-content;
  }

  .error-severity.warning {
    background: rgba(242, 153, 74, 0.1);
    color: var(--color-warning);
  }

  .error-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .error-message {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-text-primary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .error-hints {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .hint {
    font-size: 12px;
    color: var(--color-text-secondary);
    font-style: italic;
  }
</style>
