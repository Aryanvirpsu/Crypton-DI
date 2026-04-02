export const TOKEN_KEY = "crypton_token";
export const getToken  = () => localStorage.getItem(TOKEN_KEY);
export const setToken  = (t) => localStorage.setItem(TOKEN_KEY, t);
export const clearToken = () => localStorage.removeItem(TOKEN_KEY);

export function parseJwt(token) {
  try { return JSON.parse(atob(token.split('.')[1].replace(/-/g,'+').replace(/_/g,'/'))); } catch { return null; }
}

export const PROTECTED_PAGES = new Set([
  "dashboard","devices","auditlogs","rbac","recovery","risk",
  "sessions","admin","policy","orgsettings","actions",
]);

// Module-level auth ref — lets api/webauthn update App auth state without prop-drilling
export let _authRef = { user: null, logout: null, setUser: null };
