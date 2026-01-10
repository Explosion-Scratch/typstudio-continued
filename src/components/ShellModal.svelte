<script lang="ts">
  import type { Modal } from "../lib/stores";
  import { shell } from "../lib/stores";
  import { X } from "$lib/icons";
  import { fade, scale } from "svelte/transition";

  let modal: Modal | undefined;
  $: modal = $shell.modals[0];

  let inputValue = "";

  const handleClose = (cancel: boolean = true) => {
    if (cancel && modal?.type === "input") {
      modal.callback(null);
    }
    if (cancel && modal?.type === "confirm" && modal.onCancel) {
      modal.onCancel();
    }
    inputValue = "";
    shell.popModal();
  };

  const handleInputSubmit = () => {
    if (modal?.type === "input") {
      modal.callback(inputValue);
      handleClose(false);
    }
  };

  const handleConfirm = () => {
    if (modal?.type === "confirm") {
      modal.onConfirm();
      handleClose(false);
    }
  };

  const handleInputKeyUp = (event: KeyboardEvent) => {
    switch (event.key) {
      case "Enter":
        handleInputSubmit();
        break;
      case "Escape":
        handleClose();
        break;
    }
  };

  const handleBackdropClick = (event: MouseEvent) => {
    if (event.target === event.currentTarget) {
      handleClose();
    }
  };
</script>

{#if modal}
  <div
    class="modal-backdrop"
    on:click={handleBackdropClick}
    on:keydown={(e) => e.key === "Escape" && handleClose()}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    transition:fade={{ duration: 150 }}
  >
    <div
      class="modal-container"
      transition:scale={{ duration: 150, start: 0.98 }}
    >
      <div class="modal-header">
        <h2 class="modal-title">{modal.title}</h2>
        <button class="close-button" on:click={() => handleClose()}>
          <X size={16} weight="bold" />
        </button>
      </div>

      <div class="modal-content">
        {#if modal.type === "input"}
          <input
            type="text"
            class="modal-input"
            placeholder={modal.placeholder || ""}
            bind:value={inputValue}
            on:keyup={handleInputKeyUp}
            autofocus
          />
        {:else if modal.type === "confirm"}
          <p class="modal-message">{modal.message}</p>
        {/if}
      </div>

      <div class="modal-footer">
        {#if modal.type === "input"}
          <button class="modal-button secondary" on:click={() => handleClose()}>
            Cancel
          </button>
          <button class="modal-button primary" on:click={handleInputSubmit}>
            Create
          </button>
        {:else if modal.type === "confirm"}
          <button class="modal-button secondary" on:click={() => handleClose()}>
            {modal.cancelLabel || "Cancel"}
          </button>
          <button class="modal-button primary" on:click={handleConfirm}>
            {modal.confirmLabel || "Confirm"}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    backdrop-filter: blur(2px);
  }

  .modal-container {
    background: var(--color-bg-primary);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    width: 100%;
    max-width: 400px;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
  }

  .modal-title {
    flex: 1;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    color: var(--color-text-tertiary);
    transition: all var(--transition-fast);
  }

  .close-button:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .modal-content {
    padding: var(--space-xl);
  }

  .modal-input {
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: 14px;
    color: var(--color-text-primary);
    transition: border-color var(--transition-fast);
  }

  .modal-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .modal-input::placeholder {
    color: var(--color-text-placeholder);
  }

  .modal-message {
    margin: 0;
    font-size: 14px;
    color: var(--color-text-secondary);
    line-height: 1.6;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-sm);
    padding: var(--space-md) var(--space-xl);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-tertiary);
  }

  .modal-button {
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 500;
    transition: all var(--transition-fast);
    cursor: pointer;
  }

  .modal-button.secondary {
    background: transparent;
    color: var(--color-text-secondary);
  }

  .modal-button.secondary:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }

  .modal-button.primary {
    background: var(--color-accent);
    color: white;
  }

  .modal-button.primary:hover {
    background: var(--color-accent-hover);
  }
</style>
