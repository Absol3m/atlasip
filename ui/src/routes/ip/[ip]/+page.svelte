<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { ArrowLeft, Globe, Server, Mail, Phone, MapPin, Network,
           TriangleAlert, FileText, ChevronDown, ChevronUp, Copy, Download,
           ExternalLink, ShieldCheck, GripVertical, Navigation, Layers, X, GitMerge } from 'lucide-svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { resultsStore } from '$lib/stores/results.svelte';
  import { lookupSingle } from '$lib/services/lookup';
  import type { IpRecord } from '$lib/types/ip';

  const ip = $derived(page.params.ip ?? '');

  let record = $state<IpRecord | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let showRawWhois  = $state(false);
  let copied        = $state(false);
  let copiedWhois   = $state(false);
  let mapSource     = $state<'rdap' | 'geoip'>('rdap');

  import Sortable from 'sortablejs';
  import { i18n } from '$lib/services/i18n.svelte';
  import { reverseIpLookup, type ReverseIpResult } from '$lib/services/atlasip';

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
    id:     string;
    title:  string;
    icon:   typeof Globe;
    fields: { label: string; value: string | null | string[] }[];
  }

  function fmtCoords(lat: number | null, lon: number | null): string | null {
    if (lat == null || lon == null) return null;
    return `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
  }

  let groups = $derived.by<FieldGroup[]>(() => {
    if (!record) return [];
    return [
      {
        id: 'network', title: i18n.t('detail.group.network'), icon: Network,
        fields: [
          { label: i18n.t('detail.field.cidr'),         value: record.cidr },
          { label: i18n.t('detail.field.from_ip'),      value: record.from_ip },
          { label: i18n.t('detail.field.to_ip'),        value: record.to_ip },
          { label: i18n.t('detail.field.network_name'), value: record.network_name },
          { label: i18n.t('detail.field.status'),       value: record.status },
          { label: i18n.t('detail.field.allocated'),    value: record.allocated },
        ],
      },
      {
        id: 'organisation', title: i18n.t('detail.group.organisation'), icon: Globe,
        fields: [
          { label: i18n.t('detail.field.owner'),         value: record.owner_name },
          { label: i18n.t('detail.field.country'),       value: record.country },
          { label: i18n.t('detail.field.address'),       value: record.address },
          { label: i18n.t('detail.field.postal_code'),   value: record.postal_code },
          { label: i18n.t('detail.field.contact'),       value: record.contact_name },
          { label: i18n.t('detail.field.abuse_contact'), value: record.abuse_contact },
        ],
      },
      {
        id: 'contact', title: i18n.t('detail.group.contact'), icon: Mail,
        fields: [
          { label: i18n.t('detail.field.emails'),       value: record.emails },
          { label: i18n.t('detail.field.abuse_emails'), value: record.abuse_emails },
          { label: i18n.t('detail.field.phone'),        value: record.phone },
          { label: i18n.t('detail.field.fax'),          value: record.fax },
        ],
      },
      {
        id: 'dns', title: i18n.t('detail.group.dns'), icon: Server,
        fields: [
          { label: i18n.t('detail.field.hostname'),      value: record.host_name },
          { label: i18n.t('detail.field.resolved_from'), value: record.resolved_name },
          { label: i18n.t('detail.field.whois_source'),  value: record.whois_source },
        ],
      },
      {
        id: 'geolocation', title: i18n.t('detail.group.geolocation'), icon: Navigation,
        fields: [
          { label: i18n.t('detail.field.geo_city'),    value: record.geo_city },
          { label: i18n.t('detail.field.geo_country'), value: record.country },
          { label: i18n.t('detail.field.geo_coords'),  value: fmtCoords(record.geo_lat, record.geo_lon) },
        ],
      },
    ];
  });

  function formatValue(v: string | null | string[]): string {
    if (!v) return '—';
    if (Array.isArray(v)) return v.length > 0 ? v.join(', ') : '—';
    return v;
  }

  // ── Draggable card order ──────────────────────────────────────────────────

  const ALL_CARD_IDS = ['network', 'organisation', 'contact', 'dns', 'geolocation', 'dns_records', 'map', 'bgp', 'whois'];
  const LS_ORDER_KEY = 'atlasip.detail-group-order-v2';

  function loadCardOrder(): string[] {
    try {
      const saved = localStorage.getItem(LS_ORDER_KEY);
      if (saved) {
        const parsed: string[] = JSON.parse(saved);
        const missing = ALL_CARD_IDS.filter(id => !parsed.includes(id));
        return [...parsed.filter(id => ALL_CARD_IDS.includes(id)), ...missing];
      }
    } catch { /* ignore */ }
    return [...ALL_CARD_IDS];
  }

  function saveCardOrder(order: string[]) {
    try { localStorage.setItem(LS_ORDER_KEY, JSON.stringify(order)); } catch { /* ignore */ }
  }

  let cardOrder = $state<string[]>(loadCardOrder());

  let visibleCards = $derived.by(() => {
    const visibleIds = new Set<string>();
    for (const g of groups) {
      if (g.fields.some(f => formatValue(f.value) !== '—')) visibleIds.add(g.id);
    }
    if (record?.dns_records?.length)                           visibleIds.add('dns_records');
    if (record?.address || record?.country || record?.geo_lat) visibleIds.add('map');
    if (record?.bgp?.asn)                                      visibleIds.add('bgp');
    if (record?.raw_whois)                                     visibleIds.add('whois');
    const ordered   = cardOrder.filter(id => visibleIds.has(id));
    const unordered = [...visibleIds].filter(id => !cardOrder.includes(id));
    return [...ordered, ...unordered];
  });

  function sortableAction(node: HTMLElement) {
    const instance = Sortable.create(node, {
      animation: 200,
      handle: '.drag-handle',
      ghostClass: 'sortable-ghost',
      chosenClass: 'sortable-chosen',
      onEnd(evt) {
        const oldIdx = evt.oldIndex ?? -1;
        const newIdx = evt.newIndex ?? -1;
        if (oldIdx === newIdx || oldIdx < 0 || newIdx < 0) return;
        const next = [...visibleCards];
        const [moved] = next.splice(oldIdx, 1);
        next.splice(newIdx, 0, moved);
        cardOrder = next;
        saveCardOrder(next);
      },
    });
    return { destroy: () => instance.destroy() };
  }

  // ── Reverse IP drawer ─────────────────────────────────────────────────────

  let drawerOpen      = $state(false);
  let reverseLoading  = $state(false);
  let reverseResult   = $state<ReverseIpResult | null>(null);
  let reverseError    = $state<string | null>(null);

  async function openReverseDrawer() {
    drawerOpen = true;
    if (reverseResult) return; // already loaded
    reverseLoading = true;
    reverseError   = null;
    try {
      reverseResult = await reverseIpLookup(ip, record?.resolved_name ?? undefined);
    } catch (e) {
      reverseError = e instanceof Error ? e.message : i18n.t('reverse_ip.error');
    } finally {
      reverseLoading = false;
    }
  }

  let visibleErrors = $derived(
    (record?.lookup_errors ?? []).filter(e => {
      if (e.startsWith('PTR:')) return false;
      if (e.startsWith('WHOIS:') && record?.whois_source === 'RDAP') return false;
      return true;
    })
  );
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
        <button class="reverse-btn" onclick={openReverseDrawer} title={i18n.t('reverse_ip.btn')}>
          <Layers size={14} />
          {i18n.t('reverse_ip.btn')}
        </button>
      </div>
      {#if visibleErrors.length > 0}
        <div class="error-pills">
          {#each visibleErrors as err}
            <span class="error-pill" title={err}>
              <TriangleAlert size={11} /> {err.length > 60 ? err.slice(0, 57) + '…' : err}
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

    <!-- All tiles (draggable) -->
    <div class="groups" role="list" use:sortableAction>
      {#each visibleCards as card (card)}
        {@const group = groups.find(g => g.id === card)}

        {#if group}
          <!-- ── Field group card ── -->
          <div role="listitem" class="group-card">
            <div class="group-title">
              <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
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

        {:else if card === 'dns_records'}
          <!-- ── DNS records card ── -->
          {@const dnssecOk = record.dns_records.some(r => r.dnssec_validated)}
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
                {#each record.dns_records as rec}
                  <tr>
                    <td><span class="dns-type dns-type--{rec.record_type.toLowerCase()}">{rec.record_type}</span></td>
                    <td class="dns-value">{rec.value}</td>
                    <td class="ttl-col dns-ttl">{rec.ttl}s</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

        {:else if card === 'map'}
          <!-- ── Map card ── -->
          {@const rdapQuery   = [record.address, record.country].filter(Boolean).join(', ')}
          {@const hasGeoIp    = record.geo_lat != null && record.geo_lon != null}
          {@const geoQuery    = hasGeoIp ? `${record.geo_lat},${record.geo_lon}` : ''}
          {@const activeQuery = mapSource === 'geoip' && hasGeoIp ? geoQuery : rdapQuery}
          <div role="listitem" class="map-card">
            <div class="map-title">
              <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
              <MapPin size={15} />
              <div class="map-source-toggle">
                <button class="map-source-btn" class:active={mapSource === 'rdap'} onclick={() => (mapSource = 'rdap')}>{i18n.t('map.source.rdap')}</button>
                <button class="map-source-btn" class:active={mapSource === 'geoip'} disabled={!hasGeoIp} onclick={() => hasGeoIp && (mapSource = 'geoip')} title={!hasGeoIp ? i18n.t('map.geoip.unavailable') : undefined}>{i18n.t('map.source.geoip')}</button>
              </div>
              {#if activeQuery}
                <button class="map-open-btn" onclick={() => openUrl(`https://maps.google.com/?q=${encodeURIComponent(activeQuery)}`)} title="Open in Google Maps"><ExternalLink size={13} /></button>
              {/if}
            </div>
            {#if mapSource === 'geoip' && !hasGeoIp}
              <div class="map-unavailable">
                <p>{i18n.t('map.geoip.unavailable')}</p>
                <a href="/settings" class="map-configure-link">{i18n.t('map.geoip.configure')}</a>
              </div>
            {:else if activeQuery}
              <iframe class="map-frame" title="Location" src="https://maps.google.com/maps?q={encodeURIComponent(activeQuery)}&output=embed&z={mapSource === 'geoip' ? 12 : 10}" loading="lazy" referrerpolicy="no-referrer"></iframe>
            {/if}
          </div>

        {:else if card === 'bgp' && record.bgp}
          <!-- ── BGP / ASN card ── -->
          {@const bgp = record.bgp}
          <div role="listitem" class="bgp-card">
            <div class="bgp-title">
              <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
              <GitMerge size={15} />
              {i18n.t('bgp.title')}
              {#if bgp.asn}
                <span class="bgp-asn-badge">AS{bgp.asn}</span>
              {/if}
            </div>

            <!-- ASN summary row -->
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

            <!-- Prefixes -->
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

            <!-- Peers -->
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

        {:else if card === 'whois'}
          <!-- ── Raw WHOIS card ── -->
          <div role="listitem" class="raw-section">
            <div class="raw-header">
              <span class="drag-handle" aria-hidden="true"><GripVertical size={13} /></span>
              <button class="raw-toggle" onclick={() => showRawWhois = !showRawWhois}>
                <FileText size={15} />
                Raw WHOIS
                {#if showRawWhois}<ChevronUp size={15} />{:else}<ChevronDown size={15} />{/if}
              </button>
              <button class="copy-whois-btn" onclick={async () => { await navigator.clipboard.writeText(record!.raw_whois!); copiedWhois = true; setTimeout(() => (copiedWhois = false), 1500); }} title="Copy raw WHOIS">
                <Copy size={13} />{copiedWhois ? 'Copied!' : 'Copy WHOIS'}
              </button>
              <button class="copy-whois-btn" onclick={exportWhois} title="Export WHOIS as .txt">
                <Download size={13} />Export WHOIS (.txt)
              </button>
            </div>
            {#if showRawWhois}
              <pre class="raw-content">{record.raw_whois}</pre>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {:else}
    <div class="not-found">
      <p>IP not found: <code>{ip}</code></p>
    </div>
  {/if}
</div>

<!-- Reverse IP drawer -->
{#if drawerOpen}
  <div class="drawer-backdrop" onclick={() => (drawerOpen = false)} role="presentation"></div>
  <aside class="drawer" aria-label={i18n.t('reverse_ip.title')}>
    <div class="drawer-header">
      <div class="drawer-title">
        <Layers size={15} />
        {i18n.t('reverse_ip.title')}
        <span class="drawer-ip">{ip}</span>
      </div>
      <button class="drawer-close" onclick={() => (drawerOpen = false)} aria-label="Close">
        <X size={16} />
      </button>
    </div>

    <div class="drawer-body">
      {#if reverseLoading}
        <div class="drawer-loading">
          <div class="spinner"></div>
          <span>{i18n.t('reverse_ip.loading')}</span>
        </div>
      {:else if reverseError}
        <div class="drawer-error">
          <TriangleAlert size={15} />
          {reverseError}
        </div>
      {:else if reverseResult}
        <div class="drawer-meta">
          <span class="drawer-count">
            {i18n.t('reverse_ip.count').replace('{n}', String(reverseResult.count))}
          </span>
        </div>
        {#if reverseResult.source_errors.length > 0}
          <div class="drawer-source-errors">
            {#each reverseResult.source_errors as se}
              <span class="source-error-badge" title={se.error}>{se.source}</span>
            {/each}
          </div>
        {/if}
        {#if reverseResult.results.length === 0}
          <div class="drawer-empty">{i18n.t('reverse_ip.empty')}</div>
        {:else}
          <ul class="domain-list">
            {#each reverseResult.results as entry}
              <li class="domain-item">
                <Globe size={12} />
                <span class="domain-name">{entry.domain}</span>
                <span class="source-badges">
                  {#each entry.sources as src}
                    <span class="source-badge source-badge--{src}">{src}</span>
                  {/each}
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  </aside>
{/if}

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

  /* SortableJS states — :global because applied by JS outside Svelte scope */
  :global(.sortable-ghost) {
    opacity: 0.35;
    outline: 2px dashed var(--color-accent);
    outline-offset: 2px;
  }

  :global(.sortable-chosen) {
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    cursor: grabbing;
  }

  .group-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    overflow: hidden;
    box-shadow: var(--shadow);
    cursor: default;
    transition: outline 0.1s, opacity 0.15s;
  }


  .drag-handle {
    display: flex;
    align-items: center;
    color: var(--color-border);
    cursor: grab;
    opacity: 0.35;
    transition: opacity 0.15s;
    flex-shrink: 0;
    touch-action: none;
    -webkit-user-select: none;
    user-select: none;
  }

  :global(body.is-dragging) {
    cursor: grabbing !important;
    user-select: none;
  }

  .group-title:hover .drag-handle {
    opacity: 1;
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
    cursor: grab;
    user-select: none;
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
    grid-column: 1 / -1;
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

  .map-source-toggle {
    display: flex;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .map-source-btn {
    padding: 3px 10px;
    font-size: 11px;
    font-weight: 500;
    text-transform: none;
    letter-spacing: 0;
    border: none;
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
  }

  .map-source-btn.active {
    background: var(--color-accent);
    color: #fff;
  }

  .map-source-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .map-open-btn {
    display: flex;
    align-items: center;
    margin-left: auto;
    padding: 4px 6px;
    border: 1px solid var(--color-border);
    border-radius: 5px;
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .map-open-btn:hover {
    background: var(--color-hover);
    color: var(--color-accent);
  }

  .map-unavailable {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 120px;
    font-size: 12.5px;
    color: var(--color-text-muted);
  }

  .map-configure-link {
    font-size: 12px;
    color: var(--color-accent);
    text-decoration: none;
  }
  .map-configure-link:hover { text-decoration: underline; }

  .map-frame {
    display: block;
    width: 100%;
    height: 280px;
    border: none;
  }

  /* ── BGP / ASN card ── */
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

  .bgp-peers-table tbody tr {
    border-bottom: 1px solid var(--color-border);
  }

  .bgp-peers-table tbody tr:last-child { border-bottom: none; }
  .bgp-peers-table tbody tr:hover { background: var(--color-row-hover); }

  .bgp-peers-table td {
    padding: 6px 14px;
    vertical-align: middle;
  }

  .peer-asn {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-accent);
    white-space: nowrap;
    width: 90px;
  }

  .peer-name {
    color: var(--color-text);
    word-break: break-word;
  }

  .peer-country {
    color: var(--color-text-muted);
    font-size: 12px;
    white-space: nowrap;
    width: 60px;
  }

  /* ── Raw WHOIS ── */
  .raw-section {
    grid-column: 1 / -1;
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

  /* ── DNS records ── */
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

  .dns-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  .dns-table thead th {
    padding: 6px 14px;
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-muted);
    background: var(--color-row-even);
    border-bottom: 1px solid var(--color-border);
  }

  .dns-table tbody tr {
    border-bottom: 1px solid var(--color-border);
  }

  .dns-table tbody tr:last-child { border-bottom: none; }

  .dns-table tbody tr:hover { background: var(--color-row-hover); }

  .dns-table td {
    padding: 7px 14px;
    vertical-align: middle;
  }

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

  .dns-value {
    font-family: var(--font-mono);
    font-size: 12.5px;
    word-break: break-all;
    color: var(--color-text);
  }

  .ttl-col {
    text-align: right;
    width: 72px;
  }

  .dns-ttl {
    font-size: 11.5px;
    color: var(--color-text-muted);
    white-space: nowrap;
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

  /* ── Reverse IP button ── */
  .reverse-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 12px;
    background: none;
    border: 1px solid var(--color-accent);
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
    color: var(--color-accent);
    transition: background-color 0.15s, color 0.15s;
  }

  .reverse-btn:hover {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  }

  /* ── Drawer ── */
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    z-index: 100;
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 400px;
    background: var(--color-surface);
    border-left: 1px solid var(--color-border);
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.15);
    z-index: 101;
    display: flex;
    flex-direction: column;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-header-bg);
    flex-shrink: 0;
  }

  .drawer-title {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
  }

  .drawer-ip {
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    padding: 1px 7px;
    border-radius: 4px;
  }

  .drawer-close {
    display: flex;
    align-items: center;
    padding: 4px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    border-radius: 4px;
    transition: background-color 0.15s, color 0.15s;
  }

  .drawer-close:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 0;
  }

  .drawer-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 160px;
    color: var(--color-text-muted);
    font-size: 13px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .spinner {
    width: 22px;
    height: 22px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  .drawer-error {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 12px 16px;
    padding: 10px 12px;
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    border: 1px solid var(--color-error);
    border-radius: 7px;
    color: var(--color-error);
    font-size: 13px;
  }

  .drawer-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 16px 10px;
    border-bottom: 1px solid var(--color-border);
    margin-bottom: 4px;
  }

  .drawer-count {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text);
  }

  .drawer-source {
    font-size: 11px;
    color: var(--color-text-muted);
  }

  .drawer-empty {
    padding: 40px 16px;
    text-align: center;
    font-size: 13px;
    color: var(--color-text-muted);
  }

  .domain-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .domain-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-muted);
    transition: background-color 0.1s;
  }

  .domain-item:last-child {
    border-bottom: none;
  }

  .domain-item:hover {
    background: var(--color-hover);
  }

  .domain-name {
    font-family: var(--font-mono);
    font-size: 12.5px;
    color: var(--color-text);
    word-break: break-all;
    flex: 1;
  }

  .source-badges {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .source-badge {
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--color-hover);
    color: var(--color-text-muted);
  }

  .source-badge--hackertarget {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    color: var(--color-accent);
  }

  .source-badge--crtsh {
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    color: var(--color-success);
  }

  .drawer-source-errors {
    display: flex;
    gap: 6px;
    padding: 6px 16px 10px;
  }

  .source-error-badge {
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10.5px;
    font-weight: 600;
    background: color-mix(in srgb, var(--color-error) 12%, transparent);
    color: var(--color-error);
    cursor: help;
  }
</style>
