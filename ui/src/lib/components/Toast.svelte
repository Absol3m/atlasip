<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { CircleCheck, CircleX, TriangleAlert, Info, X } from 'lucide-svelte';
  import { analysisStore } from '$lib/stores/analysis.svelte';
  import type { ToastType } from '$lib/stores/analysis.svelte';

  const ICONS: Record<ToastType, typeof Info> = {
    info:    Info,
    success: CircleCheck,
    warning: TriangleAlert,
    error:   CircleX,
  };
</script>

<div class="toast-container" aria-live="polite" aria-atomic="false">
  {#each analysisStore.toasts as toast (toast.id)}
    {@const Icon = ICONS[toast.type]}
    <div
      class="toast toast--{toast.type}"
      role="alert"
      in:fly={{ y: 12, duration: 200 }}
      out:fade={{ duration: 150 }}
    >
      <Icon size={15} />
      <span class="toast-msg">{toast.message}</span>
      <button
        class="toast-close"
        aria-label="Dismiss"
        onclick={() => analysisStore.dismissToast(toast.id)}
      >
        <X size={13} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 9000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 380px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.45;
    border: 1px solid transparent;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
    pointer-events: all;
    background: var(--color-surface);
  }

  .toast--info {
    border-color: var(--color-border);
    color: var(--color-text);
  }

  .toast--success {
    background: color-mix(in srgb, var(--color-success) 12%, var(--color-surface));
    border-color: var(--color-success);
    color: var(--color-success);
  }

  .toast--warning {
    background: color-mix(in srgb, #f59e0b 12%, var(--color-surface));
    border-color: #f59e0b;
    color: #b45309;
  }

  :global([data-theme='dark']) .toast--warning {
    color: #fbbf24;
  }

  .toast--error {
    background: color-mix(in srgb, var(--color-error) 12%, var(--color-surface));
    border-color: var(--color-error);
    color: var(--color-error);
  }

  .toast-msg {
    flex: 1;
  }

  .toast-close {
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    opacity: 0.6;
    padding: 0;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }

  .toast-close:hover {
    opacity: 1;
  }
</style>
