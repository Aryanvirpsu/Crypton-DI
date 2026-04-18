import {
  getSessionToken,
  setSessionToken,
  clearSessionToken,
  parseJwt,
  TOKEN_KEY
} from '@crypton/sdk';

export const getToken = getSessionToken;
export const setToken = setSessionToken;
export const clearToken = clearSessionToken;
export { parseJwt };
export { TOKEN_KEY };

// Admin protected surfaces — only operator panel routes
export const PROTECTED_PAGES = new Set([
  "admin","devices","auditlogs","rbac","recovery","risk",
  "sessions","policy","orgsettings",
]);

// Returns the raw token if it exists and is not expired; otherwise clears it and returns null.
// Use this instead of getToken() wherever auth-gating decisions are made.
export function getValidToken() {
  const tok = getToken();
  if (!tok) return null;
  const parsed = parseJwt(tok);
  if (parsed && parsed.exp && parsed.exp * 1000 > Date.now()) return tok;
  clearToken();
  return null;
}

// Module-level auth ref — lets api/webauthn update App auth state without prop-drilling
export let _authRef = { user: null, logout: null, setUser: null };
