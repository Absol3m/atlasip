const BASE_URL = "http://127.0.0.1:8080";

async function request<T>(path: string): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  return response.json() as Promise<T>;
}

export async function getHealth(): Promise<string> {
  const data = await request<{ status: string }>("/health");
  return data.status;
}
