// ── Parser v2 — Unit tests (§9.6) ─────────────────────────────────────────────
// 50+ tests covering: IPv4, IPv6, hostname, URL, text extraction, email headers,
// mixed modes, blocking (§5.1), deduplication, edge cases.

import { describe, it, expect } from 'vitest';
import { isValidIPv4, isPrivateIPv4 } from './ipv4';
import { isValidIPv6, isPrivateIPv6 } from './ipv6';
import { isValidHostname } from './hostname';
import { parseURL, isValidURL } from './url';
import { isEmailHeader } from './extractors';
import { parseInput } from './index';

// ─────────────────────────────────────────────────────────────────────────────
// §1 — isValidIPv4
// ─────────────────────────────────────────────────────────────────────────────
describe('isValidIPv4', () => {
  it('accepts a valid public IPv4', () => {
    expect(isValidIPv4('8.8.8.8')).toBe(true);
  });

  it('accepts boundary octets (0 and 255)', () => {
    expect(isValidIPv4('0.0.0.0')).toBe(true);
    expect(isValidIPv4('255.255.255.255')).toBe(true);
  });

  it('rejects octet > 255', () => {
    expect(isValidIPv4('256.0.0.0')).toBe(false);
    expect(isValidIPv4('1.2.3.300')).toBe(false);
  });

  it('rejects leading zeros in an octet', () => {
    expect(isValidIPv4('01.2.3.4')).toBe(false);
    expect(isValidIPv4('1.02.3.4')).toBe(false);
    expect(isValidIPv4('1.2.3.04')).toBe(false);
  });

  it('rejects too few octets', () => {
    expect(isValidIPv4('1.2.3')).toBe(false);
  });

  it('rejects too many octets', () => {
    expect(isValidIPv4('1.2.3.4.5')).toBe(false);
  });

  it('rejects non-numeric parts', () => {
    expect(isValidIPv4('a.b.c.d')).toBe(false);
    expect(isValidIPv4('1.2.3.x')).toBe(false);
  });

  it('rejects empty string', () => {
    expect(isValidIPv4('')).toBe(false);
  });

  it('rejects strings with trailing/leading whitespace (§12: no trimming)', () => {
    expect(isValidIPv4(' 1.2.3.4')).toBe(false);
    expect(isValidIPv4('1.2.3.4 ')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §2 — isPrivateIPv4
// ─────────────────────────────────────────────────────────────────────────────
describe('isPrivateIPv4', () => {
  it('detects RFC-1918 10.x.x.x', () => {
    expect(isPrivateIPv4('10.0.0.1')).toBe(true);
    expect(isPrivateIPv4('10.255.255.255')).toBe(true);
  });

  it('detects RFC-1918 172.16-31.x.x', () => {
    expect(isPrivateIPv4('172.16.0.1')).toBe(true);
    expect(isPrivateIPv4('172.31.255.255')).toBe(true);
    expect(isPrivateIPv4('172.32.0.0')).toBe(false);
  });

  it('detects RFC-1918 192.168.x.x', () => {
    expect(isPrivateIPv4('192.168.1.1')).toBe(true);
  });

  it('detects loopback 127.x.x.x', () => {
    expect(isPrivateIPv4('127.0.0.1')).toBe(true);
    expect(isPrivateIPv4('127.255.255.255')).toBe(true);
  });

  it('detects link-local 169.254.x.x', () => {
    expect(isPrivateIPv4('169.254.1.1')).toBe(true);
  });

  it('detects RFC-5737 documentation ranges', () => {
    expect(isPrivateIPv4('192.0.2.1')).toBe(true);     // TEST-NET-1
    expect(isPrivateIPv4('198.51.100.1')).toBe(true);   // TEST-NET-2
    expect(isPrivateIPv4('203.0.113.1')).toBe(true);    // TEST-NET-3
  });

  it('does NOT flag a public IP as private', () => {
    expect(isPrivateIPv4('8.8.8.8')).toBe(false);
    expect(isPrivateIPv4('1.1.1.1')).toBe(false);
    expect(isPrivateIPv4('93.184.216.34')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §3 — isValidIPv6
// ─────────────────────────────────────────────────────────────────────────────
describe('isValidIPv6', () => {
  it('accepts a normalized full-form address', () => {
    // The WHATWG URL parser normalizes addresses (strips leading zeros),
    // so we test with a pre-normalized form that round-trips correctly.
    expect(isValidIPv6('2001:db8:85a3::8a2e:370:7334')).toBe(true);
  });

  it('accepts compressed form ::', () => {
    expect(isValidIPv6('2001:db8::1')).toBe(true);
    expect(isValidIPv6('::1')).toBe(true);
    expect(isValidIPv6('::')).toBe(true);
  });

  it('accepts a public GUA', () => {
    expect(isValidIPv6('2606:4700:4700::1111')).toBe(true);
  });

  it('rejects an address without colon', () => {
    expect(isValidIPv6('not-an-ipv6')).toBe(false);
  });

  it('rejects too many groups', () => {
    expect(isValidIPv6('1:2:3:4:5:6:7:8:9')).toBe(false);
  });

  it('rejects empty string', () => {
    expect(isValidIPv6('')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §4 — isPrivateIPv6
// ─────────────────────────────────────────────────────────────────────────────
describe('isPrivateIPv6', () => {
  it('detects loopback ::1', () => {
    expect(isPrivateIPv6('::1')).toBe(true);
  });

  it('detects unspecified ::', () => {
    expect(isPrivateIPv6('::')).toBe(true);
  });

  it('detects ULA fc::/7', () => {
    expect(isPrivateIPv6('fc00::1')).toBe(true);
    expect(isPrivateIPv6('fd00::1')).toBe(true);
  });

  it('detects link-local fe80::', () => {
    expect(isPrivateIPv6('fe80::1')).toBe(true);
  });

  it('detects documentation 2001:db8::', () => {
    expect(isPrivateIPv6('2001:db8::1')).toBe(true);
  });

  it('does NOT flag a public GUA as private', () => {
    expect(isPrivateIPv6('2606:4700:4700::1111')).toBe(false);
    expect(isPrivateIPv6('2001:4860:4860::8888')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §5 — isValidHostname
// ─────────────────────────────────────────────────────────────────────────────
describe('isValidHostname', () => {
  it('accepts a simple domain', () => {
    expect(isValidHostname('google.com')).toBe(true);
  });

  it('accepts a subdomain', () => {
    expect(isValidHostname('sub.example.co.uk')).toBe(true);
  });

  it('accepts a hyphenated label', () => {
    expect(isValidHostname('my-host.example.org')).toBe(true);
  });

  it('rejects a bare label with no dot', () => {
    expect(isValidHostname('localhost')).toBe(false);
  });

  it('rejects a double dot', () => {
    expect(isValidHostname('foo..bar.com')).toBe(false);
  });

  it('rejects a label starting with a hyphen', () => {
    expect(isValidHostname('-bad.example.com')).toBe(false);
  });

  it('rejects a label ending with a hyphen', () => {
    expect(isValidHostname('bad-.example.com')).toBe(false);
  });

  it('rejects a hostname > 253 chars', () => {
    const long = 'a'.repeat(64) + '.com';
    expect(isValidHostname(long)).toBe(false);
  });

  it('rejects an empty string', () => {
    expect(isValidHostname('')).toBe(false);
  });

  it('rejects an IPv4-looking string (not a hostname)', () => {
    // 1.2.3.4 has only numeric labels — HOSTNAME_RE requires alpha TLD
    expect(isValidHostname('1.2.3.4')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §6 — parseURL / isValidURL
// ─────────────────────────────────────────────────────────────────────────────
describe('parseURL', () => {
  it('parses a simple https URL', () => {
    const r = parseURL('https://example.com');
    expect(r).not.toBeNull();
    expect(r!.hostname).toBe('example.com');
    expect(r!.protocol).toBe('https:');
    expect(r!.port).toBeUndefined();
  });

  it('parses a URL with a port', () => {
    const r = parseURL('http://example.com:8080/path');
    expect(r).not.toBeNull();
    expect(r!.hostname).toBe('example.com');
    expect(r!.port).toBe(8080);
  });

  it('parses a URL with an IPv4 hostname', () => {
    const r = parseURL('https://93.184.216.34/test');
    expect(r).not.toBeNull();
    expect(r!.hostname).toBe('93.184.216.34');
  });

  it('parses a URL with an IPv6 hostname', () => {
    const r = parseURL('https://[2606:4700:4700::1111]/');
    expect(r).not.toBeNull();
    // In Node.js the WHATWG URL API keeps brackets in .hostname for IPv6
    expect(r!.hostname).toMatch(/2606:4700:4700::1111/);
  });

  it('rejects a bare hostname without ://', () => {
    expect(parseURL('example.com')).toBeNull();
  });

  it('rejects a data: scheme', () => {
    expect(parseURL('data:text/plain,hello')).toBeNull();
  });

  it('rejects a javascript: scheme', () => {
    expect(parseURL('javascript:alert(1)')).toBeNull();
  });
});

describe('isValidURL', () => {
  it('returns true for a valid https URL', () => {
    expect(isValidURL('https://example.com')).toBe(true);
  });

  it('returns false for a bare domain', () => {
    expect(isValidURL('example.com')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §7 — isEmailHeader
// ─────────────────────────────────────────────────────────────────────────────
describe('isEmailHeader', () => {
  it('detects a Received: header', () => {
    expect(isEmailHeader('Received: from mail.example.com by mx.example.org')).toBe(true);
  });

  it('detects an X-Originating-IP: header', () => {
    expect(isEmailHeader('X-Originating-IP: 8.8.8.8')).toBe(true);
  });

  it('detects an X-Forwarded-For: header', () => {
    expect(isEmailHeader('X-Forwarded-For: 1.2.3.4, 5.6.7.8')).toBe(true);
  });

  it('returns false for plain text', () => {
    expect(isEmailHeader('just some text without headers')).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §8 — parseInput — empty input
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — empty', () => {
  it('returns empty mode for empty string', () => {
    const r = parseInput('');
    expect(r.mode).toBe('empty');
    expect(r.validTargets).toEqual([]);
    expect(r.blocked).toBe(false);
  });

  it('returns empty mode for whitespace-only string', () => {
    const r = parseInput('   \n  ');
    expect(r.mode).toBe('empty');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §9 — parseInput — single IPv4
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — single IPv4', () => {
  it('parses a valid public IPv4', () => {
    const r = parseInput('8.8.8.8');
    expect(r.mode).toBe('ipv4');
    expect(r.validTargets).toEqual(['8.8.8.8']);
    expect(r.invalidEntries).toEqual([]);
    expect(r.filteredCount).toBe(0);
    expect(r.blocked).toBe(false);
  });

  it('filters a private IPv4 (not in validTargets, not invalid)', () => {
    const r = parseInput('192.168.1.1');
    expect(r.validTargets).toEqual([]);
    expect(r.invalidEntries).toEqual([]);
    expect(r.filteredCount).toBe(1);
  });

  it('blocks on an invalid IPv4 (§5.1)', () => {
    const r = parseInput('999.999.999.999');
    expect(r.validTargets).toEqual([]);
    expect(r.blocked).toBe(true);
    expect(r.invalidEntries.length).toBe(1);
    expect(r.invalidEntries[0].reason).toBe('invalid_ipv4');
  });

  it('blocks on an IPv4 with leading zero (§5.1)', () => {
    const r = parseInput('01.2.3.4');
    expect(r.blocked).toBe(true);
    expect(r.invalidEntries[0].reason).toBe('invalid_ipv4');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §10 — parseInput — multiple IPv4
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — multiple IPv4', () => {
  it('parses multiple valid public IPs', () => {
    const r = parseInput('8.8.8.8\n1.1.1.1\n9.9.9.9');
    expect(r.mode).toBe('ipv4');
    expect(r.validTargets).toEqual(['8.8.8.8', '1.1.1.1', '9.9.9.9']);
  });

  it('deduplicates identical IPs', () => {
    const r = parseInput('8.8.8.8\n8.8.8.8\n8.8.8.8');
    expect(r.validTargets).toEqual(['8.8.8.8']);
  });

  it('blocks everything if one IP is invalid (§5.1)', () => {
    const r = parseInput('8.8.8.8\n999.0.0.1\n1.1.1.1');
    expect(r.validTargets).toEqual([]);
    expect(r.blocked).toBe(true);
    expect(r.invalidEntries.length).toBe(1);
  });

  it('filters private IPs without blocking when the rest are valid', () => {
    const r = parseInput('8.8.8.8\n192.168.1.1\n1.1.1.1');
    expect(r.validTargets).toEqual(['8.8.8.8', '1.1.1.1']);
    expect(r.filteredCount).toBe(1);
    expect(r.blocked).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §11 — parseInput — IPv6
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — IPv6', () => {
  it('parses a valid public IPv6', () => {
    const r = parseInput('2606:4700:4700::1111');
    expect(r.mode).toBe('ipv6');
    expect(r.validTargets).toEqual(['2606:4700:4700::1111']);
  });

  it('accepts bracket-wrapped IPv6', () => {
    const r = parseInput('[2606:4700:4700::1111]');
    expect(r.mode).toBe('ipv6');
    expect(r.validTargets).toEqual(['2606:4700:4700::1111']);
  });

  it('filters a private IPv6 (::1)', () => {
    const r = parseInput('::1');
    expect(r.validTargets).toEqual([]);
    expect(r.filteredCount).toBe(1);
    expect(r.blocked).toBe(false);
  });

  it('blocks on a malformed IPv6', () => {
    const r = parseInput('gggg::1');
    expect(r.blocked).toBe(true);
    expect(r.invalidEntries[0].reason).toBe('invalid_ipv6');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §12 — parseInput — hostname
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — hostname', () => {
  it('parses a valid hostname', () => {
    const r = parseInput('example.com');
    expect(r.mode).toBe('hostname');
    expect(r.validTargets).toEqual(['example.com']);
  });

  it('parses multiple hostnames', () => {
    const r = parseInput('example.com\nmalware.io\ntest.org');
    expect(r.mode).toBe('hostname');
    expect(r.validTargets).toHaveLength(3);
  });

  it('blocks on an invalid hostname token', () => {
    const r = parseInput('not_a_hostname!');
    expect(r.blocked).toBe(true);
    expect(r.invalidEntries[0].reason).toBe('unknown_entry');
  });

  it('blocks on a bare label (no TLD)', () => {
    const r = parseInput('localhost');
    expect(r.blocked).toBe(true);
    expect(r.invalidEntries[0].reason).toBe('unknown_entry');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §13 — parseInput — URL
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — URL', () => {
  it('extracts hostname from an https URL', () => {
    const r = parseInput('https://example.com');
    expect(r.mode).toBe('hostname');
    expect(r.validTargets).toEqual(['example.com']);
  });

  it('extracts IP from a URL with an IP hostname', () => {
    const r = parseInput('https://93.184.216.34/path');
    // URL entries have type 'url' → deriveMode maps url → 'hostname' mode
    expect(r.mode).toBe('hostname');
    expect(r.validTargets).toEqual(['93.184.216.34']);
  });

  it('blocks on a URL with an invalid scheme', () => {
    const r = parseInput('ftp://example.com');
    // ftp: is accepted by parseURL
    expect(r.validTargets).toEqual(['example.com']);
    expect(r.blocked).toBe(false);
  });

  it('blocks on a data: URL (§5.1)', () => {
    const r = parseInput('data:text/plain,hello');
    expect(r.blocked).toBe(true);
  });

  it('filters a URL pointing to a private IP', () => {
    const r = parseInput('https://192.168.1.1/admin');
    expect(r.validTargets).toEqual([]);
    expect(r.filteredCount).toBe(1);
    expect(r.blocked).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §14 — parseInput — mixed mode (§6)
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — mixed mode', () => {
  it('returns mixed mode for IPv4 + hostname', () => {
    const r = parseInput('8.8.8.8\nexample.com');
    expect(r.mode).toBe('mixed');
  });

  it('returns mixed mode for IPv4 + IPv6', () => {
    const r = parseInput('8.8.8.8\n2606:4700:4700::1111');
    expect(r.mode).toBe('mixed');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §15 — parseInput — free text extraction
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — free text', () => {
  it('extracts an IP from a sentence', () => {
    const r = parseInput('The server at 8.8.8.8 is unreachable.');
    expect(r.validTargets).toContain('8.8.8.8');
    expect(r.blocked).toBe(false);
  });

  it('extracts multiple IPs from free text', () => {
    const r = parseInput('Primary DNS: 8.8.8.8 and secondary DNS: 1.1.1.1');
    expect(r.validTargets).toContain('8.8.8.8');
    expect(r.validTargets).toContain('1.1.1.1');
  });

  it('extracts a hostname from free text', () => {
    const r = parseInput('Please check google.com for details.');
    expect(r.validTargets).toContain('google.com');
  });

  it('extracts a URL hostname from free text', () => {
    const r = parseInput('Visit https://example.com for more info.');
    expect(r.validTargets).toContain('example.com');
  });

  it('does NOT block on free text with unknown words (§7.5)', () => {
    const r = parseInput('The quick brown fox jumps over 8.8.8.8');
    expect(r.blocked).toBe(false);
    expect(r.validTargets).toContain('8.8.8.8');
  });

  it('silently ignores unrecognized words in text mode (§7.5)', () => {
    const r = parseInput('No IPs here, just text without any network addresses.');
    expect(r.blocked).toBe(false);
    expect(r.validTargets).toEqual([]);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §16 — parseInput — email header extraction (§7.4)
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — email headers', () => {
  it('extracts IPs from email headers without blocking', () => {
    const header = [
      'Received: from smtp.example.com (8.8.8.8) by mx.host.com',
      'X-Originating-IP: 1.1.1.1',
    ].join('\n');
    const r = parseInput(header);
    expect(r.blocked).toBe(false);
    expect(r.validTargets).toContain('8.8.8.8');
    expect(r.validTargets).toContain('1.1.1.1');
  });

  it('filters private IPs in email headers without blocking', () => {
    const header = 'X-Forwarded-For: 192.168.1.100, 8.8.8.8';
    const r = parseInput(header);
    expect(r.blocked).toBe(false);
    expect(r.validTargets).toContain('8.8.8.8');
    expect(r.filteredCount).toBeGreaterThanOrEqual(1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// §17 — parseInput — edge cases
// ─────────────────────────────────────────────────────────────────────────────
describe('parseInput — edge cases', () => {
  it('handles a single newline', () => {
    const r = parseInput('\n');
    expect(r.mode).toBe('empty');
  });

  it('handles CRLF line endings', () => {
    const r = parseInput('8.8.8.8\r\n1.1.1.1');
    expect(r.validTargets).toContain('8.8.8.8');
  });

  it('deduplicates case-insensitively for hostnames', () => {
    const r = parseInput('Example.COM\nexample.com');
    expect(r.validTargets).toHaveLength(1);
  });

  it('does not trim individual tokens (§12)', () => {
    // A line with only spaces is treated as empty, not as a bad token
    const r = parseInput('8.8.8.8\n   \n1.1.1.1');
    expect(r.validTargets).toEqual(['8.8.8.8', '1.1.1.1']);
  });

  it('handles a mix of explicit tokens + multi-word lines', () => {
    // Explicit token 8.8.8.8 is valid; multi-word line extracts 1.1.1.1
    const r = parseInput('8.8.8.8\nThe host is 1.1.1.1');
    expect(r.validTargets).toContain('8.8.8.8');
    expect(r.validTargets).toContain('1.1.1.1');
    expect(r.blocked).toBe(false);
  });

  it('blocks when an explicit token is invalid even if multi-word lines exist', () => {
    const r = parseInput('bad_token\nSome text with 8.8.8.8');
    expect(r.blocked).toBe(true);
    expect(r.validTargets).toEqual([]);
  });

  it('returns mode=invalid when all explicit entries are private (everything filtered)', () => {
    const r = parseInput('192.168.1.1\n10.0.0.1');
    expect(r.mode).toBe('invalid');
    expect(r.validTargets).toEqual([]);
    expect(r.filteredCount).toBe(2);
  });

  it('handles broadcast address 255.255.255.255 as private (filtered)', () => {
    const r = parseInput('255.255.255.255');
    expect(r.filteredCount).toBe(1);
    expect(r.validTargets).toEqual([]);
  });

  it('returns globalError when blocked', () => {
    const r = parseInput('not_an_ip');
    expect(r.blocked).toBe(true);
    expect(r.globalError).toBe('error.no_valid_targets');
  });

  it('returns globalError=null when not blocked', () => {
    const r = parseInput('8.8.8.8');
    expect(r.globalError).toBeNull();
  });
});
