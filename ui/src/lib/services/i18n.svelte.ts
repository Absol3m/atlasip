import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import enUsUi      from '../../../../i18n/en-US/ui.json';
import enUsErrors  from '../../../../i18n/en-US/errors.json';
import frFrUi      from '../../../../i18n/fr-FR/ui.json';
import frFrErrors  from '../../../../i18n/fr-FR/errors.json';

type TranslationMap = Record<string, string>;

const TRANSLATIONS: Record<string, TranslationMap> = {
  'en-US': { ...(enUsUi as TranslationMap), ...(enUsErrors as TranslationMap) },
  'fr-FR': { ...(frFrUi as TranslationMap), ...(frFrErrors as TranslationMap) },
};

const FALLBACK = TRANSLATIONS['en-US'];

interface AppConfig {
  locale?: string;
}

class I18n {
  locale  = $state('en-US');
  version = $state('');
  private map = $state<TranslationMap>(FALLBACK);

  async init(): Promise<void> {
    const [cfg, ver] = await Promise.allSettled([
      invoke<AppConfig>('get_config'),
      getVersion(),
    ]);
    if (cfg.status === 'fulfilled') {
      this.setLocale(cfg.value.locale ?? 'en-US');
    }
    if (ver.status === 'fulfilled') {
      this.version = ver.value;
    }
  }

  setLocale(loc: string): void {
    this.locale = loc;
    this.map = TRANSLATIONS[loc] ?? FALLBACK;
  }

  t(key: string): string {
    return this.map[key] ?? FALLBACK[key] ?? key;
  }

  tn(singular: string, plural: string, n: number): string {
    const key = n === 1 ? singular : plural;
    return this.t(key).replace('{n}', String(n));
  }
}

export const i18n = new I18n();
