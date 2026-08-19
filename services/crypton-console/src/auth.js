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
export { parseJwt, TOKEN_KEY };

// Every page that requires a logged-in user. Admin-ness is NOT a separate
// token or a separate gate here — there is one JWT. `client.me.get()`
// tells the app the caller's role, and admin pages show a clear
// "access required" state (with a bootstrap affordance) instead of a
// second login flow if the backend 403s. The backend's AdminUser
// extractor is what actually enforces this — this list only decides
// whether to bounce to /login.
export const PROTECTED_PAGES = new Set([
  "dashboard", "devices", "recovery", "actions",
  "admin", "admin_users", "admin_audit_logs",
]);

// Returns the raw token if it exists and is not expired; otherwise clears
// it and returns null. Use this instead of getToken() wherever an
// auth-gating decision is made — this exact expiry check was the subject
// of a real regression (see __tests__/auth.regression.test.js).
export function getValidToken() {
  const tok = getToken();
  if (!tok) return null;
  const parsed = parseJwt(tok);
  if (parsed && parsed.exp && parsed.exp * 1000 > Date.now()) return tok;
  clearToken();
  return null;
}

// Module-level auth ref — lets non-React modules (none currently, kept for
// parity with the SDK's own error-driven logout-on-401 behavior) update
// App auth state without prop-drilling.
export let _authRef = { user: null, logout: null, setUser: null };
