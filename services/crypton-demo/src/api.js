import { crypton } from './sdk';
import { _authRef, _adminAuthRef, getAdminToken, clearAdminToken } from './auth';

export const API_BASE = process.env.REACT_APP_API_BASE || "";

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
  get:   (path)       => _wrap(crypton['transport'].request(path, { method: 'GET' })),
  post:  (path, body) => _wrap(crypton['transport'].request(path, { method: 'POST', body: JSON.stringify(body) })),
  del:   (path)       => _wrap(crypton['transport'].request(path, { method: 'DELETE' })),
  patch: (path, body) => _wrap(crypton['transport'].request(path, { method: 'PATCH', body: JSON.stringify(body) })),
};

async function _adminFetch(method, path, body) {
  const token = getAdminToken();
  const res = await fetch(API_BASE + path, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    ...(body !== undefined ? { body: JSON.stringify(body) } : {}),
  });
  if (res.status === 401) throw new Error("session_expired");
  const json = await res.json();
  if (!json.success) throw new Error(json.error?.message || 'API error');
  return json.data;
}

async function _adminWrap(promise) {
  try {
    return await promise;
  } catch (err) {
    if (err.message === "session_expired") {
      clearAdminToken();
      if (_adminAuthRef.logout) _adminAuthRef.logout();
    }
    throw err;
  }
}

export const adminApi = {
  get:   (path)       => _adminWrap(_adminFetch('GET',    path)),
  post:  (path, body) => _adminWrap(_adminFetch('POST',   path, body)),
  del:   (path)       => _adminWrap(_adminFetch('DELETE', path)),
  patch: (path, body) => _adminWrap(_adminFetch('PATCH',  path, body)),
};
