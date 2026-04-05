<script lang="ts">
  import { page } from '$app/state';
  import { Search, Clock, Settings, Info, Moon, Sun } from 'lucide-svelte';
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
    { href: '/history',  labelKey: 'nav.history',  icon: Clock,    enabled: false },
    { href: '/settings', labelKey: 'nav.settings', icon: Settings, enabled: false },
    { href: '/about',    labelKey: 'nav.about',    icon: Info,     enabled: false },
  ];

  function isActive(href: string): boolean {
    if (href === '/lookup') {
      return page.url.pathname === href || page.url.pathname.startsWith('/ip/');
    }
    return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
  }
</script>

<aside class="sidebar">
  <!-- Brand -->
  <div class="brand">
    <span class="logo" aria-hidden="true"></span>
    <span class="brand-name">ATLAS<span class="brand-dot">•</span>IP</span>
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
        >
          <link.icon size={16} />
          <span>{i18n.t(link.labelKey)}</span>
        </a>
      {:else}
        <span class="nav-item nav-item--disabled" title="Coming soon">
          <link.icon size={16} />
          <span>{i18n.t(link.labelKey)}</span>
          <span class="badge-soon">soon</span>
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
      title="Toggle theme"
    >
      {#if $theme === 'light'}
        <Moon size={15} />
        <span>Dark mode</span>
      {:else}
        <Sun size={15} />
        <span>Light mode</span>
      {/if}
    </button>

    <div class="version">
      v{i18n.t('app.version')}
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
  }

  /* ── Brand ── */
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 18px 20px 16px;
    border-bottom: 1px solid var(--color-border);
  }

  /* Logo via CSS mask so fill color is controlled by background-color,
     regardless of the hardcoded fill in the SVG source. */
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
    font-size: 1.1rem;
    font-weight: 800;
    color: #144379;
    letter-spacing: 0.06em;
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
    padding: 12px 10px;
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 7px;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--color-text-muted);
    text-decoration: none;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
    user-select: none;
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
    border-radius: 4px;
    padding: 1px 5px;
  }

  /* ── Bottom ── */
  .sidebar-bottom {
    padding: 12px 10px;
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
    padding: 8px 12px;
    border-radius: 7px;
    border: none;
    background: none;
    font-size: 13.5px;
    color: var(--color-text-muted);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.15s, color 0.15s;
  }

  .theme-toggle:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .version {
    font-size: 11px;
    color: var(--color-text-muted);
    padding: 0 12px;
    opacity: 0.6;
  }
</style>
