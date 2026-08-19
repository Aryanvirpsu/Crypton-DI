// crypton-console routes — one app, one login.
export const ROUTES = {
  "/": "landing",
  "/login": "login",
  "/register": "register",
  "/dashboard": "dashboard",
  "/devices": "devices",
  "/recovery": "recovery",
  "/actions": "actions",

  "/admin": "admin",
  "/admin/users": "admin_users",
  "/admin/audit-logs": "admin_audit_logs",
};

export const PAGE_TO_PATH = Object.fromEntries(Object.entries(ROUTES).map(([k, v]) => [v, k]));

export function getPageFromPath() {
  return ROUTES[window.location.pathname] || "landing";
}
