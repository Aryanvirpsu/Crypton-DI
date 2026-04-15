import { CryptonError } from './errors';
import { getSessionToken, clearSessionToken } from './session';
import { ApiResponse } from './types';

export class CryptonTransport {
  constructor(private readonly endpoint: string) {}

  public async request<T>(path: string, options?: RequestInit): Promise<T> {
    const token = getSessionToken();
    const headers = new Headers(options?.headers || {});
    headers.set('Content-Type', 'application/json');
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
      const err = json as { success: false; error: { code: string; message: string } };
      throw new CryptonError(err.error.code, err.error.message);
    }

    return (json as { success: true; data: T }).data;
  }

  get = <T>(path: string) => this.request<T>(path, { method: 'GET' });
  post = <T>(path: string, body?: any) => this.request<T>(path, { method: 'POST', body: JSON.stringify(body) });
  del = <T>(path: string) => this.request<T>(path, { method: 'DELETE' });
  patch = <T>(path: string, body?: any) => this.request<T>(path, { method: 'PATCH', body: JSON.stringify(body) });

  /** Raw fetch with auth headers — for non-JSON responses (e.g. blob downloads). */
  public async rawFetch(path: string, options?: RequestInit): Promise<Response> {
    const token = getSessionToken();
    const headers = new Headers(options?.headers || {});
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }
    return fetch(`${this.endpoint}${path}`, { ...options, headers });
  }
}
