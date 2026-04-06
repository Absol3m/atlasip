// @vitest-environment happy-dom

import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte';

// ── Mock Tauri invoke (hoisted above all imports) ─────────────────────────────
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
const mockInvoke = vi.mocked(invoke);

import Settings from './+page.svelte';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG = {
  network: {
    dns_resolver:    'system',
    ip_mode:         'dual',
    pooling_enabled: true,
    max_connections: 10,
    keep_alive:      true,
    retry_strategy:  'none',
    retry_delay_ms:  1_000,
  },
  proxy: {
    enabled:    false,
    proxy_type: 'HTTP',
    url:        '',
    no_proxy:   '',
  },
  timeouts: {
    global_ms:  5_000,
    request_ms: 10_000,
    dns_ms:     2_000,
    geoip_ms:   3_000,
  },
  retry: {
    enabled:  false,
    count:    3,
    delay_ms: 1_000,
  },
  locale: 'en',
  theme:  'system',
};

function setupDefaultMock() {
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'get_config') return Promise.resolve(DEFAULT_CONFIG);
    return Promise.resolve(null);
  });
}

/**
 * Render the page and wait for the entire onMount chain to finish:
 *   invoke('get_config') → applyConfig() → tick() → saveEnabled = true
 *
 * Simply detecting that get_config was called is not enough — at that point
 * the mock promise hasn't resolved yet, so saveEnabled is still false and
 * the $effect guard would silently skip saves on subsequent interactions.
 */
async function mountAndLoad() {
  const result = render(Settings);
  await waitFor(() => {
    expect(mockInvoke).toHaveBeenCalledWith('get_config');
  });
  // One macrotask tick lets the pending microtask chain complete:
  // mockInvoke resolve → applyConfig → Svelte tick() → saveEnabled = true
  await new Promise(r => setTimeout(r, 20));
  return result;
}

beforeEach(() => {
  setupDefaultMock();
});

afterEach(() => {
  cleanup();          // Remove rendered components from DOM between tests
  vi.clearAllMocks();
});

// ── Suite 1: config load on mount ─────────────────────────────────────────────

describe('Settings — config load on mount', () => {
  it('calls get_config when the page mounts', async () => {
    render(Settings);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_config');
    });
  });

  it('applies backend config to the DNS resolver select', async () => {
    // Return a value that differs from the component's hard-coded initial state
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve({
        ...DEFAULT_CONFIG,
        network: { ...DEFAULT_CONFIG.network, dns_resolver: 'cloudflare' },
      });
      return Promise.resolve(null);
    });

    render(Settings);

    const select = screen.getByLabelText('DNS resolver') as HTMLSelectElement;
    await waitFor(() => {
      expect(select.value).toBe('cloudflare');
    });
  });
});

// ── Suite 2: field changes trigger set_config ─────────────────────────────────

describe('Settings — field changes call set_config after debounce', () => {
  it('calls set_config with updated proxy.enabled after 300 ms', async () => {
    await mountAndLoad();
    mockInvoke.mockClear();

    const proxyToggle = screen.getByRole('switch', { name: 'Enable proxy' });
    await fireEvent.click(proxyToggle);

    // Not called immediately (debounce pending)
    expect(mockInvoke).not.toHaveBeenCalledWith('set_config', expect.anything());

    await waitFor(
      () => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'set_config',
          expect.objectContaining({
            config: expect.objectContaining({
              proxy: expect.objectContaining({ enabled: true }),
            }),
          }),
        );
      },
      { timeout: 700 },
    );
  }, 3_000);

  it('calls set_config with updated network.keep_alive after 300 ms', async () => {
    await mountAndLoad();
    mockInvoke.mockClear();

    // DEFAULT_CONFIG has keep_alive: true — toggling flips it to false
    const keepAliveToggle = screen.getByRole('switch', { name: 'Enable keep-alive' });
    await fireEvent.click(keepAliveToggle);

    expect(mockInvoke).not.toHaveBeenCalledWith('set_config', expect.anything());

    await waitFor(
      () => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'set_config',
          expect.objectContaining({
            config: expect.objectContaining({
              network: expect.objectContaining({ keep_alive: false }),
            }),
          }),
        );
      },
      { timeout: 700 },
    );
  }, 3_000);
});

// ── Suite 3: validation blocks invalid saves ───────────────────────────────────

describe('Settings — validation', () => {
  it('shows error when max connections exceeds 100', async () => {
    await mountAndLoad();

    // max-connections is visible because pooling_enabled = true in DEFAULT_CONFIG
    const maxConn = await waitFor(() => screen.getByLabelText('Max connections'));
    await fireEvent.input(maxConn, { target: { value: '200', valueAsNumber: 200 } });

    await waitFor(() => {
      expect(screen.getByText('Must be between 1 and 100')).toBeTruthy();
    });
  });

  it('does not call set_config while max connections has a validation error', async () => {
    await mountAndLoad();
    mockInvoke.mockClear();

    const maxConn = screen.getByLabelText('Max connections') as HTMLInputElement;
    await fireEvent.input(maxConn, { target: { value: '999', valueAsNumber: 999 } });

    // Wait past the 300 ms debounce
    await new Promise(r => setTimeout(r, 400));

    expect(mockInvoke).not.toHaveBeenCalledWith('set_config', expect.anything());
  }, 3_000);

  it('shows error when retry count exceeds 10', async () => {
    await mountAndLoad();

    // Enable auto-retry to reveal the retry count field
    const retryToggle = screen.getByRole('switch', { name: 'Enable auto-retry' });
    await fireEvent.click(retryToggle);

    const retryCount = await waitFor(() => screen.getByLabelText('Retry count'));
    await fireEvent.input(retryCount, { target: { value: '15', valueAsNumber: 15 } });

    await waitFor(() => {
      expect(screen.getByText('Must be between 1 and 10')).toBeTruthy();
    });
  });
});
