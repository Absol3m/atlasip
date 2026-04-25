<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Clock, Search, Trash2, X, ChevronDown, ChevronUp } from 'lucide-svelte';
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

  function formatRelative(ts: number): string {
    const diff = Date.now() - ts;
    const min  = Math.floor(diff / 60_000);
    const hour = Math.floor(diff / 3_600_000);
    const day  = Math.floor(diff / 86_400_000);
    if (min  < 1)  return i18n.t('history.relative.now');
    if (min  < 60) return i18n.t('history.relative.minutes').replace('{n}', String(min));
    if (hour < 24) return i18n.t('history.relative.hours').replace('{n}', String(hour));
    if (day  < 7)  return i18n.t('history.relative.days').replace('{n}', String(day));
    return formatDate(ts);
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

        <div class="group-card" class:is-expanded={isExpanded}>

          <!-- ── Card header ── -->
          <div class="group-header">
            <div class="group-left">
              <div class="group-targets">
                {#each group.targets.slice(0, 5) as t}
                  <span class="target-chip">{t}</span>
                {/each}
                {#if group.targets.length > 5}
                  <span class="target-more">+{group.targets.length - 5}</span>
                {/if}
              </div>
              <div class="group-meta">
                <span class="meta-date">{formatRelative(group.lastTimestamp)}</span>
                {#if hasMany}
                  <span class="meta-sep">·</span>
                  <span class="meta-count">
                    {i18n.tn('history.consultations.one', 'history.consultations.many', group.consultations.length)}
                  </span>
                {:else}
                  <span class="meta-sep">·</span>
                  <span class="meta-count">
                    {i18n.tn('history.result.one', 'history.result.many', group.consultations[0].resultCount)}
                  </span>
                {/if}
              </div>
            </div>

            <div class="group-actions">
              {#if hasMany}
                <span class="count-badge">{group.consultations.length}×</span>
              {/if}
              <button class="rerun-btn" title={i18n.t('history.rerun.title')}
                onclick={() => rerun(group.targets)}
              >
                <Search size={13} /> {i18n.t('history.rerun')}
              </button>
              <button class="del-btn" title={i18n.t('history.delete.group')}
                onclick={() => historyStore.removeMany(group.consultations.map(c => c.id))}
              >
                <X size={13} />
              </button>
              {#if hasMany}
                <button class="chevron-btn" aria-label={isExpanded ? 'Collapse' : 'Expand'}
                  onclick={() => toggleGroup(group.key)}
                >
                  {#if isExpanded}<ChevronUp size={14} />{:else}<ChevronDown size={14} />{/if}
                </button>
              {/if}
            </div>
          </div>

          <!-- ── Consultation list (expanded) ── -->
          {#if isExpanded && hasMany}
            <div class="consultations">
              {#each group.consultations as c (c.id)}
                <div class="consultation">
                  <span class="c-date">{formatDate(c.timestamp)}</span>
                  <span class="c-results">
                    {i18n.tn('history.result.one', 'history.result.many', c.resultCount)}
                  </span>
                  <button class="c-del" title={i18n.t('history.delete')}
                    onclick={() => historyStore.remove(c.id)}
                  >
                    <X size={11} />
                  </button>
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
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    overflow: hidden;
    transition: border-color 0.15s;
  }

  .group-card:hover {
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }

  .group-card.is-expanded {
    border-color: color-mix(in srgb, var(--color-accent) 30%, transparent);
  }

  /* ── Card header ── */
  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
  }

  .group-left {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

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

  .group-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--color-text-muted);
  }

  .meta-sep { opacity: 0.4; }

  /* ── Actions ── */
  .group-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .count-badge {
    padding: 2px 8px;
    font-size: 11.5px;
    font-weight: 700;
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    color: var(--color-accent);
    border-radius: var(--radius-full);
    font-family: var(--font-mono);
    letter-spacing: 0;
  }

  .rerun-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: none;
    font-size: 12px;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
    white-space: nowrap;
  }

  .rerun-btn:hover {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    color: var(--color-accent);
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }

  .del-btn, .chevron-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .del-btn:hover {
    background: color-mix(in srgb, var(--color-error) 10%, transparent);
    color: var(--color-error);
    border-color: var(--color-error);
  }

  .chevron-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  /* ── Consultation list ── */
  .consultations {
    border-top: 1px solid var(--color-border);
    background: var(--color-header-bg);
  }

  .consultation {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--color-border);
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .consultation:last-child { border-bottom: none; }

  .c-date { font-family: var(--font-mono); font-size: 11.5px; }
  .c-results { margin-left: auto; }

  .c-del {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s, color 0.15s, border-color 0.15s;
  }

  .c-del:hover {
    opacity: 1;
    color: var(--color-error);
    border-color: var(--color-error);
  }
</style>
