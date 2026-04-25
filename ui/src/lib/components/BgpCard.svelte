<script lang="ts">
  import { GripVertical, GitMerge } from 'lucide-svelte';
  import { i18n } from '$lib/services/i18n.svelte';
  import type { BgpInfo } from '$lib/types/ip';

  let { bgp }: { bgp: BgpInfo } = $props();
</script>

<div role="listitem" class="bgp-card">
  <div class="bgp-title">
    <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
    <GitMerge size={15} />
    {i18n.t('bgp.title')}
    {#if bgp.asn}
      <span class="bgp-asn-badge">AS{bgp.asn}</span>
    {/if}
  </div>

  <dl class="field-list">
    {#if bgp.as_name}
      <div class="field-row">
        <dt class="field-label">{i18n.t('bgp.field.as_name')}</dt>
        <dd class="field-value">{bgp.as_name}</dd>
      </div>
    {/if}
    {#if bgp.as_country}
      <div class="field-row">
        <dt class="field-label">{i18n.t('bgp.field.as_country')}</dt>
        <dd class="field-value">{bgp.as_country}</dd>
      </div>
    {/if}
  </dl>

  {#if bgp.prefixes_v4.length > 0 || bgp.prefixes_v6.length > 0}
    <div class="bgp-section-header">
      {#if bgp.prefixes_v4.length > 0}
        <span class="bgp-section-label">{i18n.t('bgp.section.prefixes_v4')}</span>
        <span class="bgp-count-badge">{i18n.t('bgp.prefix.count').replace('{n}', String(bgp.prefixes_v4.length))}</span>
      {/if}
      {#if bgp.prefixes_v6.length > 0}
        <span class="bgp-section-label" style="margin-left: 12px">{i18n.t('bgp.section.prefixes_v6')}</span>
        <span class="bgp-count-badge">{i18n.t('bgp.prefix.count').replace('{n}', String(bgp.prefixes_v6.length))}</span>
      {/if}
    </div>
    <div class="bgp-prefix-grid">
      {#each bgp.prefixes_v4 as prefix}
        <span class="prefix-tag prefix-tag--v4">{prefix}</span>
      {/each}
      {#each bgp.prefixes_v6 as prefix}
        <span class="prefix-tag prefix-tag--v6">{prefix}</span>
      {/each}
    </div>
  {/if}

  {#if bgp.peers.length > 0}
    <div class="bgp-section-header">
      <span class="bgp-section-label">{i18n.t('bgp.section.peers')}</span>
      <span class="bgp-count-badge">{i18n.t('bgp.peers.count').replace('{n}', String(bgp.peers.length))}</span>
    </div>
    <div class="bgp-peers-scroll">
      <table class="bgp-peers-table">
        <thead>
          <tr>
            <th>{i18n.t('bgp.col.asn')}</th>
            <th>{i18n.t('bgp.col.name')}</th>
            <th>{i18n.t('bgp.col.country')}</th>
          </tr>
        </thead>
        <tbody>
          {#each bgp.peers as peer}
            <tr>
              <td class="peer-asn">AS{peer.asn}</td>
              <td class="peer-name">{peer.name ?? '—'}</td>
              <td class="peer-country">{peer.country ?? '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .bgp-card {
    grid-column: 1 / -1;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }
  .bgp-title {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 10px 14px;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    background: var(--color-header-bg);
    border-bottom: 1px solid var(--color-border);
    cursor: grab;
    user-select: none;
  }
  .bgp-asn-badge {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: none;
    letter-spacing: 0;
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    color: var(--color-accent);
    border-radius: var(--radius-full);
    font-family: var(--font-mono);
  }
  .bgp-section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px 5px;
    border-top: 1px solid var(--color-border);
  }
  .bgp-section-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted);
  }
  .bgp-count-badge {
    padding: 1px 7px;
    font-size: 10.5px;
    font-weight: 600;
    background: var(--color-hover);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full);
    color: var(--color-text-muted);
  }
  .bgp-prefix-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    padding: 6px 14px 10px;
  }
  .prefix-tag {
    font-family: var(--font-mono);
    font-size: 11.5px;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-weight: 500;
  }
  .prefix-tag--v4 {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    color: var(--color-accent);
  }
  .prefix-tag--v6 {
    background: color-mix(in srgb, var(--color-success) 10%, transparent);
    color: var(--color-success);
  }
  .bgp-peers-scroll {
    max-height: 240px;
    overflow-y: auto;
    border-top: 1px solid var(--color-border);
  }
  .bgp-peers-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .bgp-peers-table thead th {
    padding: 6px 14px;
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-muted);
    background: var(--color-row-even);
    border-bottom: 1px solid var(--color-border);
    position: sticky;
    top: 0;
  }
  .bgp-peers-table tbody tr { border-bottom: 1px solid var(--color-border); }
  .bgp-peers-table tbody tr:last-child { border-bottom: none; }
  .bgp-peers-table tbody tr:hover { background: var(--color-row-hover); }
  .bgp-peers-table td { padding: 6px 14px; vertical-align: middle; }
  .peer-asn {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-accent);
    white-space: nowrap;
    width: 90px;
  }
  .peer-name { color: var(--color-text); word-break: break-word; }
  .peer-country { color: var(--color-text-muted); font-size: 12px; white-space: nowrap; width: 60px; }

  /* field-list shared with parent — reuse parent vars */
  .field-list { display: grid; }
  .field-row {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 8px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--color-border);
    align-items: start;
  }
  .field-label { font-size: 12px; color: var(--color-text-muted); padding-top: 1px; }
  .field-value { font-size: 13px; color: var(--color-text); word-break: break-word; }
</style>
