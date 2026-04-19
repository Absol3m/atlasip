<script lang="ts">
  import { page } from '$app/state';
  import { Search, Clock, Settings, Moon, Sun, PanelLeftClose, PanelLeftOpen, Info } from 'lucide-svelte';
  import { aboutModal } from '$lib/stores/about.svelte';
  import { theme } from '$lib/stores/app';
  import { i18n } from '$lib/services/i18n.svelte';

  interface NavLink {
    href: string;
    labelKey: Parameters<typeof i18n.t>[0];
    icon: typeof Search;
  }

  const links: NavLink[] = [
    { href: '/lookup',   labelKey: 'nav.analysis', icon: Search   },
    { href: '/history',  labelKey: 'nav.history',  icon: Clock    },
    { href: '/settings', labelKey: 'nav.settings', icon: Settings },
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
    {#if !collapsed}
      <div class="collapse-wrap">
        <button
          class="collapse-btn"
          onclick={() => (collapsed = true)}
          aria-label="Collapse sidebar"
          title="Collapse sidebar"
        >
          <PanelLeftClose size={15} />
        </button>
      </div>
      <span class="logo" aria-hidden="true"></span>
      <span class="brand-name">ATLAS<span class="brand-dot">•</span>IP</span>
    {:else}
      <button
        class="expand-btn"
        onclick={() => (collapsed = false)}
        aria-label="Expand sidebar"
        title="Expand sidebar"
      >
        <PanelLeftOpen size={15} />
      </button>
    {/if}
  </div>

  <!-- Navigation -->
  <nav class="nav" aria-label="Main navigation">
    {#each links as link}
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

    {#if i18n.version}
      <button class="version" onclick={aboutModal.show} title="v{i18n.version}">
        <Info size={13} />
        {#if !collapsed}<span>v{i18n.version}</span>{/if}
      </button>
    {/if}
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
    padding: 18px 12px 16px;
    border-bottom: 1px solid var(--color-border);
    min-height: 63px;
    overflow: hidden;
  }

  .collapsed .brand {
    justify-content: center;
    padding: 16px 0;
  }

  /* Wrapper that slides in from the left on hover, pushing logo+name right */
  .collapse-wrap {
    max-width: 0;
    overflow: hidden;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    transition: max-width 0.18s ease;
  }

  .brand:hover .collapse-wrap {
    max-width: 36px; /* 24px button + 8px right margin + 4px breathing room */
  }

  .collapse-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    margin-right: 8px;
    border: none;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
  }

  .collapse-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .expand-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
  }

  .expand-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
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
    margin-left: 10px;
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
    background: none;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    width: 100%;
    text-align: left;
    transition: opacity var(--transition-fast), background-color var(--transition-fast);
  }

  .version:hover {
    opacity: 1;
    background: var(--color-hover);
  }

  .collapsed .version {
    justify-content: center;
    padding: 4px 0;
  }
</style>
