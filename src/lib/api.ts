import { invoke, isTauri } from '@tauri-apps/api/core';
export const native = isTauri();
export async function api<T>(name: string, data: Record<string, unknown> = {}): Promise<T> {
  if (native) return invoke<T>('request', { name, data });
  if (import.meta.env.DEV && import.meta.env.VITE_CORE_ENDPOINT) {
    const response = await fetch(import.meta.env.VITE_CORE_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Test-Token': import.meta.env.VITE_TEST_TOKEN,
      },
      body: JSON.stringify({ name, data }),
    });
    const value: unknown = await response.json();
    if (!response.ok) throw value;
    return value as T;
  }
  throw new Error(
    'Open EO Glossary Editor as a desktop application. The browser preview needs the explicit local test server.',
  );
}
export function message(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) return String(error.message);
  return String(error);
}
export async function openLink(url: string): Promise<void> {
  const parsed = new URL(url);
  if (parsed.protocol !== 'https:') throw new Error('Only HTTPS links are supported.');
  if (native) {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } else window.open(url, '_blank', 'noopener,noreferrer');
}
