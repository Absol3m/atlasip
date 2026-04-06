<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import HybridDurationInput from '$lib/components/HybridDurationInput.svelte';
  import Toggle from '$lib/components/Toggle.svelte';
  import { theme } from '$lib/stores/app';
  import { i18n } from '$lib/services/i18n.svelte';

  // ── Types ─────────────────────────────────────────────────────────────────

  type LocaleSetting  = 'en' | 'fr';
  type ThemeSetting   = 'light' | 'dark' | 'system';
  type DnsResolver    = 'system' | 'cloudflare' | 'google' | 'quad9';
  type IpMode         = 'ipv4' | 'ipv6' | 'dual';
  type RetryStrategy  = 'none' | 'linear' | 'exponential';
  type ProxyType      = 'HTTP' | 'HTTPS' | 'SOCKS4' | 'SOCKS5';

  // Shape matching config::AppConfig (snake_case, as Tauri serialises it)
  interface AppConfig {
    network: {
      dns_resolver:    string;
      ip_mode:         string;
      pooling_enabled: boolean;
      max_connections: number;
      keep_alive:      boolean;
      retry_strategy:  string;
      retry_delay_ms:  number;
    };
    proxy: {
      enabled:    boolean;
      proxy_type: string;
      url:        string;
      no_proxy:   string;
    };
    timeouts: {
      global_ms:  number;
      request_ms: number;
      dns_ms:     number;
      geoip_ms:   number;
    };
    retry: {
      enabled:  boolean;
      count:    number;
      delay_ms: number;
    };
    locale: string;
    theme:  string;
  }

  // ── Locale ────────────────────────────────────────────────────────────────

  const LOCALE_LABELS: Record<LocaleSetting, string> = {
    en: 'English',
    fr: 'Français',
  };

  let localeSetting = $state<LocaleSetting>(i18n.locale as LocaleSetting);

  function onLocaleChange(e: Event) {
    localeSetting = (e.target as HTMLSelectElement).value as LocaleSetting;
    i18n.setLocale(localeSetting);
  }

  // ── Theme ─────────────────────────────────────────────────────────────────

  const LS_THEME_KEY = 'atlasip.theme-setting';

  function loadThemeSetting(): ThemeSetting {
    try { return (localStorage.getItem(LS_THEME_KEY) as ThemeSetting) || 'system'; }
    catch { return 'system'; }
  }

  function applyThemeSetting(setting: ThemeSetting) {
    if (setting === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      theme.init(prefersDark ? 'dark' : 'light');
    } else {
      theme.init(setting);
    }
    try { localStorage.setItem(LS_THEME_KEY, setting); } catch { /* ignore */ }
  }

  let themeSetting = $state<ThemeSetting>(loadThemeSetting());

  function onThemeChange(e: Event) {
    themeSetting = (e.target as HTMLSelectElement).value as ThemeSetting;
    applyThemeSetting(themeSetting);
  }

  // ── Network ───────────────────────────────────────────────────────────────

  let network = $state({
    dnsResolver:         'system' as DnsResolver,
    ipMode:              'dual'   as IpMode,
    poolingEnabled:      true,
    maxConnections:      10,
    maxConnectionsError: '',
    keepAlive:           true,
    retryStrategy:       'none'   as RetryStrategy,
    retryDelay:          1_000,
    retryDelayError:     '',
  });

  function validateMaxConnections() {
    const v = network.maxConnections;
    network.maxConnectionsError = (!Number.isInteger(v) || v < 1 || v > 100)
      ? 'Must be between 1 and 100' : '';
  }

  function validateRetryDelay() {
    const v = network.retryDelay;
    network.retryDelayError = (!Number.isInteger(v) || v < 0 || v > 600_000)
      ? 'Must be between 0 and 600000 ms' : '';
  }

  // ── Retry Options ─────────────────────────────────────────────────────────

  let retry = $state({
    enabled:    false,
    count:      3,
    countError: '',
    delay:      1_000,
    delayError: '',
  });

  function validateRetryCount() {
    const v = retry.count;
    retry.countError = (!Number.isInteger(v) || v < 1 || v > 10)
      ? 'Must be between 1 and 10' : '';
  }

  function validateRetryDelayField() {
    const v = retry.delay;
    retry.delayError = (!Number.isInteger(v) || v < 0 || v > 600_000)
      ? 'Must be between 0 and 600000 ms' : '';
  }

  // ── Timeouts ──────────────────────────────────────────────────────────────

  let timeouts = $state({
    global:  5_000,
    request: 10_000,
    dns:     2_000,
    geoip:   3_000,
  });

  // ── Proxy ─────────────────────────────────────────────────────────────────

  const PROXY_SCHEMES: Record<ProxyType, string> = {
    HTTP:   'http',
    HTTPS:  'https',
    SOCKS4: 'socks4',
    SOCKS5: 'socks5',
  };

  const PROXY_PLACEHOLDERS: Record<ProxyType, string> = {
    HTTP:   'http://host:8080',
    HTTPS:  'https://host:8080',
    SOCKS4: 'socks4://host:1080',
    SOCKS5: 'socks5://host:1080',
  };

  let proxy = $state({
    enabled:      false,
    type:         'HTTP' as ProxyType,
    url:          '',
    noProxy:      '',
    urlError:     '',
    noProxyError: '',
  });

  function validateProxy() {
    if (proxy.url) {
      const expected = PROXY_SCHEMES[proxy.type] + '://';
      if (!proxy.url.startsWith(expected)) {
        proxy.urlError = `URL must start with ${expected}`;
      } else {
        try { new URL(proxy.url); proxy.urlError = ''; }
        catch { proxy.urlError = 'Invalid URL'; }
      }
    } else {
      proxy.urlError = '';
    }
    if (proxy.noProxy) {
      const parts = proxy.noProxy.split(',').map(s => s.trim());
      proxy.noProxyError = parts.some(p => p === '') ? 'Invalid comma-separated list' : '';
    } else {
      proxy.noProxyError = '';
    }
  }

  // ── Tauri config bridge ───────────────────────────────────────────────────

  function toConfig(): AppConfig {
    return {
      network: {
        dns_resolver:    network.dnsResolver,
        ip_mode:         network.ipMode,
        pooling_enabled: network.poolingEnabled,
        max_connections: network.maxConnections,
        keep_alive:      network.keepAlive,
        retry_strategy:  network.retryStrategy,
        retry_delay_ms:  network.retryDelay,
      },
      proxy: {
        enabled:    proxy.enabled,
        proxy_type: proxy.type,
        url:        proxy.url,
        no_proxy:   proxy.noProxy,
      },
      timeouts: {
        global_ms:  timeouts.global,
        request_ms: timeouts.request,
        dns_ms:     timeouts.dns,
        geoip_ms:   timeouts.geoip,
      },
      retry: {
        enabled:  retry.enabled,
        count:    retry.count,
        delay_ms: retry.delay,
      },
      locale: localeSetting,
      theme:  themeSetting,
    };
  }

  function applyConfig(cfg: AppConfig) {
    network.dnsResolver    = cfg.network.dns_resolver as DnsResolver;
    network.ipMode         = cfg.network.ip_mode as IpMode;
    network.poolingEnabled = cfg.network.pooling_enabled;
    network.maxConnections = cfg.network.max_connections;
    network.keepAlive      = cfg.network.keep_alive;
    network.retryStrategy  = cfg.network.retry_strategy as RetryStrategy;
    network.retryDelay     = cfg.network.retry_delay_ms;

    proxy.enabled  = cfg.proxy.enabled;
    proxy.type     = cfg.proxy.proxy_type as ProxyType;
    proxy.url      = cfg.proxy.url;
    proxy.noProxy  = cfg.proxy.no_proxy;

    timeouts.global   = cfg.timeouts.global_ms;
    timeouts.request  = cfg.timeouts.request_ms;
    timeouts.dns      = cfg.timeouts.dns_ms;
    timeouts.geoip    = cfg.timeouts.geoip_ms;

    retry.enabled = cfg.retry.enabled;
    retry.count   = cfg.retry.count;
    retry.delay   = cfg.retry.delay_ms;

    if (cfg.locale === 'en' || cfg.locale === 'fr') {
      localeSetting = cfg.locale;
      i18n.setLocale(cfg.locale);
    }
    if (cfg.theme === 'light' || cfg.theme === 'dark' || cfg.theme === 'system') {
      themeSetting = cfg.theme as ThemeSetting;
      applyThemeSetting(cfg.theme as ThemeSetting);
    }
  }

  // Guard: do not save while the initial load is being applied.
  let saveEnabled = false;
  let syncTimer: ReturnType<typeof setTimeout> | undefined;

  function scheduleSync() {
    if (
      network.maxConnectionsError || network.retryDelayError ||
      retry.countError || retry.delayError ||
      proxy.urlError   || proxy.noProxyError
    ) return;
    clearTimeout(syncTimer);
    syncTimer = setTimeout(() => {
      invoke('set_config', { config: toConfig() }).catch(console.error);
    }, 300);
  }

  // Track all reactive state; save on any change after initial load.
  $effect(() => {
    void toConfig();
    if (!saveEnabled) return;
    scheduleSync();
  });

  onMount(async () => {
    try {
      const cfg = await invoke<AppConfig>('get_config');
      applyConfig(cfg);
    } catch (e) {
      console.error('[settings] get_config failed:', e);
    }
    // Wait for Svelte to flush effects triggered by applyConfig before
    // enabling saves, so the initial load does not write back to disk.
    await tick();
    saveEnabled = true;
  });
