<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { X } from 'lucide-svelte';
  import { i18n } from '$lib/services/i18n.svelte';

  interface Props { onclose: () => void }
  let { onclose }: Props = $props();

  let maxmindAccountId = $state('');
  let maxmindKey       = $state('');
  let downloading      = $state(false);
  let error            = $state('');

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onclose();
  }

  async function save() {
    error = '';
    if (maxmindAccountId.trim() && maxmindKey.trim()) {
      downloading = true;
      try {
        const cfg = await invoke<Record<string, unknown>>('get_config');
        await invoke('set_config', { config: { ...cfg, maxmind_account_id: maxmindAccountId.trim(), maxmind_license_key: maxmindKey.trim(), first_launch: false } });
        await invoke('download_geoip', { accountId: maxmindAccountId.trim(), licenseKey: maxmindKey.trim() });
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        downloading = false;
        return;
      }
      downloading = false;
    } else {
      // Skip: just mark first_launch done
      try {
        const cfg = await invoke<Record<string, unknown>>('get_config');
        await invoke('set_config', { config: { ...cfg, first_launch: false } });
      } catch { /* non-blocking */ }
    }
    onclose();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="backdrop"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  onclick={onBackdropClick}
  onkeydown={onKeydown}
>
  <div class="modal">
    <button class="close-btn" onclick={onclose} aria-label={i18n.t('about.close')}>
      <X size={16} />
    </button>

    <div class="modal-header">
      <span class="app-logo" aria-hidden="true"></span>
      <div>
        <h2 class="modal-title">{i18n.t('firstlaunch.title')}</h2>
        <p class="modal-subtitle">{i18n.t('firstlaunch.subtitle')}</p>
      </div>
    </div>

    <div class="service-block">
      <h3 class="service-title">{i18n.t('firstlaunch.maxmind.title')}</h3>
      <p class="service-desc">{i18n.t('firstlaunch.maxmind.desc')}</p>
      <button
        class="get-key-link"
        onclick={() => openUrl('https://www.maxmind.com/en/geolite2/signup')}
      >{i18n.t('firstlaunch.maxmind.get_key')}</button>
      <input
        class="key-input"
        type="text"
        placeholder={i18n.t('firstlaunch.maxmind.account_placeholder')}
        bind:value={maxmindAccountId}
      />
      <input
        class="key-input"
        type="password"
        placeholder={i18n.t('firstlaunch.maxmind.placeholder')}
        bind:value={maxmindKey}
      />
      {#if error}
        <p class="key-error">{error}</p>
      {/if}
    </div>

    <div class="modal-actions">
      <button class="btn-skip" onclick={onclose}>{i18n.t('firstlaunch.skip')}</button>
      <button class="btn-save" onclick={save} disabled={downloading}>
        {downloading ? '…' : i18n.t('firstlaunch.save')}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal {
    position: relative;
    width: 460px;
    max-width: calc(100vw - 32px);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
    padding: 28px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .close-btn {
    position: absolute;
    top: 14px; right: 14px;
    display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px;
    border: none; border-radius: var(--radius-md);
    background: none; color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast);
  }
  .close-btn:hover { background: var(--color-hover); color: var(--color-text); }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .app-logo {
    display: block;
    width: 36px; height: 36px;
    flex-shrink: 0;
    background-color: #144379;
    -webkit-mask: url('/img/logo.svg') no-repeat center / contain;
    mask: url('/img/logo.svg') no-repeat center / contain;
  }
  :global([data-theme='dark']) .app-logo { background-color: #ffffff; }

  .modal-title {
    font-size: 1.05rem;
    font-weight: 800;
    color: var(--color-text);
    line-height: 1.2;
  }

  .modal-subtitle {
    font-size: 12.5px;
    color: var(--color-text-muted);
    margin-top: 3px;
  }

  .service-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .service-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--color-text);
  }

  .service-desc {
    font-size: 12px;
    color: var(--color-text-muted);
    line-height: 1.5;
  }

  .get-key-link {
    align-self: flex-start;
    font-size: 12px;
    color: var(--color-accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
  }

  .key-input {
    width: 100%;
    padding: 7px 10px;
    font-size: 13px;
    font-family: var(--font-mono);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    outline: none;
    transition: border-color var(--transition-fast);
  }
  .key-input:focus { border-color: var(--color-accent); }

  .key-error {
    font-size: 11.5px;
    color: var(--color-error);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .btn-skip {
    padding: 7px 16px;
    font-size: 13px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background-color var(--transition-fast);
  }
  .btn-skip:hover { background: var(--color-hover); }

  .btn-save {
    padding: 7px 16px;
    font-size: 13px;
    font-weight: 600;
    border: none;
    border-radius: var(--radius-md);
    background: var(--color-accent);
    color: #fff;
    cursor: pointer;
    transition: background-color var(--transition-fast);
  }
  .btn-save:hover:not(:disabled) { background: var(--color-accent-hover); }
  .btn-save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
