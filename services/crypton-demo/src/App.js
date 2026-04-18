import { useState, useEffect, useCallback, useMemo } from "react";
import { AuthContext } from './AuthContext';
import { getToken, clearToken, getAdminToken, clearAdminToken, parseJwt, PROTECTED_PAGES, ADMIN_PAGES, _authRef, _adminAuthRef } from './auth';
import { api, API_BASE } from './api';
import { PAGE_TO_PATH, getPageFromPath } from './routes';
import { PAGE_LABELS } from './constants';
import AppShell from './AppShell';
import { BtnF, BtnO } from './Buttons';
import { useToasts } from './hooks';

import Landing from './Landing';
import Login from './Login';
import AdminLogin from './AdminLogin';
import Register from './Register';
import Dashboard from './Dashboard';
import Devices from "./Devices";
import DemoActions from "./demo-app/DemoActions";

import Admin from './Admin';
import AuditLogs from './AuditLogs';
import Sessions from './Sessions';
import Recovery from './Recovery';
import RBAC from './RBAC';
import PolicyEngine from './PolicyEngine';
import RiskIntel from './RiskIntel';
import OrgSettings from './OrgSettings';

/* ─────────────────────────────────────────────────────────────
   CRYPTON DEMO — SaaS app + Operator Panel
   ───────────────────────────────────────────────────────────── */

const FontLink = () => (
  <style>{`
    @import url('https://fonts.googleapis.com/css2?family=Bebas+Neue&family=DM+Serif+Display:ital@0;1&family=DM+Mono:wght@400;500&family=Geist:wght@300;400;500&display=swap');
    :root {
      --ink:#0A0A0A;--ink-2:#111111;--ink-3:#161616;--ink-4:#1C1C1C;
      --paper:#F4F1EC;--off:#E8E4DC;--muted:#7A7570;--muted2:#5A5550;
      --line:rgba(244,241,236,0.07);--line2:rgba(244,241,236,0.13);
      --accent:#C8F55A;--accent-dim:rgba(200,245,90,0.1);
      --success:#4ADE80;--warning:#FBBF24;--danger:#F87171;
      --s-success:rgba(74,222,128,0.1);--s-warning:rgba(251,191,36,0.1);--s-danger:rgba(248,113,113,0.1);
      --serif:'DM Serif Display',serif;
      --display:'Bebas Neue',sans-serif;
      --mono:'DM Mono',monospace;
      --body:'Geist',sans-serif;
    }
    *,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
    body{font-family:var(--body);background:var(--ink);color:var(--paper);line-height:1.6;font-size:15px}
    .grain{position:fixed;inset:0;z-index:9000;pointer-events:none;background-image:url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");opacity:.035}
  `}</style>
);

