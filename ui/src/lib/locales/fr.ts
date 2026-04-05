import type { LocaleKey } from './en';

const fr: Record<LocaleKey, string> = {
  'app.title': 'AtlasIP',
  'app.version': '0.1.0',

  'nav.analysis': 'Analyse',
  'nav.history': 'Historique',
  'nav.settings': 'Paramètres',
  'nav.about': 'À propos',

  'analysis.title': 'Analyse IP',
  'analysis.subtitle': 'Analysez des IPs, hostnames, ou collez des en-têtes email',
  'analysis.placeholder.default': 'Saisissez une ou plusieurs adresses IP (une par ligne)',
  'analysis.placeholder.header': 'IPs extraites de l\'en-tête email',
  'analysis.placeholder.text': 'IPs extraites du texte',
  'analysis.placeholder.hostname': 'Hostname résolu en IP(s)',
  'analysis.btn.analyze': 'Analyser',
  'analysis.btn.analyzing': 'Analyse en cours…',
  'analysis.btn.clear': 'Effacer',
  'analysis.hint': 'Ctrl+Entrée pour analyser',
  'analysis.empty': 'Aucun résultat. Saisissez des IPs ci-dessus pour commencer.',
  'analysis.results.title': 'Résultats',

  // ── Badge mode labels (P0-UX-003 / P1-UX-005) ────────────────────────────
  'parse.mode.ipv4':     'IPv4 DÉTECTÉ',
  'parse.mode.ipv6':     'IPv6 DÉTECTÉ',
  'parse.mode.hostname': 'HOSTNAME DÉTECTÉ',
  'parse.mode.mixed':    'MODE MIXTE',
  'parse.mode.invalid':  'INVALIDE',
  // Legacy keys kept for any residual use
  'parse.mode.ip':       'IPs détectées',
  'parse.mode.header':   'En-tête email détecté',
  'parse.mode.text':     'IPs extraites du texte',

  // ── Count parts (P1-UX-004) ───────────────────────────────────────────────
  'parse.count.ip.one':       '{n} IP',
  'parse.count.ip.many':      '{n} IPs',
  'parse.count.hostname.one': '{n} hostname',
  'parse.count.hostname.many':'{n} hostnames',
  'parse.count.url.one':      '{n} URL',
  'parse.count.url.many':     '{n} URLs',

  // Legacy count keys
  'parse.valid.one':     '{n} IP valide',
  'parse.valid.many':    '{n} IPs valides',
  'parse.invalid.one':   '{n} invalide',
  'parse.invalid.many':  '{n} invalides',
  'parse.filtered.one':  '{n} filtrée (privée)',
  'parse.filtered.many': '{n} filtrées (privées)',
  'parse.hostname.one':  '{n} hostname',
  'parse.hostname.many': '{n} hostnames',

  'error.invalid_ip':           'Adresse IP invalide',
  'error.private_ip':           'IP privée ou réservée — ignorée',
  'error.no_ip_found':          'Aucune adresse IP trouvée dans la saisie',
  'error.lookup_failed':        'Lookup échoué (réseau ou limite de débit)',
  'error.backend_unreachable':  'Backend inaccessible — AtlasIP est-il lancé ?',
  'error.export_failed':        'Export échoué',
  'error.no_valid_targets':     'Aucune IP ou hostname valide à analyser',
  'error.fix_invalid_entries':  'Corrigez les entrées invalides avant d\'analyser',
};

export default fr;