</script>

<div class="settings-page">
  <div class="page-header">
    <h1 class="page-title">Settings</h1>
  </div>

  <div class="sections">

    <section class="section">
      <h2 class="section-title">Network</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="dns-resolver">DNS resolver</label>
          <div class="input-wrap">
            <select id="dns-resolver" class="field-select" bind:value={network.dnsResolver}>
              <option value="system">System</option>
              <option value="cloudflare">Cloudflare (1.1.1.1)</option>
              <option value="google">Google (8.8.8.8)</option>
              <option value="quad9">Quad9 (9.9.9.9)</option>
            </select>
            <p class="field-hint">DNS server used to resolve hostnames during lookups.</p>
          </div>
        </div>

        <div class="field-row">
          <label class="field-label" for="ip-mode">IP mode</label>
          <div class="input-wrap">
            <select id="ip-mode" class="field-select" bind:value={network.ipMode}>
              <option value="ipv4">IPv4 only</option>
              <option value="ipv6">IPv6 only</option>
              <option value="dual">Dual stack</option>
            </select>
            <p class="field-hint">Address family preference when opening connections.</p>
          </div>
        </div>

        <div class="field-row">
          <label class="field-label" for="pooling-enabled">Connection pooling</label>
          <Toggle id="pooling-enabled" bind:checked={network.poolingEnabled} ariaLabel="Enable connection pooling" />
        </div>
        <p class="field-hint section-hint">Reuse TCP connections across requests to reduce latency.</p>

        {#if network.poolingEnabled}
          <div class="field-row">
            <label class="field-label" for="max-connections">Max connections</label>
            <div class="input-wrap">
              <input
                id="max-connections"
                class="field-input"
                class:invalid={!!network.maxConnectionsError}
                type="number"
                min="1"
                max="100"
                bind:value={network.maxConnections}
                oninput={validateMaxConnections}
              />
              {#if network.maxConnectionsError}
                <p class="field-error">{network.maxConnectionsError}</p>
              {:else}
                <p class="field-hint">Maximum number of simultaneous open connections (1–100).</p>
              {/if}
            </div>
          </div>
        {/if}

        <div class="field-row">
          <label class="field-label" for="keep-alive">Keep-alive</label>
          <Toggle id="keep-alive" bind:checked={network.keepAlive} ariaLabel="Enable keep-alive" />
        </div>
        <p class="field-hint section-hint">Send TCP keep-alive probes to detect stale connections.</p>

        <div class="field-row">
          <label class="field-label" for="retry-strategy">Retry strategy</label>
          <div class="input-wrap">
            <select id="retry-strategy" class="field-select" bind:value={network.retryStrategy}>
              <option value="none">None</option>
              <option value="linear">Linear</option>
              <option value="exponential">Exponential</option>
            </select>
            <p class="field-hint">Algorithm used to space out retries after a failed request.</p>
          </div>
        </div>

        {#if network.retryStrategy !== 'none'}
          <div class="field-row">
            <label class="field-label" for="retry-delay">Retry delay</label>
            <div class="input-wrap">
              <input
                id="retry-delay"
                class="field-input"
                class:invalid={!!network.retryDelayError}
                type="number"
                min="0"
                max="600000"
                bind:value={network.retryDelay}
                oninput={validateRetryDelay}
              />
              {#if network.retryDelayError}
                <p class="field-error">{network.retryDelayError}</p>
              {:else}
                <p class="field-hint">Base delay in milliseconds between retries (0–600000).</p>
              {/if}
            </div>
          </div>
        {/if}

      </div>
    </section>

    <section class="section">
      <h2 class="section-title">Proxy</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="proxy-enabled">Enable proxy</label>
          <Toggle id="proxy-enabled" bind:checked={proxy.enabled} ariaLabel="Enable proxy" />
        </div>

        {#if proxy.enabled}
          <div class="field-row">
            <label class="field-label" for="proxy-type">Proxy type</label>
            <div class="input-wrap">
              <select
                id="proxy-type"
                class="field-select"
                bind:value={proxy.type}
                onchange={validateProxy}
              >
                <option value="HTTP">HTTP</option>
                <option value="HTTPS">HTTPS</option>
                <option value="SOCKS4">SOCKS4</option>
                <option value="SOCKS5">SOCKS5</option>
              </select>
            </div>
          </div>

          <div class="field-row">
            <label class="field-label" for="proxy-url">Proxy URL</label>
            <div class="input-wrap">
              <input
                id="proxy-url"
                class="field-input"
                class:invalid={proxy.urlError}
                type="text"
                placeholder={PROXY_PLACEHOLDERS[proxy.type]}
                bind:value={proxy.url}
                oninput={validateProxy}
              />
              {#if proxy.urlError}
                <p class="field-error">{proxy.urlError}</p>
              {/if}
            </div>
          </div>

          <div class="field-row">
            <label class="field-label" for="proxy-noproxy">No-proxy domains</label>
            <div class="input-wrap">
              <input
                id="proxy-noproxy"
                class="field-input"
                class:invalid={proxy.noProxyError}
                type="text"
                placeholder="localhost, 127.0.0.1, .internal"
                bind:value={proxy.noProxy}
                oninput={validateProxy}
              />
              {#if proxy.noProxyError}
                <p class="field-error">{proxy.noProxyError}</p>
              {:else}
                <p class="field-hint">Comma-separated list of hosts to exclude from the proxy.</p>
              {/if}
            </div>
          </div>
        {/if}

      </div>
    </section>

    <section class="section">
      <h2 class="section-title">Timeouts</h2>
      <div class="section-body timeouts-body">
        <HybridDurationInput
          label="Global timeout"
          description="Maximum total time allowed for a full lookup operation."
          bind:value={timeouts.global}
        />
        <HybridDurationInput
          label="Request timeout"
          description="Maximum time allowed for a single HTTP request."
          bind:value={timeouts.request}
        />
        <HybridDurationInput
          label="DNS timeout"
          description="Maximum time allowed for a DNS resolution."
          bind:value={timeouts.dns}
        />
        <HybridDurationInput
          label="GeoIP timeout"
          description="Maximum time allowed for a GeoIP lookup."
          bind:value={timeouts.geoip}
        />
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">Theme</h2>
      <div class="section-body proxy-body">
        <div class="field-row">
          <label class="field-label" for="theme-select">Appearance</label>
          <div class="input-wrap">
            <select
              id="theme-select"
              class="field-select"
              value={themeSetting}
              onchange={onThemeChange}
            >
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="system">System</option>
            </select>
            <p class="field-hint">Controls the colour scheme of the application.</p>
          </div>
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">Language</h2>
      <div class="section-body proxy-body">
        <div class="field-row">
          <label class="field-label" for="locale-select">Language</label>
          <div class="input-wrap">
            <select
              id="locale-select"
              class="field-select"
              value={localeSetting}
              onchange={onLocaleChange}
            >
              {#each Object.entries(LOCALE_LABELS) as [value, label]}
                <option {value}>{label}</option>
              {/each}
            </select>
            <p class="field-hint">Controls the display language of the application.</p>
          </div>
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">Retry Options</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="retry-enabled">Enable auto-retry</label>
          <Toggle id="retry-enabled" bind:checked={retry.enabled} ariaLabel="Enable auto-retry" />
        </div>
        <p class="field-hint section-hint">Automatically retry failed requests without user intervention.</p>

        {#if retry.enabled}
          <div class="field-row">
            <label class="field-label" for="retry-count">Retry count</label>
            <div class="input-wrap">
              <input
                id="retry-count"
                class="field-input"
                class:invalid={!!retry.countError}
                type="number"
                min="1"
                max="10"
                bind:value={retry.count}
                oninput={validateRetryCount}
              />
              {#if retry.countError}
                <p class="field-error">{retry.countError}</p>
              {:else}
                <p class="field-hint">Number of additional attempts after the first failure (1–10).</p>
              {/if}
            </div>
          </div>

          <div class="field-row">
            <label class="field-label" for="retry-delay">Retry delay (ms)</label>
            <div class="input-wrap">
              <input
                id="retry-delay"
                class="field-input"
                class:invalid={!!retry.delayError}
                type="number"
                min="0"
                max="600000"
                bind:value={retry.delay}
                oninput={validateRetryDelayField}
              />
              {#if retry.delayError}
                <p class="field-error">{retry.delayError}</p>
              {:else}
                <p class="field-hint">Wait time in milliseconds before each retry attempt (0–600000).</p>
              {/if}
            </div>
          </div>
        {/if}

      </div>
    </section>

    <section class="section section--disabled" title="This feature is not available yet.">
      <h2 class="section-title">Advanced <span class="badge-soon">Soon</span></h2>
    </section>

  </div>
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
    padding: 24px 20px;
    gap: 1.5rem;
  }

  .page-header {
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 14px;
  }

  .page-title {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--color-text);
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .section--disabled {
    opacity: 0.45;
    pointer-events: none;
    cursor: not-allowed;
  }

  .badge-soon {
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
    vertical-align: middle;
    margin-left: 6px;
  }

  .section-title {
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    background: var(--color-header-bg);
    padding: 10px 14px;
    border-bottom: 1px solid var(--color-border);
  }

  .section-body {
    padding: 16px 14px;
    min-height: 48px;
  }

  .timeouts-body {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* ── Proxy section ── */
  .proxy-body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field-row {
    display: flex;
    align-items: flex-start;
    gap: 16px;
  }



  .field-label {
    font-size: 13px;
    color: var(--color-text);
    min-width: 150px;
    flex-shrink: 0;
    padding-top: 6px;
  }

  .input-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-input {
    width: 100%;
    padding: 7px 10px;
    font-size: 13px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .field-input:focus {
    border-color: var(--color-accent);
  }

  .field-input.invalid {
    border-color: var(--color-error);
  }

  .field-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .field-select {
    width: 100%;
    padding: 7px 10px;
    font-size: 13px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: var(--color-text);
    outline: none;
    cursor: pointer;
    transition: border-color var(--transition-fast);
  }

  .field-select:focus {
    border-color: var(--color-accent);
  }

  .field-error {
    font-size: 11.5px;
    color: var(--color-error);
  }

  .field-hint {
    font-size: 11.5px;
    color: var(--color-text-muted);
  }

  .section-hint {
    margin-left: 166px;
    margin-top: -8px;
  }

</style>
