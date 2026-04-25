<script lang="ts">
  import { GripVertical, Server, ShieldCheck } from 'lucide-svelte';
  import { i18n } from '$lib/services/i18n.svelte';
  import type { DnsRecord } from '$lib/types/ip';

  let { dnsRecords }: { dnsRecords: DnsRecord[] } = $props();

  const dnssecOk = $derived(dnsRecords.some(r => r.dnssec_validated));
</script>

<div role="listitem" class="dns-card">
  <div class="dns-title">
    <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
    <Server size={15} />
    {i18n.t('dns.title')}
    {#if dnssecOk}
      <span class="dnssec-badge"><ShieldCheck size={11} />{i18n.t('dns.dnssec.validated')}</span>
    {/if}
  </div>
  <table class="dns-table">
    <thead>
      <tr>
        <th>{i18n.t('dns.col.type')}</th>
        <th>{i18n.t('dns.col.value')}</th>
        <th class="ttl-col">{i18n.t('dns.col.ttl')}</th>
      </tr>
    </thead>
    <tbody>
      {#each dnsRecords as rec}
        <tr>
          <td><span class="dns-type dns-type--{rec.record_type.toLowerCase()}">{rec.record_type}</span></td>
          <td class="dns-value">{rec.value}</td>
          <td class="ttl-col dns-ttl">{rec.ttl}s</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .dns-card {
    grid-column: 1 / -1;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }
  .dns-title {
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
  }
  .dnssec-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: none;
    letter-spacing: 0;
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
    border-radius: var(--radius-full);
  }
  .dns-table { width: 100%; border-collapse: collapse; font-size: 13px; }
  .dns-table thead th {
    padding: 6px 14px;
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-muted);
    background: var(--color-row-even);
    border-bottom: 1px solid var(--color-border);
  }
  .dns-table tbody tr { border-bottom: 1px solid var(--color-border); }
  .dns-table tbody tr:last-child { border-bottom: none; }
  .dns-table tbody tr:hover { background: var(--color-row-hover); }
  .dns-table td { padding: 7px 14px; vertical-align: middle; }
  .dns-type {
    display: inline-block;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-weight: 600;
    background: var(--color-hover);
    color: var(--color-text-muted);
  }
  .dns-type--a, .dns-type--aaaa {
    background: color-mix(in srgb, var(--color-accent) 15%, transparent);
    color: var(--color-accent);
  }
  .dns-type--mx {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }
  .dns-type--ns, .dns-type--soa {
    background: color-mix(in srgb, var(--color-warning) 15%, transparent);
    color: var(--color-warning-text);
  }
  .dns-type--txt {
    background: color-mix(in srgb, var(--color-text-muted) 15%, transparent);
    color: var(--color-text-muted);
  }
  .dns-value { font-family: var(--font-mono); font-size: 12.5px; word-break: break-all; color: var(--color-text); }
  .ttl-col { text-align: right; width: 72px; }
  .dns-ttl { font-size: 11.5px; color: var(--color-text-muted); white-space: nowrap; }
</style>
