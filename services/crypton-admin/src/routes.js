export const ROUTES = {
  "/": "landing", "/login": "login", "/register": "register", "/dashboard": "dashboard",
  "/devices": "devices", "/actions": "actions", "/audit-logs": "auditlogs", "/risk": "risk",
  "/sessions": "sessions", "/users": "rbac", "/recovery": "recovery",
  "/policy": "policy", "/org": "orgsettings", "/admin": "admin",
};

export const PAGE_TO_PATH = Object.fromEntries(Object.entries(ROUTES).map(([k, v]) => [v, k]));

export function getPageFromPath() {
  return ROUTES[window.location.pathname] || "landing";
}
