import {
  getSessionToken,
  setSessionToken,
  clearSessionToken,
  parseJwt,
  TOKEN_KEY as SESSION_TOKEN_KEY
} from './sdk/session';

export const getToken = getSessionToken;
export const setToken = setSessionToken;
export const clearToken = clearSessionToken;
export { parseJwt };
export const TOKEN_KEY = SESSION_TOKEN_KEY;

// Admin protected surfaces — only operator panel routes
export const PROTECTED_PAGES = new Set([
  "admin","devices","auditlogs","rbac","recovery","risk",
  "sessions","policy","orgsettings",
]);

// Module-level auth ref — lets api/webauthn update App auth state without prop-drilling
export let _authRef = { user: null, logout: null, setUser: null };
