<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { syncBackendConfig } from '$lib/services/atlasip';
  import HybridDurationInput from '$lib/components/HybridDurationInput.svelte';
  import Toggle from '$lib/components/Toggle.svelte';
  import { theme } from '$lib/stores/app';
  import { i18n } from '$lib/services/i18n.svelte';
  // ── Types ─────────────────────────────────────────────────────────────────

  type LocaleSetting  = 'en-US' | 'fr-FR';
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
    locale:               string;
    theme:                string;
    autostart:            boolean;
    dns_transport:        string;
    maxmind_account_id:   string | null;
    maxmind_license_key:  string | null;
    first_launch:         boolean;
  }

  // ── Locale ────────────────────────────────────────────────────────────────

  const LOCALE_LABELS: Record<LocaleSetting, string> = {
    'en-US': 'English',
    'fr-FR': 'Français',
  };

  let localeSetting        = $state<LocaleSetting>('en-US');
  let autostart            = $state(false);
  let dnsTransport         = $state('auto');
  let maxmindAccountId     = $state('');
  let maxmindKey           = $state('');
  let maxmindStatus        = $state<'missing' | 'ok' | 'outdated'>('missing');
  let maxmindDownloading   = $state(false);
  let maxmindError         = $state('');

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
      ? i18n.t('settings.error.max_connections') : '';
  }

  function validateRetryDelay() {
    const v = network.retryDelay;
    network.retryDelayError = (!Number.isInteger(v) || v < 0 || v > 600_000)
      ? i18n.t('settings.error.retry_delay') : '';
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
      ? i18n.t('settings.error.retry_count') : '';
  }

  function validateRetryDelayField() {
    const v = retry.delay;
    retry.delayError = (!Number.isInteger(v) || v < 0 || v > 600_000)
      ? i18n.t('settings.error.retry_delay_ms') : '';
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
        proxy.urlError = i18n.t('settings.error.proxy_url_scheme').replace('{scheme}', expected);
      } else {
        try { new URL(proxy.url); proxy.urlError = ''; }
        catch { proxy.urlError = i18n.t('settings.error.proxy_url_invalid'); }
      }
    } else {
      proxy.urlError = '';
    }
    if (proxy.noProxy) {
      const parts = proxy.noProxy.split(',').map(s => s.trim());
      proxy.noProxyError = parts.some(p => p === '') ? i18n.t('settings.error.no_proxy_list') : '';
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
      locale:              localeSetting,
      theme:               themeSetting,
      autostart:            autostart,
      dns_transport:        dnsTransport,
      maxmind_account_id:   maxmindAccountId || null,
      maxmind_license_key:  maxmindKey || null,
      first_launch:         false,
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

    if (cfg.locale === 'en-US' || cfg.locale === 'fr-FR') {
      localeSetting = cfg.locale;
    }
    if (cfg.theme === 'light' || cfg.theme === 'dark' || cfg.theme === 'system') {
      themeSetting = cfg.theme as ThemeSetting;
      applyThemeSetting(cfg.theme as ThemeSetting);
    }
    autostart        = cfg.autostart ?? false;
    dnsTransport     = cfg.dns_transport ?? 'auto';
    maxmindAccountId = cfg.maxmind_account_id ?? '';
    maxmindKey       = cfg.maxmind_license_key ?? '';
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
      const cfg = toConfig();
      invoke('set_config', { config: cfg }).catch(console.error);
      syncBackendConfig({
        locale:              cfg.locale,
        dns_timeout_ms:      cfg.timeouts.dns_ms,
        rdap_timeout_ms:     cfg.timeouts.request_ms,
        whois_timeout_ms:    cfg.timeouts.request_ms,
        maxmind_account_id:  cfg.maxmind_account_id,
        maxmind_license_key: cfg.maxmind_license_key,
      });
    }, 300);
  }

  // Track all reactive state; save on any change after initial load.
  $effect(() => {
    void toConfig();
    if (!saveEnabled) return;
    scheduleSync();
  });

  async function refreshGeoIpStatus() {
    maxmindStatus = await invoke<'missing' | 'ok' | 'outdated'>('geoip_db_status');
  }

  async function downloadGeoIp() {
    if (!maxmindAccountId.trim() || !maxmindKey.trim()) return;
    maxmindDownloading = true;
    maxmindError = '';
    try {
      await invoke('download_geoip', { accountId: maxmindAccountId.trim(), licenseKey: maxmindKey.trim() });
      await refreshGeoIpStatus();
    } catch (e) {
      maxmindError = e instanceof Error ? e.message : String(e);
    } finally {
      maxmindDownloading = false;
    }
  }

  onMount(async () => {
    try {
      const cfg = await invoke<AppConfig>('get_config');
      applyConfig(cfg);
    } catch (e) {
      console.error('[settings] get_config failed:', e);
    }
    await refreshGeoIpStatus();
    // Wait for Svelte to flush effects triggered by applyConfig before
    // enabling saves, so the initial load does not write back to disk.
    await tick();
    saveEnabled = true;
  });
