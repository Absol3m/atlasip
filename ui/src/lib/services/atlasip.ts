export const BASE_URL = "http://127.0.0.1:8080";

async function request<T>(path: string): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json() as Promise<T>;
}

export interface HealthStatus {
  status: string;
  geoip_db: 'ok' | 'outdated' | 'missing';
}

export async function getHealth(): Promise<HealthStatus> {
  return request<HealthStatus>("/health");
}

export interface ReverseIpDomain {
  domain: string;
  sources: string[];
}

export interface ReverseIpSourceError {
  source: string;
  error: string;
}

export interface ReverseIpResult {
  ip: string;
  results: ReverseIpDomain[];
  count: number;
  source_errors: ReverseIpSourceError[];
}

/** Fields from Tauri AppConfig that map directly to backend ConfigUpdateRequest. */
export interface BackendConfigSync {
  locale?:               string;
  dns_timeout_ms?:       number;
  rdap_timeout_ms?:      number;
  whois_timeout_ms?:     number;
  maxmind_account_id?:   string | null;
  maxmind_license_key?:  string | null;
}

/**
 * Push relevant Tauri config fields to the backend HTTP API.
 * Fire-and-forget — errors are swallowed (backend may not be running).
 */
export async function syncBackendConfig(fields: BackendConfigSync): Promise<void> {
  try {
    await fetch(`${BASE_URL}/config`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(fields),
    });
  } catch { /* backend not ready — ignore */ }
}

export async function reverseIpLookup(
  ip: string,
  hostname?: string,
): Promise<ReverseIpResult> {
  const qs = hostname ? `?hostname=${encodeURIComponent(hostname)}` : '';
  return request<ReverseIpResult>(`/reverse-ip/${encodeURIComponent(ip)}${qs}`);
}
