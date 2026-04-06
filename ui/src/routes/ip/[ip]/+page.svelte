<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { ArrowLeft, Globe, Server, Mail, Phone, MapPin, Network,
           TriangleAlert, FileText, ChevronDown, ChevronUp, Copy, Download, ExternalLink } from 'lucide-svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { resultsStore } from '$lib/stores/results.svelte';
  import { lookupSingle } from '$lib/services/lookup';
  import type { IpRecord } from '$lib/types/ip';

  const ip = $derived(page.params.ip ?? '');

  let record = $state<IpRecord | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let showRawWhois = $state(false);
  let copied = $state(false);
  let copiedWhois = $state(false);

  onMount(async () => {
    // Try the in-memory store first
    const found = resultsStore.results.find(r => r.ip === ip);
    if (found) {
      record = found;
      return;
    }
    // Fallback: call the API
    loading = true;
    try {
      record = await lookupSingle(ip);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Erreur inconnue';
    } finally {
      loading = false;
    }
  });

  async function copyIP() {
    await navigator.clipboard.writeText(ip);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function exportWhois() {
    if (!record?.raw_whois) return;
    const blob = new Blob([record.raw_whois], { type: 'text/plain' });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement('a');
    a.href = url;
    a.download = `whois_${ip}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  interface FieldGroup {
    title: string;
    icon: typeof Globe;
    fields: { label: string; value: string | null | string[] }[];
  }

  let groups = $derived.by<FieldGroup[]>(() => {
    if (!record) return [];
    return [
      {
        title: 'Network',
        icon: Network,
        fields: [
          { label: 'CIDR',          value: record.cidr },
          { label: 'From',          value: record.from_ip },
          { label: 'To',            value: record.to_ip },
          { label: 'Network Name',  value: record.network_name },
          { label: 'Status',        value: record.status },
          { label: 'Allocated',     value: record.allocated },
        ],
      },
      {
        title: 'Organisation',
        icon: Globe,
        fields: [
          { label: 'Owner',         value: record.owner_name },
          { label: 'Country',       value: record.country },
          { label: 'Address',       value: record.address },
          { label: 'Postal Code',   value: record.postal_code },
          { label: 'Contact',       value: record.contact_name },
          { label: 'Abuse Contact', value: record.abuse_contact },
        ],
      },
      {
        title: 'Contact',
        icon: Mail,
        fields: [
          { label: 'Emails',        value: record.emails },
          { label: 'Abuse Emails',  value: record.abuse_emails },
          { label: 'Phone',         value: record.phone },
          { label: 'Fax',           value: record.fax },
        ],
      },
      {
        title: 'DNS',
        icon: Server,
        fields: [
          { label: 'Hostname',      value: record.host_name },
          { label: 'Resolved from', value: record.resolved_name },
          { label: 'WHOIS Source',  value: record.whois_source },
        ],
      },
    ];
  });

  function formatValue(v: string | null | string[]): string {
    if (!v) return '—';
    if (Array.isArray(v)) return v.length > 0 ? v.join(', ') : '—';
    return v;
  }
</script>

<div class="detail-page">
  <!-- Back button -->
  <button class="back-btn" onclick={() => goto('/lookup')}>
    <ArrowLeft size={16} /> Back
  </button>

  {#if loading}
    <!-- Skeleton header -->
    <div class="sk-header">
      <div class="sk-ip"></div>
      <div class="sk-pills">
        <div class="sk-pill"></div>
        <div class="sk-pill" style="width:120px"></div>
        <div class="sk-pill" style="width:90px"></div>
      </div>
    </div>

    <!-- Skeleton cards grid -->
    <div class="sk-groups">
      {#each [4, 5, 3, 3] as rows}
        <div class="sk-card">
          <div class="sk-card-title"></div>
          {#each Array(rows) as _}
            <div class="sk-row">
              <div class="sk-label"></div>
              <div class="sk-value"></div>
            </div>
          {/each}
        </div>
      {/each}
    </div>

    <!-- Skeleton WHOIS -->
    <div class="sk-card sk-whois">
      <div class="sk-card-title"></div>
    </div>
  {:else if error}
    <div class="error-banner">
      <TriangleAlert size={16} />
      <span>{error}</span>
    </div>
  {:else if record}
    <!-- Header -->
    <div class="detail-header">
      <div class="ip-display">
        <span class="ip-text">{record.ip}</span>
        <button class="copy-btn" onclick={copyIP} title="Copy">
          <Copy size={14} />
          {#if copied}<span class="copied-label">Copied!</span>{/if}
        </button>
      </div>
      {#if record.lookup_errors.length > 0}
        <div class="error-pills">
          {#each record.lookup_errors as err}
            <span class="error-pill" title={err}>
              <TriangleAlert size={11} /> Resolution error
            </span>
          {/each}
        </div>
      {/if}
      <div class="meta-pills">
        {#if record.country}
          <span class="pill"><MapPin size={12} /> {record.country}</span>
        {/if}
        {#if record.owner_name}
          <span class="pill"><Globe size={12} /> {record.owner_name}</span>
        {/if}
        {#if record.network_name}
          <span class="pill"><Network size={12} /> {record.network_name}</span>
        {/if}
        {#if record.phone}
          <span class="pill"><Phone size={12} /> {record.phone}</span>
        {/if}
      </div>
    </div>

    <!-- Field groups -->
    <div class="groups">
      {#each groups as group}
        {@const nonEmpty = group.fields.filter(f => formatValue(f.value) !== '—')}
        {#if nonEmpty.length > 0}
          <div class="group-card">
            <div class="group-title">
              <group.icon size={15} />
              {group.title}
            </div>
            <dl class="field-list">
              {#each group.fields as field}
                {@const val = formatValue(field.value)}
                {#if val !== '—'}
                  <div class="field-row">
                    <dt class="field-label">{field.label}</dt>
                    <dd class="field-value">{val}</dd>
                  </div>
                {/if}
              {/each}
            </dl>
          </div>
        {/if}
      {/each}
    </div>

    <!-- GeoIP map -->
    {#if record.address || record.country}
      {@const mapQuery = [record.address, record.country].filter(Boolean).join(', ')}
      <div class="map-card">
        <div class="map-title">
          <MapPin size={15} />
          Server Location (GeoIP)
          <button
            class="map-open-btn"
            onclick={() => openUrl(`https://maps.google.com/?q=${encodeURIComponent(mapQuery)}`)}
            title="Open in Google Maps"
          >
            <ExternalLink size={13} /> Open in Maps
          </button>
        </div>
        <iframe
          class="map-frame"
          title="GeoIP Location"
          src="https://maps.google.com/maps?q={encodeURIComponent(mapQuery)}&output=embed&z=10"
          loading="lazy"
          referrerpolicy="no-referrer"
        ></iframe>
      </div>
    {/if}

    <!-- Raw WHOIS -->
    {#if record.raw_whois}
      <div class="raw-section">
        <div class="raw-header">
          <button
            class="raw-toggle"
            onclick={() => showRawWhois = !showRawWhois}
          >
            <FileText size={15} />
            Raw WHOIS
            {#if showRawWhois}<ChevronUp size={15} />{:else}<ChevronDown size={15} />{/if}
          </button>
          <button
            class="copy-whois-btn"
            onclick={async () => {
              await navigator.clipboard.writeText(record!.raw_whois!);
              copiedWhois = true;
              setTimeout(() => (copiedWhois = false), 1500);
            }}
            title="Copy raw WHOIS"
          >
            <Copy size={13} />
            {copiedWhois ? 'Copied!' : 'Copy WHOIS'}
          </button>
          <button
            class="copy-whois-btn"
            onclick={exportWhois}
            title="Export WHOIS as .txt"
          >
            <Download size={13} />
            Export WHOIS (.txt)
          </button>
        </div>
        {#if showRawWhois}
          <pre class="raw-content">{record.raw_whois}</pre>
        {/if}
      </div>
    {/if}
  {:else}
    <div class="not-found">
      <p>IP not found: <code>{ip}</code></p>
    </div>
  {/if}
</div>

<style>
  .detail-page {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    max-width: 1100px;
    width: 100%;
    margin: 0 auto;
    padding: 24px 20px;
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: none;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--color-text-muted);
    width: fit-content;
    transition: background-color 0.15s, color 0.15s;
  }

  .back-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  /* ── Skeletons ── */
  @keyframes shimmer {
    0%   { background-position: -400px 0; }
    100% { background-position:  400px 0; }
  }

  .sk-header {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .sk-groups {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 1.25rem;
  }

  .sk-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .sk-whois {
    height: 48px;
  }

  .sk-card-title {
    height: 38px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-header-bg);
  }

  .sk-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--color-border);
  }

  .sk-row:last-child {
    border-bottom: none;
  }

  .sk-ip,
  .sk-pill,
  .sk-label,
  .sk-value,
  .sk-card-title {
    border-radius: 4px;
    background: linear-gradient(
      90deg,
      var(--color-border) 25%,
      var(--color-hover)  50%,
      var(--color-border) 75%
    );
    background-size: 800px 100%;
    animation: shimmer 1.4s infinite linear;
  }

  .sk-ip {
    width: 200px;
    height: 36px;
    border-radius: 6px;
  }

  .sk-pills {
    display: flex;
    gap: 6px;
  }

  .sk-pill {
    width: 80px;
    height: 22px;
    border-radius: 999px;
  }

  .sk-label {
    width: 90px;
    height: 13px;
    flex-shrink: 0;
  }

  .sk-value {
    flex: 1;
    height: 13px;
    max-width: 220px;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    border: 1px solid var(--color-error);
    border-radius: 8px;
    color: var(--color-error);
    font-size: 13.5px;
  }

  /* ── Header ── */
  .detail-header {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .ip-display {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ip-text {
    font-size: 2rem;
    font-weight: 700;
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
    color: var(--color-accent);
    letter-spacing: 0.02em;
  }

  .copy-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    background: none;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-text-muted);
    transition: background-color 0.15s;
  }

  .copy-btn:hover {
    background: var(--color-hover);
  }

  .copied-label {
    color: var(--color-success);
    font-weight: 500;
  }

  .meta-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 10px;
    background: var(--color-hover);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    font-size: 12.5px;
    color: var(--color-text-muted);
  }

  .error-pills {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .error-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    border: 1px solid var(--color-error);
    border-radius: 999px;
    font-size: 11.5px;
    color: var(--color-error);
  }

  /* ── Groups ── */
  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 1.25rem;
  }

  .group-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .group-title {
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

  .field-list {
    padding: 2px 0;
  }

  .field-row {
    display: flex;
    align-items: baseline;
    padding: 7px 14px;
    border-bottom: 1px solid var(--color-border);
    gap: 12px;
  }

  .field-row:last-child {
    border-bottom: none;
  }

  .field-label {
    font-size: 13px;
    color: var(--color-text-muted);
    min-width: 110px;
    flex-shrink: 0;
  }

  .field-value {
    font-size: 13px;
    color: var(--color-text);
    word-break: break-word;
  }

  /* ── GeoIP map ── */
  .map-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .map-title {
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

  .map-open-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    padding: 4px 10px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: none;
    font-size: 11.5px;
    font-weight: 500;
    text-transform: none;
    letter-spacing: 0;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .map-open-btn:hover {
    background: var(--color-hover);
    color: var(--color-accent);
  }

  .map-frame {
    display: block;
    width: 100%;
    height: 280px;
    border: none;
  }

  /* ── Raw WHOIS ── */
  .raw-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .raw-header {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--color-border);
  }

  .raw-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    border-bottom: none;
    padding: 10px 14px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text);
    text-align: left;
    transition: background-color 0.15s;
  }

  .raw-toggle:hover {
    background: var(--color-hover);
  }

  .copy-whois-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
    padding: 6px 12px;
    margin-right: 8px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: none;
    font-size: 12px;
    color: var(--color-text-muted);
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 0.15s, color 0.15s;
  }

  .copy-whois-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .raw-content {
    padding: 14px 16px;
    border-top: none;
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
    font-size: 12.5px;
    line-height: 1.75;
    color: var(--color-text);
    background: var(--color-bg);
    overflow-x: auto;
    overflow-y: auto;
    white-space: pre;
    word-break: normal;
    tab-size: 4;
    max-height: 520px;
  }

  .not-found {
    padding: 3rem;
    text-align: center;
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .not-found code {
    background: var(--color-hover);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }
</style>
