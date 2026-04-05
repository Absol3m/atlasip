<script lang="ts">
  import { page } from '$app/state';
  import { Search, House, Moon, Sun } from 'lucide-svelte';
  import { theme } from '$lib/stores/app';

  const navLinks = [
    { href: '/',       label: 'Accueil', icon: House },
    { href: '/lookup', label: 'Analyse', icon: Search },
  ];
</script>

<nav class="navbar">
  <span class="brand">AtlasIP</span>

  <div class="nav-links">
    {#each navLinks as link}
      <a
        href={link.href}
        class="nav-link"
        class:active={page.url.pathname === link.href ||
          (link.href !== '/' && page.url.pathname.startsWith(link.href))}
      >
        <link.icon size={14} />
        {link.label}
      </a>
    {/each}
  </div>

  <button class="theme-toggle" onclick={() => theme.toggleTheme()} aria-label="Basculer le thème">
    {#if $theme === 'light'}
      <Moon size={16} />
    {:else}
      <Sun size={16} />
    {/if}
  </button>
</nav>

<style>
  .navbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0 1.25rem;
    height: 48px;
    background: var(--color-nav-bg);
    border-bottom: 1px solid var(--color-border);
    position: sticky;
    top: 0;
    z-index: 50;
    backdrop-filter: blur(8px);
  }

  .brand {
    font-size: 1.05rem;
    font-weight: 800;
    color: var(--color-accent);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin-right: 0.5rem;
  }

  .nav-links {
    display: flex;
    gap: 2px;
    flex: 1;
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--color-text-muted);
    text-decoration: none;
    transition: background-color 0.15s, color 0.15s;
  }

  .nav-link:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .nav-link.active {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
    color: var(--color-accent);
  }

  .theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    padding: 6px 8px;
    cursor: pointer;
    color: var(--color-text-muted);
    transition: background-color 0.15s, color 0.15s;
  }

  .theme-toggle:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }
</style>
