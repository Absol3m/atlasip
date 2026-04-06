<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'primary' | 'ghost' | 'danger' | 'icon';
    type?: 'button' | 'submit';
    disabled?: boolean;
    ariaLabel?: string;
    title?: string;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }

  let {
    variant = 'primary',
    type = 'button',
    disabled = false,
    ariaLabel,
    title,
    onclick,
    children,
  }: Props = $props();
</script>

<button
  class="btn btn--{variant}"
  {type}
  {disabled}
  aria-label={ariaLabel}
  {title}
  {onclick}
>
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background-color var(--transition-fast),
      color var(--transition-fast),
      border-color var(--transition-fast),
      box-shadow var(--transition-fast);
    line-height: 1;
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  /* ── Primary ── */
  .btn--primary {
    background: var(--color-accent);
    color: #fff;
    box-shadow: 0 1px 4px color-mix(in srgb, var(--color-accent) 30%, transparent);
  }

  .btn--primary:hover:not(:disabled) {
    background: var(--color-accent-hover);
    box-shadow: 0 3px 8px color-mix(in srgb, var(--color-accent) 40%, transparent);
  }

  .btn--primary:active:not(:disabled) {
    box-shadow: 0 1px 3px color-mix(in srgb, var(--color-accent) 25%, transparent);
  }

  /* ── Ghost ── */
  .btn--ghost {
    background: transparent;
    color: var(--color-text-muted);
    border-color: var(--color-border);
  }

  .btn--ghost:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-text);
  }

  /* ── Danger ── */
  .btn--danger {
    background: transparent;
    color: var(--color-error);
    border-color: color-mix(in srgb, var(--color-error) 40%, transparent);
  }

  .btn--danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-error) 10%, transparent);
    border-color: var(--color-error);
  }

  /* ── Icon ── */
  .btn--icon {
    padding: 5px;
    background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
  }

  .btn--icon:hover:not(:disabled) {
    background: var(--color-hover);
    color: var(--color-text);
  }
</style>
