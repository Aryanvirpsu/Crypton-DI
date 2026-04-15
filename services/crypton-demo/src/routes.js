// crypton-demo routes — SaaS surface and Admin Operator surface
export const ROUTES = {
  "/": "landing",
  "/login": "login",
  "/register": "register",
  "/dashboard": "dashboard",
  "/devices": "devices",
  "/demo": "demo",
  
  // Admin Routes (Second Boundary)
  "/admin/login": "admin_login",
  "/admin": "admin",
  "/admin/auditlogs": "auditlogs",
  "/admin/rbac": "rbac",
  "/admin/risk": "risk",
  "/admin/sessions": "sessions",
  "/admin/recovery": "recovery",
  "/admin/policy": "policy",
  "/admin/orgsettings": "orgsettings"
};

export const PAGE_TO_PATH = Object.fromEntries(Object.entries(ROUTES).map(([k, v]) => [v, k]));

export function getPageFromPath() {
  return ROUTES[window.location.pathname] || "landing";
}
