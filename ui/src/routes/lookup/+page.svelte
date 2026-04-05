<script lang="ts">
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { goto } from '$app/navigation';
  import { i18n } from '$lib/services/i18n.svelte';
  import { analysisStore } from '$lib/stores/analysis.svelte';
  import { resultsStore } from '$lib/stores/results.svelte';
  import { lookupIPs } from '$lib/services/lookup';
  import AnalysisInput from '$lib/components/AnalysisInput.svelte';
  import ResultsTable from '$lib/components/ResultsTable.svelte';

  // P2-UX-007: keep a ref to the textarea for focus restore after analysis.
  let textareaRef = $state<HTMLTextAreaElement | undefined>(undefined);

  // ── Single-result redirect ────────────────────────────────────────────────
  // When analyze() finds exactly one result, we navigate to the detail page
  // without ever rendering the table.  This flag is local to the component
  // instance: it resets to false whenever the user navigates back (SvelteKit
  // destroys and recreates the page component on route change), so returning
  // from the detail page shows the table normally.
  let suppressTable = $state(false);

  // ── Auto-purge ───────────────────────────────────────────────────────────
  // Generation counter: incremented each time the input is cleared so that any
  // in-flight analysis resolve() can detect it is stale and discard its results.
  let analysisGeneration = 0;

  $effect(() => {
    // Track rawInput reactively; act only when it becomes fully empty.
    if (analysisStore.rawInput === '') {
      analysisGeneration++;                // invalidate any in-flight lookup
      resultsStore.setResults([]);         // clear the table immediately
      resultsStore.loading = false;        // stop any lingering spinner
      resultsStore.error   = null;         // dismiss error state
      suppressTable        = false;        // reset redirect guard
      // Status + parseResult already reset by analysisStore.clear() / scheduleparse.
      // Focus stays on the textarea — the user is still interacting with it.
    }
  });

  async function analyze() {
    const targets = analysisStore.parseResult?.validTargets ?? [];
    if (targets.length === 0) {
      analysisStore.addToast('warning', i18n.t('error.no_valid_targets'));
      return;
    }

    // Capture the current generation so we can detect stale results later.
    const generation = ++analysisGeneration;

    analysisStore.setStatus('loading');
    resultsStore.loading = true;
    resultsStore.error   = null;
    suppressTable        = false;

    try {
      const records = await lookupIPs(targets);

      // Discard results if the input was cleared while the request was in flight.
      if (generation !== analysisGeneration) return;

      // Populate the store so the detail page can find the record without a
      // second API call (resultsStore.results is read by /ip/[ip]/+page.svelte).
      resultsStore.setResults(records);

      if (records.length === 1) {
        // Single result: suppress the table and navigate to the detail page.
        // suppressTable is set BEFORE goto() so the template never renders the
        // table even during the microtask gap between setResults() and navigation.
        suppressTable = true;
        analysisStore.setStatus('success');
        setTimeout(() => analysisStore.setStatus('idle'), 1500);
        await goto(`/ip/${records[0].ip}`);
        return;
      }

      analysisStore.setStatus('success');

      // Flash success → back to idle after 1.5 s
      setTimeout(() => analysisStore.setStatus('idle'), 1500);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      resultsStore.error = msg;
      analysisStore.setStatus('error');

      const isNetwork =
        msg.includes('fetch') ||
        msg.includes('NetworkError') ||
        msg.includes('Failed to fetch');
      analysisStore.addToast(
        'error',
        isNetwork
          ? i18n.t('error.backend_unreachable')
          : i18n.t('error.lookup_failed'),
      );

      setTimeout(() => analysisStore.setStatus('idle'), 2500);
    } finally {
      resultsStore.loading = false;

      // P2-UX-007: restore focus to textarea after any analysis outcome.
      await tick();
      textareaRef?.focus();
    }
  }
</script>

<!-- ── Analysis Page ─────────────────────────────────────────────────────── -->
<div class="analysis-page">

  <!-- Sticky top: page header + input -->
  <div class="sticky-top">
    <header class="page-header">
      <div class="header-left">
        <h1 class="page-title">{i18n.t('analysis.title')}</h1>
        <p class="page-subtitle">{i18n.t('analysis.subtitle')}</p>
      </div>
    </header>

    <div class="input-section">
      <AnalysisInput onanalyze={analyze} bind:textareaRef />
    </div>
  </div>

  <!-- Results area (P2-LAYOUT-001 — overflow:hidden so grid controls its own scroll) -->
  <div class="results-section">
    {#if resultsStore.loading}
      <div class="results-loading" aria-live="polite" aria-busy="true">
        <span class="spinner-lg" aria-hidden="true"></span>
        <span>{i18n.t('analysis.btn.analyzing')}</span>
      </div>

    {:else if resultsStore.results.length > 0 && !suppressTable}
      <!-- P3-UI-002: fade-in animation when DataGrid appears -->
      <div class="results-wrap" transition:fade={{ duration: 180 }}>
        <ResultsTable />
      </div>

    {:else}
      <!-- P0-DATAGRID-002: empty state always shown, including after an error -->
      <div class="results-empty">
        {#if resultsStore.error}
          <div class="empty-icon" aria-hidden="true">⚠</div>
          <p class="empty-text">{resultsStore.error}</p>
        {:else}
          <div class="empty-icon" aria-hidden="true">⬡</div>
          <p class="empty-text">{i18n.t('analysis.empty')}</p>
        {/if}
      </div>
    {/if}
  </div>

</div>

<style>
  .analysis-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  /* ── Sticky top (header + input) ── */
  .sticky-top {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--color-bg);
    border-bottom: 1px solid var(--color-border);
    z-index: 10;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 24px 8px;
    min-height: 56px;
  }

  .header-left {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .page-title {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--color-text);
    line-height: 1.2;
  }

  .page-subtitle {
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .input-section {
    padding: 0 20px 12px;
  }

  /* ── Results section (P2-LAYOUT-001) ── */
  /* overflow:hidden lets the inner grid fill the space and scroll on its own. */
  .results-section {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding: 14px 20px;
  }

  /* ResultsTable wrapper — must grow to fill available height */
  .results-wrap {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ── Loading ── */
  .results-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .spinner-lg {
    width: 20px;
    height: 20px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ── Empty / error state ── */
  .results-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 10px;
    color: var(--color-text-muted);
    user-select: none;
  }

  .empty-icon {
    font-size: 2.5rem;
    opacity: 0.2;
    color: var(--color-accent);
    line-height: 1;
  }

  .empty-text {
    font-size: 13.5px;
    opacity: 0.75;
    text-align: center;
    max-width: 360px;
  }
</style>
