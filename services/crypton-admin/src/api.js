import { getSessionToken, clearSessionToken, CryptonError } from '@crypton/sdk';
import { _authRef } from './auth';

export const API_BASE = process.env.REACT_APP_API_BASE || "";

async function _request(path, options) {
  const token = getSessionToken();
  const headers = new Headers(options?.headers || {});
  headers.set('Content-Type', 'application/json');
  if (token) headers.set('Authorization', `Bearer ${token}`);

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

  if (res.status === 401) {
    clearSessionToken();
    throw new CryptonError("session_expired", "Session Expired");
  }

  if (res.status === 403) {
    throw new CryptonError("access_denied", "Access Denied");
  }

  let json;
  try {
    json = await res.json();
  } catch {
    throw new CryptonError("network_error", `HTTP Error ${res.status}`);
  }

  if (!json.success) {
    throw new CryptonError(json.error?.code ?? "unknown_error", json.error?.message ?? "An error occurred");
  }
  return json.data;
}

async function _wrap(promise) {
  try {
    return await promise;
  } catch (err) {
    if (err.code === "session_expired" || err.message === "Session Expired") {
      _authRef.user = null;
      if (_authRef.logout) _authRef.logout();
    }
    throw err;
  }
}

export const api = {
  get:   (path)       => _wrap(_request(path, { method: 'GET' })),
  post:  (path, body) => _wrap(_request(path, { method: 'POST', body: JSON.stringify(body) })),
  del:   (path)       => _wrap(_request(path, { method: 'DELETE' })),
  patch: (path, body) => _wrap(_request(path, { method: 'PATCH', body: JSON.stringify(body) })),
};
