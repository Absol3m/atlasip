import type { IpRecord } from '$lib/types/ip';
import { BASE_URL } from '$lib/services/atlasip';

export async function lookupIPs(ips: string[]): Promise<IpRecord[]> {
  const response = await fetch(`${BASE_URL}/lookup/bulk`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ targets: ips }),
  });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json() as Promise<IpRecord[]>;
}

export async function lookupSingle(ip: string): Promise<IpRecord> {
  const response = await fetch(`${BASE_URL}/lookup/ip/${encodeURIComponent(ip)}`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json() as Promise<IpRecord>;
}
