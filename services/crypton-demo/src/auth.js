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

export const ADMIN_TOKEN_KEY = "crypton_admin_token";
export const getAdminToken = () => localStorage.getItem(ADMIN_TOKEN_KEY);
export const setAdminToken = (t) => localStorage.setItem(ADMIN_TOKEN_KEY, t);
export const clearAdminToken = () => localStorage.removeItem(ADMIN_TOKEN_KEY);

export const PROTECTED_PAGES = new Set([
  "dashboard","devices","demo",
]);

export const ADMIN_PAGES = new Set([
  "auditlogs","rbac","recovery","risk","sessions","admin","policy","orgsettings"
]);

// Module-level auth ref — lets api/webauthn update App auth state without prop-drilling
export let _authRef = { user: null, logout: null, setUser: null };
export let _adminAuthRef = { user: null, logout: null, setUser: null };