</script>

<div class="settings-page">
  <div class="page-header">
    <h1 class="page-title">{i18n.t('settings.title')}</h1>
  </div>

  <div class="sections">

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.network')}</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="dns-resolver">{i18n.t('settings.field.dns_resolver')}</label>
          <div class="input-wrap">
            <select id="dns-resolver" class="field-select" bind:value={network.dnsResolver}>
              <option value="system">{i18n.t('settings.opt.dns.system')}</option>
              <option value="cloudflare">{i18n.t('settings.opt.dns.cloudflare')}</option>
              <option value="google">{i18n.t('settings.opt.dns.google')}</option>
              <option value="quad9">{i18n.t('settings.opt.dns.quad9')}</option>
            </select>
            <p class="field-hint">{i18n.t('settings.hint.dns_resolver')}</p>
          </div>
        </div>

        <div class="field-row">
          <label class="field-label" for="dns-transport">{i18n.t('settings.field.dns_transport')}</label>
          <div class="input-wrap">
            <select id="dns-transport" class="field-select" bind:value={dnsTransport}>
              <option value="auto">{i18n.t('settings.opt.dns_transport.auto')}</option>
              <option value="system">{i18n.t('settings.opt.dns_transport.system')}</option>
              <option value="doh">{i18n.t('settings.opt.dns_transport.doh')}</option>
              <option value="dot">{i18n.t('settings.opt.dns_transport.dot')}</option>
            </select>
            <p class="field-hint">{i18n.t('settings.hint.dns_transport')}</p>
          </div>
        </div>

        <div class="field-row">
          <label class="field-label" for="ip-mode">{i18n.t('settings.field.ip_mode')}</label>
          <div class="input-wrap">
            <select id="ip-mode" class="field-select" bind:value={network.ipMode}>
              <option value="ipv4">{i18n.t('settings.opt.ip.ipv4')}</option>
              <option value="ipv6">{i18n.t('settings.opt.ip.ipv6')}</option>
              <option value="dual">{i18n.t('settings.opt.ip.dual')}</option>
            </select>
            <p class="field-hint">{i18n.t('settings.hint.ip_mode')}</p>
          </div>
        </div>

        <div class="field-row">
          <label class="field-label" for="pooling-enabled">{i18n.t('settings.field.pooling')}</label>
          <Toggle id="pooling-enabled" bind:checked={network.poolingEnabled} ariaLabel="Enable connection pooling" />
        </div>
        <p class="field-hint section-hint">{i18n.t('settings.hint.pooling')}</p>

        {#if network.poolingEnabled}
          <div class="field-row">
            <label class="field-label" for="max-connections">{i18n.t('settings.field.max_connections')}</label>
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
                <p class="field-hint">{i18n.t('settings.hint.max_connections')}</p>
              {/if}
            </div>
          </div>
        {/if}

        <div class="field-row">
          <label class="field-label" for="keep-alive">{i18n.t('settings.field.keep_alive')}</label>
          <Toggle id="keep-alive" bind:checked={network.keepAlive} ariaLabel="Enable keep-alive" />
        </div>
        <p class="field-hint section-hint">{i18n.t('settings.hint.keep_alive')}</p>

        <div class="field-row">
          <label class="field-label" for="retry-strategy">{i18n.t('settings.field.retry_strategy')}</label>
          <div class="input-wrap">
            <select id="retry-strategy" class="field-select" bind:value={network.retryStrategy}>
              <option value="none">{i18n.t('settings.opt.retry.none')}</option>
              <option value="linear">{i18n.t('settings.opt.retry.linear')}</option>
              <option value="exponential">{i18n.t('settings.opt.retry.exponential')}</option>
            </select>
            <p class="field-hint">{i18n.t('settings.hint.retry_strategy')}</p>
          </div>
        </div>

        {#if network.retryStrategy !== 'none'}
          <div class="field-row">
            <label class="field-label" for="retry-delay">{i18n.t('settings.field.retry_delay')}</label>
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
                <p class="field-hint">{i18n.t('settings.hint.retry_delay')}</p>
              {/if}
            </div>
          </div>
        {/if}

      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.proxy')}</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="proxy-enabled">{i18n.t('settings.field.proxy_enabled')}</label>
          <Toggle id="proxy-enabled" bind:checked={proxy.enabled} ariaLabel="Enable proxy" />
        </div>

        {#if proxy.enabled}
          <div class="field-row">
            <label class="field-label" for="proxy-type">{i18n.t('settings.field.proxy_type')}</label>
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
            <label class="field-label" for="proxy-url">{i18n.t('settings.field.proxy_url')}</label>
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
            <label class="field-label" for="proxy-noproxy">{i18n.t('settings.field.no_proxy')}</label>
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
                <p class="field-hint">{i18n.t('settings.hint.no_proxy')}</p>
              {/if}
            </div>
          </div>
        {/if}

      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.timeouts')}</h2>
      <div class="section-body timeouts-body">
        <HybridDurationInput
          label={i18n.t('settings.field.timeout_global')}
          description={i18n.t('settings.hint.timeout_global')}
          bind:value={timeouts.global}
        />
        <HybridDurationInput
          label={i18n.t('settings.field.timeout_request')}
          description={i18n.t('settings.hint.timeout_request')}
          bind:value={timeouts.request}
        />
        <HybridDurationInput
          label={i18n.t('settings.field.timeout_dns')}
          description={i18n.t('settings.hint.timeout_dns')}
          bind:value={timeouts.dns}
        />
        <HybridDurationInput
          label={i18n.t('settings.field.timeout_geoip')}
          description={i18n.t('settings.hint.timeout_geoip')}
          bind:value={timeouts.geoip}
        />
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.theme')}</h2>
      <div class="section-body proxy-body">
        <div class="field-row">
          <label class="field-label" for="theme-select">{i18n.t('settings.field.appearance')}</label>
          <div class="input-wrap">
            <select
              id="theme-select"
              class="field-select"
              value={themeSetting}
              onchange={onThemeChange}
            >
              <option value="light">{i18n.t('settings.opt.theme.light')}</option>
              <option value="dark">{i18n.t('settings.opt.theme.dark')}</option>
              <option value="system">{i18n.t('settings.opt.theme.system')}</option>
            </select>
            <p class="field-hint">{i18n.t('settings.hint.appearance')}</p>
          </div>
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.language')}</h2>
      <div class="section-body proxy-body">
        <div class="field-row">
          <label class="field-label" for="locale-select">{i18n.t('settings.field.language')}</label>
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
            <p class="field-hint">{i18n.t('settings.hint.language')}</p>
          </div>
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.behavior')}</h2>
      <div class="section-body proxy-body">
        <div class="field-row">
          <label class="field-label" for="autostart">{i18n.t('settings.field.autostart')}</label>
          <Toggle id="autostart" bind:checked={autostart} ariaLabel="Launch at startup" />
        </div>
        <p class="field-hint section-hint">{i18n.t('settings.hint.autostart')}</p>
      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.apikeys')}</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="maxmind-account">{i18n.t('settings.field.maxmind_account_id')}</label>
          <div class="input-wrap">
            <input
              id="maxmind-account"
              class="field-input"
              type="text"
              placeholder={i18n.t('settings.maxmind.account_placeholder')}
              bind:value={maxmindAccountId}
            />
            <p class="field-hint">{i18n.t('settings.hint.maxmind_account_id')}</p>
          </div>
        </div>

        <div class="field-row">
          <label class="field-label" for="maxmind-key">{i18n.t('settings.field.maxmind_key')}</label>
          <div class="input-wrap">
            <input
              id="maxmind-key"
              class="field-input"
              type="password"
              placeholder={i18n.t('settings.maxmind.placeholder')}
              bind:value={maxmindKey}
            />
            <p class="field-hint">
              {i18n.t('settings.hint.maxmind_key')}
              <!-- svelte-ignore a11y_invalid_attribute -->
              <a href="#" onclick={(e) => { e.preventDefault(); import('@tauri-apps/plugin-opener').then(m => m.openUrl('https://www.maxmind.com/en/geolite2/signup')); }} class="hint-link">
                {i18n.t('settings.maxmind.get_key')}
              </a>
            </p>
          </div>
        </div>

        <div class="field-row">
          <span class="field-label">{i18n.t('settings.field.maxmind_status')}</span>
          <div class="input-wrap geoip-status-wrap">
            <span class="geoip-status geoip-status--{maxmindStatus}">
              {i18n.t(`settings.maxmind.status.${maxmindStatus}`)}
            </span>
            {#if maxmindStatus !== 'ok'}
              <button
                class="btn-download"
                onclick={downloadGeoIp}
                disabled={maxmindDownloading || !maxmindAccountId.trim() || !maxmindKey.trim()}
              >
                {maxmindDownloading
                  ? i18n.t('settings.maxmind.downloading')
                  : i18n.t('settings.maxmind.download')}
              </button>
            {/if}
            {#if maxmindError}
              <p class="field-error">{maxmindError}</p>
            {/if}
          </div>
        </div>

      </div>
    </section>

    <section class="section">
      <h2 class="section-title">{i18n.t('settings.section.retry')}</h2>
      <div class="section-body proxy-body">

        <div class="field-row">
          <label class="field-label" for="retry-enabled">{i18n.t('settings.field.retry_enabled')}</label>
          <Toggle id="retry-enabled" bind:checked={retry.enabled} ariaLabel="Enable auto-retry" />
        </div>
        <p class="field-hint section-hint">{i18n.t('settings.hint.retry_enabled')}</p>

        {#if retry.enabled}
          <div class="field-row">
            <label class="field-label" for="retry-count">{i18n.t('settings.field.retry_count')}</label>
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
                <p class="field-hint">{i18n.t('settings.hint.retry_count')}</p>
              {/if}
            </div>
          </div>

          <div class="field-row">
            <label class="field-label" for="retry-delay">{i18n.t('settings.field.retry_delay_ms')}</label>
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
                <p class="field-hint">{i18n.t('settings.hint.retry_delay_ms')}</p>
              {/if}
            </div>
          </div>
        {/if}

      </div>
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

  .hint-link {
    color: var(--color-accent);
    text-decoration: none;
  }
  .hint-link:hover { text-decoration: underline; }

  .geoip-status-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .geoip-status {
    font-size: 11.5px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-full);
  }
  .geoip-status--ok       { background: color-mix(in srgb, var(--color-success) 15%, transparent); color: var(--color-success); }
  .geoip-status--missing  { background: color-mix(in srgb, var(--color-text-muted) 15%, transparent); color: var(--color-text-muted); }
  .geoip-status--outdated { background: color-mix(in srgb, var(--color-warning) 15%, transparent); color: var(--color-warning-text); }

  .btn-download {
    font-size: 12px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text);
    cursor: pointer;
    transition: background-color var(--transition-fast);
  }
  .btn-download:hover:not(:disabled) { background: var(--color-hover); }
  .btn-download:disabled { opacity: 0.45; cursor: not-allowed; }

</style>
