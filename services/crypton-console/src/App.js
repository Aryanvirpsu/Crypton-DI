import { useState, useEffect, useCallback, useMemo } from "react";
import { AuthContext } from './AuthContext';
import { getValidToken, clearToken, parseJwt, PROTECTED_PAGES, _authRef } from './auth';
import { crypton } from './sdk';
import { PAGE_TO_PATH, getPageFromPath } from './routes';
import { MAIN_URL } from './config';
import { useToasts } from './hooks';

import Login from './Login';
import Register from './Register';
import Dashboard from './Dashboard';
import Devices from './Devices';
import Recovery from './Recovery';
import Actions from './Actions';
import AdminOverview from './AdminOverview';
import AdminUsers from './AdminUsers';
import AdminAuditLogs from './AdminAuditLogs';

/* ─────────────────────────────────────────────────────────────
   CRYPTON CONSOLE — one app, one login, real data only.
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
          padding: "11px 16px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", color: "var(--paper)", minWidth: 220
        }}>
          {`// ${t.msg}`}
        </div>
      ))}
    </div>
  );
}

function PageLoadSkeleton() {
  const sk = (h, w = '100%', mb = 8) => (
    <div style={{ height: h, width: w, background: 'var(--ink-3)', backgroundImage: 'linear-gradient(90deg, var(--ink-3) 0%, rgba(255,255,255,0.05) 50%, var(--ink-3) 100%)', backgroundSize: '200% 100%', animation: 'shimmer 1.6s ease-in-out infinite', marginBottom: mb, borderRadius: 2 }} />
  );
  return (
    <div style={{ minHeight: '100vh', background: 'var(--ink)', display: 'flex' }}>
      <div style={{ width: 200, borderRight: '1px solid var(--line)', padding: '20px 12px', flexShrink: 0 }}>
        {sk(24, '80%', 24)}
        {[0, 1, 2].map(i => <div key={i} style={{ marginBottom: 4 }}>{sk(32)}</div>)}
      </div>
      <div style={{ flex: 1, padding: '36px 44px' }}>
        {sk(36, '36%', 8)}
        {sk(12, '18%', 32)}
        {sk(120, '100%', 0)}
      </div>
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState(getPageFromPath);
  const [toasts, addToast] = useToasts();

  const [authUser, setAuthUser] = useState(() => {
    const t = getValidToken();
    return t ? parseJwt(t) : null;
  });
  // Role is NOT trusted client-side data — it's a UI convenience fetched
  // from GET /me so the sidebar can show/hide admin links. The backend's
  // AdminUser extractor is what actually enforces authorization; a stale
  // or absent role here only affects what's *shown*, never what's allowed.
  const [role, setRole] = useState(null);
  const [authReady, setAuthReady] = useState(false);

  const toast = useCallback((msg, type = "info") => addToast(msg, type), [addToast]);

  const go = useCallback(id => {
    if (id === "landing") {
      const path = getValidToken() ? "/dashboard" : "/login";
      window.history.pushState({}, "", path);
      setPage(getValidToken() ? "dashboard" : "login");
      return;
    }
    if (PROTECTED_PAGES.has(id) && !getValidToken()) {
      window.history.pushState({ page: "login" }, "", PAGE_TO_PATH["login"]);
      setPage("login");
      return;
    }
    const path = PAGE_TO_PATH[id] || "/";
    window.history.pushState({ page: id }, "", path);
    setPage(id);
    window.scrollTo({ top: 0 });
  }, []);

  // Resolve "/" and initial auth-gating on first mount.
  useEffect(() => {
    const initial = getPageFromPath();
    const authed = !!getValidToken();

    if (initial === "landing") {
      const target = authed ? "dashboard" : "login";
      window.history.replaceState({}, "", PAGE_TO_PATH[target]);
      setPage(target);
    } else if (PROTECTED_PAGES.has(initial) && !authed) {
      window.history.replaceState({}, "", PAGE_TO_PATH["login"]);
      setPage("login");
    }
    setAuthReady(true);
  }, []);

  useEffect(() => {
    const onPop = () => {
      const p = getPageFromPath();
      if (PROTECTED_PAGES.has(p) && !getValidToken()) {
        window.history.replaceState({}, "", PAGE_TO_PATH["login"]);
        setPage("login");
      } else {
        setPage(p === "landing" ? (getValidToken() ? "dashboard" : "login") : p);
      }
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  // Whenever we have an authenticated user, resolve their role from the
  // server. This is the single source of truth for "is this an admin" —
  // there is no separate admin login or admin token in this app.
  useEffect(() => {
    if (!authUser) { setRole(null); return; }
    let cancelled = false;
    crypton.me.get()
      .then(me => { if (!cancelled) setRole(me.role); })
      .catch(() => { if (!cancelled) setRole('user'); });
    return () => { cancelled = true; };
  }, [authUser]);

  const logout = useCallback(() => {
    crypton.auth.logout().catch(() => {});
    clearToken();
    setAuthUser(null);
    go("login");
  }, [go]);

  _authRef.logout = logout;
  _authRef.setUser = setAuthUser;

  const authCtx = useMemo(() => ({ authUser, role, authReady, logout }), [authUser, role, authReady, logout]);

  const isAdmin = role === "admin" || role === "super_admin";

  return (
    <AuthContext.Provider value={authCtx}>
      <FontLink />
      <div className="grain" />
      <ToastStack toasts={toasts} />

      {!authReady && PROTECTED_PAGES.has(page) && <PageLoadSkeleton />}

      {authReady && (
        <>
          {page === "login" && <Login go={go} toast={toast} />}
          {page === "register" && <Register go={go} toast={toast} />}

          {page === "dashboard" && <Dashboard go={go} toast={toast} />}
          {page === "devices" && <Devices go={go} toast={toast} />}
          {page === "recovery" && <Recovery go={go} toast={toast} />}
          {page === "actions" && <Actions go={go} toast={toast} />}

          {page === "admin" && <AdminOverview go={go} toast={toast} isAdmin={isAdmin} />}
          {page === "admin_users" && <AdminUsers go={go} toast={toast} isAdmin={isAdmin} />}
          {page === "admin_audit_logs" && <AdminAuditLogs go={go} toast={toast} isAdmin={isAdmin} />}
        </>
      )}
    </AuthContext.Provider>
  );
}

export { MAIN_URL };
