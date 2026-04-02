import { getToken, clearToken, _authRef } from './auth';

export const API_BASE = process.env.REACT_APP_API_BASE || "";

function _handleResponse(res) {
  if (res.status === 401) {
    clearToken();
    _authRef.user = null;
    if (_authRef.logout) _authRef.logout();
    throw new Error("session_expired");
  }
  if (!res.ok) throw new Error(`API error ${res.status}`);
  return res.json();
}

export const api = {
  headers: () => {
    const tok = getToken();
    return {
      "Content-Type": "application/json",
      ...(tok ? { Authorization: `Bearer ${tok}` } : {}),
    };
  },
  get:   async (path)       => _handleResponse(await fetch(`${API_BASE}${path}`, { headers: api.headers() })),
  post:  async (path, body) => _handleResponse(await fetch(`${API_BASE}${path}`, { method: "POST",   headers: api.headers(), body: JSON.stringify(body) })),
  del:   async (path)       => _handleResponse(await fetch(`${API_BASE}${path}`, { method: "DELETE", headers: api.headers() })),
  patch: async (path, body) => _handleResponse(await fetch(`${API_BASE}${path}`, { method: "PATCH",  headers: api.headers(), body: JSON.stringify(body) })),
};
