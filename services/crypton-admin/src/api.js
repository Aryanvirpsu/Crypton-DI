import { crypton } from './sdk';
import { _authRef } from './auth';

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
