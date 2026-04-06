const en = {
  'app.title': 'AtlasIP',
  'app.version': '0.3.0-alpha',

  'nav.analysis': 'Analysis',
  'nav.history': 'History',
  'nav.settings': 'Settings',
  'nav.about': 'About',

  'analysis.title': 'IP Analysis',
  'analysis.subtitle': 'Analyze IP addresses, hostnames, or paste email headers',
  'analysis.placeholder.default': 'Enter one or more IP addresses (one per line)',
  'analysis.placeholder.header': 'IPs extracted from email header',
  'analysis.placeholder.text': 'IPs extracted from text',
  'analysis.placeholder.hostname': 'Hostname resolved to IP(s)',
  'analysis.btn.analyze': 'Analyze',
  'analysis.btn.analyzing': 'Analyzing…',
  'analysis.btn.clear': 'Clear',
  'analysis.hint': 'Ctrl+Enter to analyze',
  'analysis.empty': 'No results yet. Enter IPs above to begin.',
  'analysis.results.title': 'Results',

  // ── Badge mode labels (P0-UX-003 / P1-UX-005) ────────────────────────────
  'parse.mode.ipv4':     'IPv4 DETECTED',
  'parse.mode.ipv6':     'IPv6 DETECTED',
  'parse.mode.hostname': 'HOSTNAME DETECTED',
  'parse.mode.mixed':    'MIXED MODE',
  'parse.mode.invalid':  'INVALID',
  // Legacy keys kept for any residual use
  'parse.mode.ip':       'IPs detected',
  'parse.mode.header':   'Email header detected',
  'parse.mode.text':     'IPs extracted from text',

  // ── Count parts (P1-UX-004) ───────────────────────────────────────────────
  'parse.count.ip.one':       '{n} IP',
  'parse.count.ip.many':      '{n} IPs',
  'parse.count.hostname.one': '{n} hostname',
  'parse.count.hostname.many':'{n} hostnames',
  'parse.count.url.one':      '{n} URL',
  'parse.count.url.many':     '{n} URLs',

  // Legacy count keys
  'parse.valid.one':     '{n} valid IP',
  'parse.valid.many':    '{n} valid IPs',
  'parse.invalid.one':   '{n} invalid',
  'parse.invalid.many':  '{n} invalid',
  'parse.filtered.one':  '{n} filtered (private)',
  'parse.filtered.many': '{n} filtered (private)',
  'parse.hostname.one':  '{n} hostname',
  'parse.hostname.many': '{n} hostnames',

  'error.invalid_ip':           'Invalid IP address',
  'error.private_ip':           'Private or reserved IP — ignored',
  'error.no_ip_found':          'No IP address found in input',
  'error.lookup_failed':        'Lookup failed (network or rate limit)',
  'error.backend_unreachable':  'Backend unreachable — is AtlasIP running?',
  'error.export_failed':        'Export failed',
  'error.no_valid_targets':     'No valid IP or hostname to analyze',
  'error.fix_invalid_entries':  'Fix invalid entries before analyzing',
} as const;

export type LocaleKey = keyof typeof en;
export default en;
