import { vi, describe, it, expect, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

import { i18n } from './i18n.svelte';

beforeEach(() => {
  i18n.setLocale('en-US');
  vi.clearAllMocks();
});

// ── appel translate() — i18n.t() returns correct translations ─────────────────

describe('i18n — translate (appel translate)', () => {
  it('t() returns the translation for a known en-US key', () => {
    expect(i18n.t('nav.analysis')).toBe('Analysis');
  });

  it('t() returns the translation for an errors key', () => {
    expect(i18n.t('error.invalid_ip')).toBe('Invalid IP address');
  });

  it('t() returns the raw key when the key does not exist', () => {
    const key = 'nonexistent.__test__.key';
    expect(i18n.t(key)).toBe(key);
  });

  it('t() returns a non-empty string for every key in en-US ui', () => {
    const uiKeys = [
      'nav.analysis', 'nav.history', 'nav.settings', 'nav.about',
      'analysis.title', 'analysis.subtitle',
      'analysis.btn.analyze', 'analysis.btn.analyzing', 'analysis.btn.clear',
      'analysis.hint', 'analysis.empty',
    ];
    for (const key of uiKeys) {
      const val = i18n.t(key);
      expect(val, `key "${key}" should be non-empty`).toBeTruthy();
      expect(val, `key "${key}" should not echo back`).not.toBe(key);
    }
  });
});

// ── affichage dynamique — locale switching updates t() results ─────────────────

describe('i18n — affichage dynamique', () => {
  it('t() returns French translation after setLocale("fr-FR")', () => {
    i18n.setLocale('fr-FR');
    expect(i18n.t('nav.analysis')).toBe('Analyse');
  });

  it('t() returns English translation after switching back to "en-US"', () => {
    i18n.setLocale('fr-FR');
    i18n.setLocale('en-US');
    expect(i18n.t('nav.analysis')).toBe('Analysis');
  });

  it('locale property reflects the active locale', () => {
    expect(i18n.locale).toBe('en-US');
    i18n.setLocale('fr-FR');
    expect(i18n.locale).toBe('fr-FR');
  });

  it('tn() returns singular form when n === 1', () => {
    expect(i18n.tn('parse.count.ip.one', 'parse.count.ip.many', 1)).toBe('1 IP');
  });

  it('tn() returns plural form when n > 1', () => {
    expect(i18n.tn('parse.count.ip.one', 'parse.count.ip.many', 3)).toBe('3 IPs');
  });

  it('tn() returns French plural form in fr-FR', () => {
    i18n.setLocale('fr-FR');
    expect(i18n.tn('parse.count.ip.one', 'parse.count.ip.many', 5)).toBe('5 IPs');
  });
});

// ── init() reads locale from get_config ───────────────────────────────────────

describe('i18n — init() reads locale from get_config', () => {
  it('init() invokes get_config', async () => {
    mockInvoke.mockResolvedValue({ locale: 'en-US' });
    await i18n.init();
    expect(mockInvoke).toHaveBeenCalledWith('get_config');
  });

  it('init() sets locale from config', async () => {
    mockInvoke.mockResolvedValue({ locale: 'fr-FR' });
    await i18n.init();
    expect(i18n.locale).toBe('fr-FR');
    expect(i18n.t('nav.analysis')).toBe('Analyse');
  });

  it('init() defaults to en-US when get_config fails', async () => {
    mockInvoke.mockRejectedValue(new Error('Tauri unavailable'));
    await i18n.init();
    expect(i18n.locale).toBe('en-US');
    expect(i18n.t('nav.analysis')).toBe('Analysis');
  });

  it('init() defaults to en-US when locale field is missing', async () => {
    mockInvoke.mockResolvedValue({});
    await i18n.init();
    expect(i18n.locale).toBe('en-US');
  });
});
