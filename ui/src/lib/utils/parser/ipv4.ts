// ── IPv4 — strict validation + private/reserved range detection ───────────────
// §3.1: Format strict, 4 octets, each octet ∈ [0, 255].
// §12: Pure, no side effects, no trimming, no fallback.

/** Matches exactly a decimal-dotted IPv4 token (no leading spaces, no trailing). */
const IPV4_EXACT_RE =
  /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/;

/**
 * Return true iff `s` is a syntactically valid IPv4 address.
 * Pure function — no trimming.
 */
export function isValidIPv4(s: string): boolean {
  const m = IPV4_EXACT_RE.exec(s);
  if (!m) return false;
  for (let i = 1; i <= 4; i++) {
    const n = Number(m[i]);
    // Leading zeros are not standard (e.g. "01" is ambiguous) — reject them.
    if (m[i].length > 1 && m[i][0] === '0') return false;
    if (n < 0 || n > 255) return false;
  }
  return true;
}

// ── Private / reserved IPv4 ranges (RFC 1918, RFC 5735, etc.) ────────────────

interface CidrRange { base: number; mask: number }

function ipToInt(ip: string): number {
  const p = ip.split('.');
  return (((+p[0] << 24) | (+p[1] << 16) | (+p[2] << 8) | +p[3]) >>> 0);
}

function cidr(notation: string): CidrRange {
  const [addr, bits] = notation.split('/');
  const prefixLen = Number(bits);
  const mask = prefixLen === 0 ? 0 : (~0 << (32 - prefixLen)) >>> 0;
  return { base: ipToInt(addr) & mask, mask };
}

const PRIVATE_RANGES: CidrRange[] = [
  cidr('0.0.0.0/8'),          // "This" network
  cidr('10.0.0.0/8'),         // Private
  cidr('100.64.0.0/10'),      // Shared Address Space (RFC 6598)
  cidr('127.0.0.0/8'),        // Loopback
  cidr('169.254.0.0/16'),     // Link-local
  cidr('172.16.0.0/12'),      // Private
  cidr('192.0.0.0/24'),       // IETF Protocol Assignments
  cidr('192.0.2.0/24'),       // TEST-NET-1 (RFC 5737)
  cidr('192.88.99.0/24'),     // 6to4 Relay (deprecated)
  cidr('192.168.0.0/16'),     // Private
  cidr('198.18.0.0/15'),      // Benchmarking (RFC 2544)
  cidr('198.51.100.0/24'),    // TEST-NET-2 (RFC 5737)
  cidr('203.0.113.0/24'),     // TEST-NET-3 (RFC 5737)
  cidr('224.0.0.0/4'),        // Multicast
  cidr('240.0.0.0/4'),        // Reserved (future use)
  cidr('255.255.255.255/32'), // Broadcast
];

/**
 * Return true iff `ip` is a private, loopback, reserved, or multicast IPv4.
 * Precondition: `isValidIPv4(ip)` is true.
 */
export function isPrivateIPv4(ip: string): boolean {
  const n = ipToInt(ip);
  return PRIVATE_RANGES.some(r => (n & r.mask) === r.base);
}

/**
 * Extract all valid IPv4 addresses embedded in `text`.
 * Uses a word-boundary-aware regex so "999.999.999.999" is extracted but fails
 * `isValidIPv4` later.
 */
export function extractIPv4(text: string): string[] {
  // Capture anything that looks like dotted-quad; validate afterward.
  const candidates = text.match(/\b\d{1,3}(?:\.\d{1,3}){3}\b/g) ?? [];
  return candidates.filter(isValidIPv4);
}
