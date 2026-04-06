<script lang="ts">
  import { page } from '$app/state';
  import { Search, Clock, Settings, Info, Moon, Sun, PanelLeftClose, PanelLeftOpen } from 'lucide-svelte';
  import { theme } from '$lib/stores/app';
  import { i18n } from '$lib/services/i18n.svelte';

  interface NavLink {
    href: string;
    labelKey: Parameters<typeof i18n.t>[0];
    icon: typeof Search;
    enabled: boolean;
  }

  const links: NavLink[] = [
    { href: '/lookup',   labelKey: 'nav.analysis', icon: Search,   enabled: true  },
    { href: '/history',  labelKey: 'nav.history',  icon: Clock,    enabled: true  },
    { href: '/settings', labelKey: 'nav.settings', icon: Settings, enabled: true  },
    { href: '/about',    labelKey: 'nav.about',    icon: Info,     enabled: false },
  ];

  let collapsed = $state(false);

  function isActive(href: string): boolean {
    if (href === '/lookup') {
      return page.url.pathname === href || page.url.pathname.startsWith('/ip/');
    }
    return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
  }
</script>

<aside class="sidebar" class:collapsed>
  <!-- Brand -->
  <div class="brand">
    <span class="logo" aria-hidden="true"></span>
    {#if !collapsed}
      <span class="brand-name">ATLAS<span class="brand-dot">•</span>IP</span>
    {/if}
    <button
      class="collapse-btn"
      onclick={() => collapsed = !collapsed}
      aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      {#if collapsed}
        <PanelLeftOpen size={15} />
      {:else}
        <PanelLeftClose size={15} />
      {/if}
    </button>
  </div>

  <!-- Navigation -->
  <nav class="nav" aria-label="Main navigation">
    {#each links as link}
      {#if link.enabled}
        <a
          href={link.href}
          class="nav-item"
          class:active={isActive(link.href)}
          aria-current={isActive(link.href) ? 'page' : undefined}
          title={collapsed ? i18n.t(link.labelKey) : undefined}
        >
          <link.icon size={16} />
          {#if !collapsed}<span>{i18n.t(link.labelKey)}</span>{/if}
        </a>
      {:else}
        <span
          class="nav-item nav-item--disabled"
          title={collapsed ? i18n.t(link.labelKey) : 'Coming soon'}
        >
          <link.icon size={16} />
          {#if !collapsed}
            <span>{i18n.t(link.labelKey)}</span>
            <span class="badge-soon">soon</span>
          {/if}
        </span>
      {/if}
    {/each}
  </nav>

  <!-- Bottom section -->
  <div class="sidebar-bottom">
    <button
      class="theme-toggle"
      onclick={() => theme.toggleTheme()}
      aria-label="Toggle theme"
      title={collapsed ? ($theme === 'light' ? 'Dark mode' : 'Light mode') : undefined}
    >
      {#if $theme === 'light'}
        <Moon size={15} />
        {#if !collapsed}<span>Dark mode</span>{/if}
      {:else}
        <Sun size={15} />
        {#if !collapsed}<span>Light mode</span>{/if}
      {/if}
    </button>

    <div class="version" title="v{i18n.t('app.version')}">
      <Info size={13} />
      {#if !collapsed}<span>v{i18n.t('app.version')}</span>{/if}
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: 240px;
    flex-shrink: 0;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    overflow: hidden;
    position: sticky;
    top: 0;
    transition: width 0.2s ease;
  }

  .sidebar.collapsed {
    width: 52px;
  }

  /* ── Brand ── */
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 18px 12px 16px;
    border-bottom: 1px solid var(--color-border);
    min-height: 63px;
  }

  .collapsed .brand {
    flex-direction: column;
    justify-content: center;
    padding: 10px 0;
    gap: 6px;
  }

  .logo {
    display: block;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    background-color: #144379;
    -webkit-mask: url('/img/logo.svg') no-repeat center / contain;
    mask: url('/img/logo.svg') no-repeat center / contain;
  }

  :global([data-theme='dark']) .logo {
    background-color: #ffffff;
  }

  .brand-name {
    flex: 1;
    font-size: 1.1rem;
    font-weight: 800;
    color: #144379;
    letter-spacing: 0.06em;
    white-space: nowrap;
  }

  :global([data-theme='dark']) .brand-name {
    color: #ffffff;
  }

  .brand-dot {
    color: var(--color-accent);
  }

  /* ── Collapse button ── */
  .collapse-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: 4px;
    border: none;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
    margin-left: auto;
  }

  .collapsed .collapse-btn {
    margin-left: 0;
  }

  .collapse-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  /* ── Navigation ── */
  .nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 6px;
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-lg);
    font-size: 13.5px;
    font-weight: 500;
    color: var(--color-text-muted);
    text-decoration: none;
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
    user-select: none;
    white-space: nowrap;
    overflow: hidden;
  }

  .collapsed .nav-item {
    justify-content: center;
    padding: 8px 0;
  }

  a.nav-item:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .nav-item.active {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    color: var(--color-accent);
  }

  .nav-item--disabled {
    opacity: 0.42;
    cursor: not-allowed;
  }

  .badge-soon {
    margin-left: auto;
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
  }

  /* ── Bottom ── */
  .sidebar-bottom {
    padding: 12px 6px;
    border-top: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .theme-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-lg);
    border: none;
    background: none;
    font-size: 13.5px;
    color: var(--color-text-muted);
    cursor: pointer;
    text-align: left;
    transition: background-color var(--transition-fast), color var(--transition-fast);
    white-space: nowrap;
    overflow: hidden;
  }

  .collapsed .theme-toggle {
    justify-content: center;
    padding: 8px 0;
  }

  .theme-toggle:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .version {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-text-muted);
    padding: 4px 10px;
    opacity: 0.6;
    white-space: nowrap;
    overflow: hidden;
    cursor: default;
  }

  .collapsed .version {
    justify-content: center;
    padding: 4px 0;
  }
</style>
