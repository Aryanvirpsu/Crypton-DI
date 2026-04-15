// crypton-admin routes — operator/security panel only.
// Demo flows (DemoActions, demo-app) belong in crypton-demo (:3001). Do NOT add them here.
export const ROUTES = {
  "/": "login",            // default → login; redirects to admin after auth
  "/login": "login",
  "/admin": "admin",
  "/devices": "devices",
  "/audit-logs": "auditlogs",
  "/risk": "risk",
  "/sessions": "sessions",
  "/users": "rbac",
  "/recovery": "recovery",
  "/policy": "policy",
  "/org": "orgsettings",
};

export const PAGE_TO_PATH = Object.fromEntries(Object.entries(ROUTES).map(([k, v]) => [v, k]));

export function getPageFromPath() {
  return ROUTES[window.location.pathname] || "landing";
}
