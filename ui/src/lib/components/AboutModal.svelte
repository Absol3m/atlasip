<script lang="ts">
  import { aboutModal } from '$lib/stores/about.svelte';
  import { i18n } from '$lib/services/i18n.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { X, ExternalLink } from 'lucide-svelte';

  const features = [
    'about.feature.1',
    'about.feature.2',
    'about.feature.3',
    'about.feature.4',
  ] as const;

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) aboutModal.hide();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') aboutModal.hide();
  }

  function openGithub() {
    openUrl(i18n.t('about.github.url'));
  }
</script>

{#if aboutModal.open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="backdrop"
    role="dialog"
    aria-modal="true"
    aria-label={i18n.t('about.close')}
    tabindex="-1"
    onclick={onBackdropClick}
    onkeydown={onKeydown}
  >
    <div class="modal">
      <button class="close-btn" onclick={aboutModal.hide} aria-label={i18n.t('about.close')}>
        <X size={16} />
      </button>

      <div class="modal-header">
        <span class="app-logo" aria-hidden="true"></span>
        <div class="app-identity">
          <h2 class="app-name">ATLAS<span class="dot">•</span>IP</h2>
          {#if i18n.version}
            <span class="app-version">v{i18n.version}</span>
          {/if}
        </div>
      </div>

      <p class="description">{i18n.t('about.description')}</p>

      <div class="section">
        <h3 class="section-title">{i18n.t('about.features.title')}</h3>
        <ul class="feature-list">
          {#each features as key}
            <li>{i18n.t(key)}</li>
          {/each}
        </ul>
      </div>

      <div class="divider"></div>

      <button class="github-link" onclick={openGithub}>
        <span>{i18n.t('about.github.label')}</span>
        <ExternalLink size={12} />
      </button>

      <div class="divider"></div>

      <div class="meta">
        <p>{i18n.t('about.credits')}</p>
        <p>{i18n.t('about.license')}</p>
      </div>

      <p class="legal">{i18n.t('about.legal')}</p>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal {
    position: relative;
    width: 480px;
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 64px);
    overflow-y: auto;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
    padding: 28px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .close-btn {
    position: absolute;
    top: 14px;
    right: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
  }

  .close-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .app-logo {
    display: block;
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    background-color: #144379;
    -webkit-mask: url('/img/logo.svg') no-repeat center / contain;
    mask: url('/img/logo.svg') no-repeat center / contain;
  }

  :global([data-theme='dark']) .app-logo {
    background-color: #ffffff;
  }

  .app-identity {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .app-name {
    font-size: 1.25rem;
    font-weight: 800;
    color: #144379;
    letter-spacing: 0.06em;
    line-height: 1;
  }

  :global([data-theme='dark']) .app-name {
    color: #ffffff;
  }

  .dot {
    color: var(--color-accent);
  }

  .app-version {
    font-size: 11.5px;
    font-family: var(--font-mono);
    color: var(--color-text-muted);
  }

  .description {
    font-size: 13.5px;
    color: var(--color-text-muted);
    line-height: 1.55;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-muted);
  }

  .feature-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .feature-list li {
    font-size: 13px;
    color: var(--color-text);
    padding-left: 14px;
    position: relative;
  }

  .feature-list li::before {
    content: '–';
    position: absolute;
    left: 0;
    color: var(--color-accent);
  }

  .divider {
    height: 1px;
    background: var(--color-border);
  }

  .github-link {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .github-link:hover {
    color: var(--color-accent-hover);
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .meta p {
    font-size: 12px;
    color: var(--color-text-muted);
  }

  .legal {
    font-size: 11.5px;
    color: var(--color-text-muted);
    opacity: 0.7;
    line-height: 1.5;
    font-style: italic;
  }
</style>