function ToastStack({ toasts }) {
  return (
    <div style={{ position: "fixed", bottom: 24, right: 24, zIndex: 8000, display: "flex", flexDirection: "column", gap: 6 }}>
      {toasts.map(t => (
        <div key={t.id} style={{
          background: "var(--ink-2)", border: "1px solid var(--line)",
          borderLeft: `2px solid ${t.type === "danger" ? "var(--danger)" : t.type === "success" ? "var(--success)" : "var(--accent)"}`,
          padding: "11px 16px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em",
          color: "var(--paper)", minWidth: 220
        }}>{`// ${t.msg}`}</div>
      ))}
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState(getPageFromPath);
  const [toasts, addToast] = useToasts();
  
  // User Auth
  const [authUser, setAuthUser] = useState(() => { const t = getToken(); return t ? parseJwt(t) : null; });
  // Admin Auth
  const [adminUser, setAdminUser] = useState(() => { const t = getAdminToken(); return t ? parseJwt(t) : null; });

  const [authReady, setAuthReady] = useState(false);

  const go = useCallback(id => {
    if (PROTECTED_PAGES.has(id) && !getToken()) {
      const path = PAGE_TO_PATH["login"] || "/login";
      window.history.pushState({ page: "login" }, "", path);
      setPage("login");
      return;
    }
    if (ADMIN_PAGES.has(id) && !getAdminToken()) {
      const path = PAGE_TO_PATH["admin_login"] || "/admin/login";
      window.history.pushState({ page: "admin_login" }, "", path);
      setPage("admin_login");
      return;
    }
    const path = PAGE_TO_PATH[id] || "/";
    window.history.pushState({ page: id }, "", path);
    setPage(id);
    window.scrollTo({ top: 0 });
  }, []);

  useEffect(() => {
    window.scrollTo({ top: 0, behavior: "instant" });
    
    // Check User Token
    const tok = getToken();
    if (tok) {
      const parsed = parseJwt(tok);
      if (parsed && parsed.exp && parsed.exp * 1000 > Date.now()) setAuthUser(parsed);
      else { clearToken(); setAuthUser(null); }
    }
    
    // Check Admin Token
    const admtok = getAdminToken();
    if (admtok) {
      const parsedA = parseJwt(admtok);
      if (parsedA && parsedA.exp && parsedA.exp * 1000 > Date.now()) setAdminUser(parsedA);
      else { clearAdminToken(); setAdminUser(null); }
    }
    
    const initialPage = getPageFromPath();
    if (PROTECTED_PAGES.has(initialPage) && !getToken()) {
      const path = PAGE_TO_PATH["login"];
      window.history.replaceState({ page: "login" }, "", path);
      setPage("login");
    } else if (ADMIN_PAGES.has(initialPage) && !getAdminToken()) {
      const path = PAGE_TO_PATH["admin_login"];
      window.history.replaceState({ page: "admin_login" }, "", path);
      setPage("admin_login");
    }
    setAuthReady(true);
  }, []);

  useEffect(() => {
    const onPop = () => {
      const p = getPageFromPath();
      if (ADMIN_PAGES.has(p) && !getAdminToken()) {
        window.history.replaceState({ page: "admin_login" }, "", PAGE_TO_PATH["admin_login"]);
        setPage("admin_login");
      } else if (PROTECTED_PAGES.has(p) && !getToken()) {
        window.history.replaceState({ page: "login" }, "", PAGE_TO_PATH["login"]);
        setPage("login");
      } else {
        setPage(p);
      }
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const toast = useCallback((msg, type = "info") => addToast(msg, type), [addToast]);

  const logout = useCallback(() => {
    import('./sdk').then(({ crypton }) => crypton.auth.logout()).catch(() => {});
    clearToken();
    setAuthUser(null);
    go("login");
  }, [go]);
  
  const adminLogout = useCallback(() => {
    const tok = getAdminToken();
    if (tok) {
      fetch(`${API_BASE}/auth/logout`, { method: 'POST', headers: { Authorization: `Bearer ${tok}` } }).catch(() => {});
    }
    clearAdminToken();
    setAdminUser(null);
    go("admin_login");
  }, [go]);

  _authRef.logout = logout;
  _authRef.setUser = setAuthUser;
  _adminAuthRef.logout = adminLogout;
  _adminAuthRef.setUser = setAdminUser;

  // We provide the active user based on whether we are in an admin page or not.
  // Wait, if an admin page uses AuthContext, it needs the adminUser.
  // We can dynamically swap out the provided context value.
  const isAdminPage = ADMIN_PAGES.has(page) || page === "admin_login";
  const authCtx = useMemo(() => ({ 
    authUser: isAdminPage ? adminUser : authUser, 
    authReady, 
    logout: isAdminPage ? adminLogout : logout 
  }), [authUser, adminUser, authReady, logout, adminLogout, isAdminPage]);

  return (
    <AuthContext.Provider value={authCtx}>
      <FontLink />
      <div className="grain" />
      <ToastStack toasts={toasts} />
      {page === "landing"    && <Landing go={go} toast={toast} />}
      {page === "login"      && <Login go={go} toast={toast} />}
      {page === "admin_login"&& <AdminLogin go={go} toast={toast} />}
      {page === "register"   && <Register go={go} toast={toast} />}
      
      {authReady && (
        <>
          {page === "dashboard"  && <Dashboard go={go} toast={toast} />}
          {page === "demo"       && <DemoActions go={go} toast={toast} />}
          {page === "devices"    && <Devices go={go} toast={toast} />}
          
          {page === "admin"      && <Admin go={go} toast={toast} />}
          {page === "auditlogs"  && <AuditLogs go={go} toast={toast} />}
          {page === "sessions"   && <Sessions go={go} toast={toast} />}
          {page === "recovery"   && <Recovery go={go} toast={toast} />}
          {page === "rbac"       && <RBAC go={go} toast={toast} />}
          {page === "policy"     && <PolicyEngine go={go} toast={toast} />}
          {page === "risk"       && <RiskIntel go={go} toast={toast} />}
          {page === "orgsettings"&& <OrgSettings go={go} toast={toast} />}
        </>
      )}
    </AuthContext.Provider>
  );
}

export { api, getToken, clearToken, parseJwt, BtnF, BtnO, AppShell, PAGE_LABELS };