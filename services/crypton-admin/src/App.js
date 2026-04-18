import { useState, useEffect, useCallback, useMemo } from "react";
import { AuthContext } from './AuthContext';
import Devices from "./Devices";
import AuditLogs from "./AuditLogs";
import { getToken, setToken, clearToken, parseJwt, getValidToken, PROTECTED_PAGES, _authRef } from './auth';
import { api } from './api';
import { PAGE_TO_PATH, getPageFromPath } from './routes';
import { PAGE_LABELS } from './constants';
import AppShell from './AppShell';
import { BtnF, BtnO } from './Buttons';
import { useToasts } from './hooks';
import Login from './Login';
import Register from './Register';
import Recovery from './Recovery';
import Admin from './Admin';
import RiskIntel from './RiskIntel';
import Sessions from './Sessions';
import RBAC from './RBAC';
import PolicyEngine from './PolicyEngine';
import OrgSettings from './OrgSettings';
import { MAIN_URL } from './config';

/* ─────────────────────────────────────────────────────────────
   CRYPTON ADMIN — Operator/Security Panel
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
    html{scroll-behavior:smooth}
    body{font-family:var(--body);background:var(--ink);color:var(--paper);overflow-x:hidden;line-height:1.6;cursor:auto;font-size:15px}
    .grain{position:fixed;inset:0;z-index:9000;pointer-events:none;background-image:url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");opacity:.035}
  `}</style>
);

function ToastStack({ toasts }) {
  return (
    <div style={{ position: "fixed", bottom: 24, right: 24, zIndex: 8000, display: "flex", flexDirection: "column", gap: 6 }}>
      {toasts.map(t => (
        <div key={t.id} className={t.out ? "toast-out" : "toast-in"} style={{
          background: "var(--ink-2)", border: "1px solid var(--line)",
          borderLeft: `2px solid ${t.type === "danger" ? "var(--danger)" : t.type === "success" ? "var(--success)" : "var(--accent)"}`,
          padding: "11px 16px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em",
          color: "var(--paper)", minWidth: 220
        }}>// {t.msg}</div>
      ))}
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState(getPageFromPath);
  const [toasts, addToast] = useToasts();
  const [authUser, setAuthUser] = useState(() => { const t = getValidToken(); return t ? parseJwt(t) : null; });
  const [authReady, setAuthReady] = useState(false);

  const go = useCallback(id => {
    if (PROTECTED_PAGES.has(id) && !getValidToken()) {
      const path = PAGE_TO_PATH["login"] || "/login";
      window.history.pushState({ page: "login" }, "", path);
      setPage("login");
      return;
    }
    const path = PAGE_TO_PATH[id] || "/";
    window.history.pushState({ page: id }, "", path);
    setPage(id);
    window.scrollTo({ top: 0 });
  }, []);

  useEffect(() => {
    window.scrollTo({ top: 0, behavior: "instant" });
    const valid = getValidToken(); // null if missing or expired; also auto-clears stale token
    setAuthUser(valid ? parseJwt(valid) : null);
    const initialPage = getPageFromPath();
    if (PROTECTED_PAGES.has(initialPage) && !valid) {
      const path = PAGE_TO_PATH["login"] || "/login";
      window.history.replaceState({ page: "login" }, "", path);
      setPage("login");
    }
    setAuthReady(true);
  }, []);

  useEffect(() => {
    const onPop = () => {
      const nextPage = getPageFromPath();
      if (PROTECTED_PAGES.has(nextPage) && !getValidToken()) {
        const path = PAGE_TO_PATH["login"] || "/login";
        window.history.replaceState({ page: "login" }, "", path);
        setPage("login");
        return;
      }
      setPage(nextPage);
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const toast = useCallback((msg, type = "info") => addToast(msg, type), [addToast]);

  const logout = useCallback(() => {
    import('./sdk').then(({ crypton }) => crypton.auth.logout()).catch(() => {});
    clearToken();
    setAuthUser(null);
    window.location.href = MAIN_URL;
  }, []);

  _authRef.logout = logout;
  _authRef.setUser = setAuthUser;

  useEffect(() => {
    setTimeout(() => toast("CRYPTON ADMIN — Systems operational.", "info"), 800);
  }, [toast]);

  const authCtx = useMemo(() => ({ authUser, authReady, logout }), [authUser, authReady, logout]);

  return (
    <AuthContext.Provider value={authCtx}>
      <FontLink />
      <div className="grain" />
      <ToastStack toasts={toasts} />

      {page === "login"      && <Login go={go} toast={toast} />}
      {page === "register"   && <Register go={go} toast={toast} />}

      {authReady && (
        <>
          {page === "admin"       && <Admin go={go} toast={toast} />}
          {page === "auditlogs"   && <AuditLogs go={go} toast={toast} />}
          {page === "devices"     && <Devices go={go} toast={toast} />}
          {page === "sessions"    && <Sessions go={go} toast={toast} />}
          {page === "recovery"    && <Recovery go={go} toast={toast} />}
          {page === "rbac"        && <RBAC go={go} toast={toast} />}
          {page === "policy"      && <PolicyEngine go={go} toast={toast} />}
          {page === "risk"        && <RiskIntel go={go} toast={toast} />}
          {page === "orgsettings" && <OrgSettings go={go} toast={toast} />}
        </>
      )}
    </AuthContext.Provider>
  );
}

export { api, getToken, setToken, clearToken, parseJwt, BtnF, BtnO, AppShell, PAGE_LABELS };
