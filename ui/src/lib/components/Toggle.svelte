<script lang="ts">
  interface Props {
    checked: boolean;
    id?: string;
    ariaLabel: string;
  }

  let { checked = $bindable(), id, ariaLabel }: Props = $props();
</script>

<button
  {id}
  class="toggle"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={ariaLabel}
  onclick={() => { checked = !checked; }}
>
  <span class="toggle-thumb"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: var(--radius-full);
    border: none;
    background: var(--color-border);
    cursor: pointer;
    transition: background-color var(--transition-base);
    flex-shrink: 0;
    padding: 0;
  }

  .toggle.on {
    background: var(--color-accent);
  }

  .toggle-thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    transition: transform var(--transition-base);
    display: block;
  }

  .toggle.on .toggle-thumb {
    transform: translateX(16px);
  }
</style>
