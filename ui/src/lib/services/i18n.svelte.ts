import en, { type LocaleKey } from '$lib/locales/en';
import fr from '$lib/locales/fr';

type Locale = 'en' | 'fr';

const LS_KEY = 'atlasip.locale';
const LOCALES: Record<Locale, Record<string, string>> = { en, fr };

class I18nStore {
  locale = $state<Locale>('en');

  /** Translate a key, substituting `{n}`, `{key}` placeholders. */
  t(key: LocaleKey, params?: Record<string, string | number>): string {
    const dict = LOCALES[this.locale];
    let msg: string = dict[key] ?? (LOCALES.en[key] as string) ?? key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        msg = msg.replaceAll(`{${k}}`, String(v));
      }
    }
    return msg;
  }

  /** Pluralised helper — uses `{n}` key. */
  tn(keyOne: LocaleKey, keyMany: LocaleKey, n: number): string {
    return this.t(n === 1 ? keyOne : keyMany, { n });
  }

  setLocale(l: Locale) {
    this.locale = l;
    try { localStorage.setItem(LS_KEY, l); } catch { /* private browsing */ }
  }

  init() {
    try {
      const saved = localStorage.getItem(LS_KEY) as Locale | null;
      if (saved && saved in LOCALES) this.locale = saved;
    } catch { /* SSR / private browsing */ }
  }
}

export const i18n = new I18nStore();
