<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { ArrowLeft, Globe, Server, Mail, Phone, MapPin, Network,
           AlertTriangle, FileText, ChevronDown, ChevronUp, Copy } from 'lucide-svelte';
  import { resultsStore } from '$lib/stores/results.svelte';
  import { lookupSingle } from '$lib/services/lookup';
  import type { IpRecord } from '$lib/types/ip';

  const ip = $derived(page.params.ip ?? '');

  let record = $state<IpRecord | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let showRawWhois = $state(false);
  let copied = $state(false);

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

  interface FieldGroup {
    title: string;
    icon: typeof Globe;
    fields: { label: string; value: string | null | string[] }[];
  }

  let groups = $derived.by<FieldGroup[]>(() => {
    if (!record) return [];
    return [
      {
        title: 'Réseau',
        icon: Network,
        fields: [
          { label: 'CIDR',          value: record.cidr },
          { label: 'De',            value: record.from_ip },
          { label: 'À',             value: record.to_ip },
          { label: 'Network Name',  value: record.network_name },
          { label: 'Statut',        value: record.status },
          { label: 'Alloué le',     value: record.allocated },
        ],
      },
      {
        title: 'Organisation',
        icon: Globe,
        fields: [
          { label: 'Propriétaire',  value: record.owner_name },
          { label: 'Pays',          value: record.country },
          { label: 'Adresse',       value: record.address },
          { label: 'Code postal',   value: record.postal_code },
          { label: 'Contact',       value: record.contact_name },
          { label: 'Abuse Contact', value: record.abuse_contact },
        ],
      },
      {
        title: 'Contact',
        icon: Mail,
        fields: [
          { label: 'Emails',        value: record.emails },
          { label: 'Abuse emails',  value: record.abuse_emails },
          { label: 'Téléphone',     value: record.phone },
          { label: 'Fax',           value: record.fax },
        ],
      },
      {
        title: 'DNS',
        icon: Server,
        fields: [
          { label: 'Hostname',      value: record.host_name },
          { label: 'Résolu depuis', value: record.resolved_name },
          { label: 'Source WHOIS',  value: record.whois_source },
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
    <ArrowLeft size={16} /> Retour
  </button>

  {#if loading}
    <div class="loading">
      <span class="spinner"></span>
      <span>Chargement…</span>
    </div>
  {:else if error}
    <div class="error-banner">
      <AlertTriangle size={16} />
      <span>{error}</span>
    </div>
  {:else if record}
    <!-- Header -->
    <div class="detail-header">
      <div class="ip-display">
        <span class="ip-text">{record.ip}</span>
        <button class="copy-btn" onclick={copyIP} title="Copier">
          <Copy size={14} />
          {#if copied}<span class="copied-label">Copié !</span>{/if}
        </button>
      </div>
      {#if record.lookup_errors.length > 0}
        <div class="error-pills">
          {#each record.lookup_errors as err}
            <span class="error-pill" title={err}>
              <AlertTriangle size={11} /> Erreur de résolution
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

    <!-- Raw WHOIS -->
    {#if record.raw_whois}
      <div class="raw-section">
        <button
          class="raw-toggle"
          onclick={() => showRawWhois = !showRawWhois}
        >
          <FileText size={15} />
          WHOIS brut
          {#if showRawWhois}<ChevronUp size={15} />{:else}<ChevronDown size={15} />{/if}
        </button>
        {#if showRawWhois}
          <pre class="raw-content">{record.raw_whois}</pre>
        {/if}
      </div>
    {/if}
  {:else}
    <div class="not-found">
      <p>IP non trouvée : <code>{ip}</code></p>
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

  .loading {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--color-text-muted);
    padding: 2rem;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    display: inline-block;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
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
    font-size: 12px;
    color: var(--color-text-muted);
    min-width: 110px;
    flex-shrink: 0;
  }

  .field-value {
    font-size: 13.5px;
    color: var(--color-text);
    word-break: break-word;
  }

  /* ── Raw WHOIS ── */
  .raw-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .raw-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 11px 16px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--color-text);
    text-align: left;
    transition: background-color 0.15s;
  }

  .raw-toggle:hover {
    background: var(--color-hover);
  }

  .raw-content {
    padding: 14px 16px;
    border-top: 1px solid var(--color-border);
    font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
    font-size: 12px;
    line-height: 1.6;
    color: var(--color-text);
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 480px;
    overflow-y: auto;
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
