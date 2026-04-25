<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Clock, Trash2, ChevronRight, ChevronDown } from 'lucide-svelte';
  import { historyStore } from '$lib/stores/history.svelte';
  import { analysisStore } from '$lib/stores/analysis.svelte';
  import { i18n } from '$lib/services/i18n.svelte';

  onMount(() => {
    historyStore.init();
  });

  // ── Grouping — one card per unique set of targets ─────────────────────────

  interface Consultation {
    id:          string;
    timestamp:   number;
    resultCount: number;
  }

  interface GroupedEntry {
    key:           string;
    targets:       string[];
    lastTimestamp: number;
    consultations: Consultation[];
  }

  const grouped = $derived.by((): GroupedEntry[] => {
    const map = new Map<string, GroupedEntry>();

    for (const entry of historyStore.entries) {
      const key = entry.targets.join('\x00');
      if (!map.has(key)) {
        map.set(key, { key, targets: entry.targets, lastTimestamp: entry.timestamp, consultations: [] });
      }
      const group = map.get(key)!;
      group.consultations.push({ id: entry.id, timestamp: entry.timestamp, resultCount: entry.resultCount });
      if (entry.timestamp > group.lastTimestamp) group.lastTimestamp = entry.timestamp;
    }

    for (const g of map.values()) {
      g.consultations.sort((a, b) => b.timestamp - a.timestamp);
    }

    return [...map.values()].sort((a, b) => b.lastTimestamp - a.lastTimestamp);
  });

  // ── Expand state ──────────────────────────────────────────────────────────

  let expanded = $state(new Set<string>());

  function toggleGroup(key: string) {
    const next = new Set(expanded);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expanded = next;
  }

  // ── Actions ───────────────────────────────────────────────────────────────

  function rerun(targets: string[]) {
    analysisStore.setInput(targets.join('\n'));
    analysisStore.pendingTrigger = true;
    goto('/lookup');
  }

  // ── Formatting ────────────────────────────────────────────────────────────

  function formatDate(ts: number): string {
    return new Date(ts).toLocaleDateString(i18n.locale, {
      day: '2-digit', month: '2-digit', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<div class="history-page">
  <div class="page-header">
    <div class="header-left">
      <h1 class="page-title"><Clock size={16} /> {i18n.t('history.title')}</h1>
      <p class="page-sub">
        {i18n.tn('history.targets.one', 'history.targets.many', grouped.length)}
      </p>
    </div>
    {#if historyStore.entries.length > 0}
      <button class="clear-btn" onclick={() => historyStore.clear()} title={i18n.t('history.clear.title')}>
        <Trash2 size={14} /> {i18n.t('history.clear')}
      </button>
    {/if}
  </div>

  {#if grouped.length === 0}
    <div class="empty">
      <Clock size={40} strokeWidth={1.2} />
      <p>{i18n.t('history.empty')}</p>
    </div>

  {:else}
    <div class="list">
      {#each grouped as group (group.key)}
        {@const isExpanded = expanded.has(group.key)}
        {@const hasMany    = group.consultations.length > 1}
        {@const latest     = group.consultations[0]}

        <div class="group-card" role="button" tabindex="0"
          onclick={() => rerun(group.targets)}
          onkeydown={(e) => e.key === 'Enter' && rerun(group.targets)}
        >
          <!-- ── Target chips ── -->
          <div class="group-targets">
            {#each group.targets.slice(0, 5) as t}
              <span class="target-chip">{t}</span>
            {/each}
            {#if group.targets.length > 5}
              <span class="target-more">+{group.targets.length - 5}</span>
            {/if}
          </div>

          <!-- ── Disclosure line ── -->
          <div class="group-disclosure">
            {#if hasMany}
              <button class="chevron-btn"
                aria-label={isExpanded ? 'Collapse' : 'Expand'}
                onclick={(e) => { e.stopPropagation(); toggleGroup(group.key); }}
              >
                {#if isExpanded}
                  <ChevronDown size={12} />
                {:else}
                  <ChevronRight size={12} />
                {/if}
              </button>
              <span class="disclosure-text">
                {i18n.tn('history.consultations.one', 'history.consultations.many', group.consultations.length)}
                <span class="disclosure-sep">—</span>
                {i18n.t('history.last')} : {formatDate(latest.timestamp)}
              </span>
            {:else}
              <span class="disclosure-text">
                {formatDate(latest.timestamp)}
                <span class="disclosure-sep">·</span>
                {i18n.tn('history.result.one', 'history.result.many', latest.resultCount)}
              </span>
            {/if}
          </div>

          <!-- ── Older consultations (expanded) ── -->
          {#if isExpanded && hasMany}
            <div class="consultations">
              {#each group.consultations.slice(1) as c (c.id)}
                <div class="consultation">
                  <span class="c-dot">·</span>
                  <span class="c-date">{formatDate(c.timestamp)}</span>
                  <span class="c-results">
                    {i18n.tn('history.result.one', 'history.result.many', c.resultCount)}
                  </span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .history-page {
    display: flex;
    flex-direction: column;
    max-width: 860px;
    width: 100%;
    margin: 0 auto;
    padding: 24px 20px;
    gap: 1rem;
  }

  .page-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
  }

  .header-left {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .page-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--color-text);
  }

  .page-sub {
    font-size: 12.5px;
    color: var(--color-text-muted);
  }

  .clear-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: none;
    font-size: 12.5px;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .clear-btn:hover {
    background: color-mix(in srgb, var(--color-error) 10%, transparent);
    color: var(--color-error);
    border-color: var(--color-error);
  }

  /* ── Empty ── */
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 4rem 0;
    color: var(--color-text-muted);
    opacity: 0.5;
    font-size: 13.5px;
    user-select: none;
  }

  /* ── List ── */
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* ── Group card ── */
  .group-card {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 11px 14px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    cursor: pointer;
    user-select: none;
    transition: background-color 0.15s, border-color 0.15s;
  }

  .group-card:hover {
    background: var(--color-hover);
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }

  /* ── Target chips ── */
  .group-targets {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    align-items: center;
  }

  .target-chip {
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
    font-size: 12px;
    padding: 2px 7px;
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    color: var(--color-accent);
    border-radius: 4px;
  }

  .target-more {
    font-size: 11.5px;
    color: var(--color-text-muted);
    padding: 2px 4px;
  }

  /* ── Disclosure line ── */
  .group-disclosure {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .chevron-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 3px;
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color 0.12s, color 0.12s;
  }

  .chevron-btn:hover {
    background: var(--color-border);
    color: var(--color-text);
  }

  .disclosure-text {
    font-size: 11.5px;
    color: var(--color-text-muted);
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .disclosure-sep {
    opacity: 0.4;
  }

  /* ── Older consultations ── */
  .consultations {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-left: 22px;
  }

  .consultation {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11.5px;
    color: var(--color-text-muted);
  }

  .c-dot {
    color: var(--color-text-muted);
    opacity: 0.4;
  }

  .c-date {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .c-results {
    opacity: 0.7;
  }
</style>
