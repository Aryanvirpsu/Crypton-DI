import { CryptonError } from './errors';
import { getSessionToken, clearSessionToken } from './session';
import { ApiResponse } from './types';

/** Single-use value for the gateway's replay protection (x-nonce). */
function newNonce(): string {
  const c = (globalThis as { crypto?: Crypto }).crypto;
  if (c?.randomUUID) return c.randomUUID();
  const b = new Uint8Array(16);
  if (c?.getRandomValues) c.getRandomValues(b);
  else for (let i = 0; i < b.length; i++) b[i] = Math.floor(Math.random() * 256);
  return Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
}

/**
 * crypton-gateway requires a fresh x-nonce and a current x-timestamp (unix
 * seconds) on every request; it rejects the request otherwise. Set on every
 * call so a retry is a new request, not a replay.
 */
function setReplayHeaders(headers: Headers): void {
  headers.set('x-nonce', newNonce());
  headers.set('x-timestamp', String(Math.floor(Date.now() / 1000)));
}

export class CryptonTransport {
  constructor(private readonly endpoint: string) {}

  public async request<T>(path: string, options?: RequestInit): Promise<T> {
    const token = getSessionToken();
    const headers = new Headers(options?.headers || {});
    headers.set('Content-Type', 'application/json');
    setReplayHeaders(headers);
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }

    const res = await fetch(`${this.endpoint}${path}`, {
      ...options,
      headers,
    });

    if (res.status === 401) {
      clearSessionToken();
      throw new CryptonError("session_expired", "Session Expired");
    }

    let json: ApiResponse<T>;
    try {
      json = await res.json();
    } catch {
      throw new CryptonError("network_error", `HTTP Error ${res.status}`);
    }

    if (!json.success) {
      throw new CryptonError(
        json.error?.code ?? "unknown_error",
        json.error?.message ?? "An error occurred",
      );
    }

    return json.data;
  }

  get = <T>(path: string) => this.request<T>(path, { method: 'GET' });
  post = <T>(path: string, body?: unknown) => this.request<T>(path, { method: 'POST', body: JSON.stringify(body) });
  patch = <T>(path: string, body?: unknown) => this.request<T>(path, { method: 'PATCH', body: JSON.stringify(body) });

  /** Raw fetch with auth headers — for non-JSON responses (e.g. blob downloads). */
  public async rawFetch(path: string, options?: RequestInit): Promise<Response> {
    const token = getSessionToken();
    const headers = new Headers(options?.headers || {});
    setReplayHeaders(headers);
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }
    return fetch(`${this.endpoint}${path}`, { ...options, headers });
  }
}
