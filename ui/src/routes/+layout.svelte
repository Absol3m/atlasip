<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from '$lib/stores/app';
  import { i18n } from '$lib/services/i18n.svelte';
  import { historyStore } from '$lib/stores/history.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toast from '$lib/components/Toast.svelte';

  let { children } = $props();

  onMount(() => {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.init(prefersDark ? 'dark' : 'light');
    i18n.init();
    historyStore.init();
  });
</script>

<svelte:head>
  <title>AtlasIP</title>
</svelte:head>

<div class="app" data-theme={$theme}>
  <Sidebar />
  <div class="workspace">
    {@render children()}
  </div>
  <Toast />
</div>

<style>
  /* ── Light theme ── */
  :global([data-theme='light']) {
    --color-bg:           #f3f4f6;
    --color-surface:      #ffffff;
    --color-nav-bg:       rgba(255, 255, 255, 0.85);
    --color-text:         #111827;
    --color-text-muted:   #6b7280;
    --color-accent:       #4361ee;
    --color-accent-hover: #3a56d4;
    --color-border:       #e5e7eb;
    --color-hover:        #f3f4f6;
    --color-success:      #16a34a;
    --color-error:        #dc2626;
    --color-card-bg:      #ffffff;
    --color-header-bg:    #f9fafb;
    --color-row-even:     #f9fafb;
    --color-row-hover:    #eff6ff;
    --color-row-selected: #dbeafe;
    --shadow:             0 1px 4px rgba(0,0,0,0.07), 0 4px 16px rgba(0,0,0,0.04);
  }

  /* ── Dark theme ── */
  :global([data-theme='dark']) {
    --color-bg:           #0d1117;
    --color-surface:      #161b22;
    --color-nav-bg:       rgba(22, 27, 34, 0.9);
    /* P3-UI-001: increased contrast for readability */
    --color-text:         #e6edf3;
    --color-text-muted:   #a0aab4;
    --color-accent:       #58a6ff;
    --color-accent-hover: #79b8ff;
    --color-border:       #30363d;
    --color-hover:        #21262d;
    --color-success:      #3fb950;
    --color-error:        #f85149;
    --color-card-bg:      #161b22;
    --color-header-bg:    #0d1117;
    --color-row-even:     #0d1117;
    --color-row-hover:    #1c2128;
    --color-row-selected: #1a2d4d;
    --shadow:             0 2px 8px rgba(0,0,0,0.4);
  }

  /* ── P1-DARKMODE-001: visible scrollbars in dark mode ── */
  :global([data-theme='dark']) {
    scrollbar-color: #484f58 #0d1117;
    scrollbar-width: thin;
  }

  :global([data-theme='dark'] ::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }

  :global([data-theme='dark'] ::-webkit-scrollbar-track) {
    background: #0d1117;
  }

  :global([data-theme='dark'] ::-webkit-scrollbar-thumb) {
    background: #484f58;
    border-radius: 4px;
    border: 2px solid #0d1117;
  }

  :global([data-theme='dark'] ::-webkit-scrollbar-thumb:hover) {
    background: #6e7681;
  }

  :global([data-theme='dark'] ::-webkit-scrollbar-corner) {
    background: #0d1117;
  }

  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: Inter, system-ui, -apple-system, sans-serif;
    font-size: 15px;
    line-height: 1.6;
    background-color: var(--color-bg);
    color: var(--color-text);
  }

  .app {
    height: 100vh;
    display: flex;
    overflow: hidden;
    background-color: var(--color-bg);
  }

  /* The scrollable main area — each page controls its own content. */
  .workspace {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
</style>
