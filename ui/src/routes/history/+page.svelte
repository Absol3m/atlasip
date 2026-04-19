<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { Clock, Search, Trash2, X } from 'lucide-svelte';
  import { historyStore } from '$lib/stores/history.svelte';
  import { analysisStore } from '$lib/stores/analysis.svelte';
  import { i18n } from '$lib/services/i18n.svelte';

  onMount(() => {
    historyStore.init();
  });

  function rerun(targets: string[]) {
    analysisStore.setInput(targets.join('\n'));
    analysisStore.pendingTrigger = true;
    goto('/lookup');
  }

  function formatDate(ts: number): string {
    const d = new Date(ts);
    return d.toLocaleDateString(i18n.locale, {
      day:    '2-digit',
      month:  '2-digit',
      year:   'numeric',
      hour:   '2-digit',
      minute: '2-digit',
    });
  }
</script>

<div class="history-page">
  <div class="page-header">
    <div class="header-left">
      <h1 class="page-title"><Clock size={16} /> {i18n.t('history.title')}</h1>
      <p class="page-sub">{i18n.tn('history.count.one', 'history.count.many', historyStore.entries.length)}</p>
    </div>
    {#if historyStore.entries.length > 0}
      <button class="clear-btn" onclick={() => historyStore.clear()} title={i18n.t('history.clear.title')}>
        <Trash2 size={14} /> {i18n.t('history.clear')}
      </button>
    {/if}
  </div>

  {#if historyStore.entries.length === 0}
    <div class="empty">
      <Clock size={40} strokeWidth={1.2} />
      <p>{i18n.t('history.empty')}</p>
    </div>
  {:else}
    <div class="list">
      {#each historyStore.entries as entry (entry.id)}
        <div class="entry" role="button" tabindex="0"
          onclick={() => rerun(entry.targets)}
          onkeydown={(e) => e.key === 'Enter' && rerun(entry.targets)}
        >
          <div class="entry-main">
            <div class="entry-targets">
              {#each entry.targets.slice(0, 5) as t}
                <span class="target-chip">{t}</span>
              {/each}
              {#if entry.targets.length > 5}
                <span class="target-more">+{entry.targets.length - 5}</span>
              {/if}
            </div>
            <div class="entry-meta">
              <span class="meta-date">{formatDate(entry.timestamp)}</span>
              <span class="meta-sep">·</span>
              <span class="meta-count">{i18n.tn('history.result.one', 'history.result.many', entry.resultCount)}</span>
            </div>
          </div>
          <div class="entry-actions">
            <button class="rerun-btn" title={i18n.t('history.rerun.title')}
              onclick={(e) => { e.stopPropagation(); rerun(entry.targets); }}
            >
              <Search size={13} /> {i18n.t('history.rerun')}
            </button>
            <button class="del-btn" title={i18n.t('history.delete')}
              onclick={(e) => { e.stopPropagation(); historyStore.remove(entry.id); }}
            >
              <X size={13} />
            </button>
          </div>
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

  .entry {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    cursor: pointer;
    transition: background-color 0.15s, border-color 0.15s;
  }

  .entry:hover {
    background: var(--color-hover);
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }

  .entry-main {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

  .entry-targets {
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

  .entry-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--color-text-muted);
  }

  .meta-sep {
    opacity: 0.4;
  }

  /* ── Actions ── */
  .entry-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
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

  .del-btn {
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
</style>
