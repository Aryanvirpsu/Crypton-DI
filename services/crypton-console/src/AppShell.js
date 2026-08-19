import { useState } from 'react';
import { PAGE_LABELS } from './constants';
import { useAuth } from './AuthContext';
import { MAIN_URL } from './config';

function Breadcrumb({ page }) {
  return (
    <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted2)", display: "flex", alignItems: "center", gap: 6, marginBottom: 8 }}>
      <button onClick={() => window.location.href = MAIN_URL} style={{ background: "none", border: "none", cursor: "pointer", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted)", padding: 0, transition: "color .2s" }}
        onMouseEnter={e => e.target.style.color = "var(--accent)"} onMouseLeave={e => e.target.style.color = "var(--muted)"}>CRYPTON</button>
      <span>/</span>
      <span style={{ color: "var(--paper)" }}>{PAGE_LABELS[page] || page}</span>
    </div>
  );
}

function Sidebar({ active, go, isOpen, onClose }) {
  const { authUser, role, logout } = useAuth();
  const isAdmin = role === "admin" || role === "super_admin";

  const navBtn = (item) => (
    <button key={item.id} onClick={() => { go(item.id); onClose && onClose(); }} style={{
      display: "flex", alignItems: "center", gap: 10, padding: "9px 10px",
      cursor: "pointer", fontSize: 13, color: active === item.id ? "var(--paper)" : "var(--muted)",
      border: "none", background: active === item.id ? "rgba(200,245,90,.07)" : "none",
      width: "100%", textAlign: "left", fontFamily: "var(--body)", position: "relative",
      borderLeft: active === item.id ? "2px solid var(--accent)" : "2px solid transparent",
      transition: "background .15s, color .15s", marginBottom: 1
    }}>
      <span style={{ fontSize: 14, width: 18, flexShrink: 0 }}>{item.ico}</span>
      <span className="si-label">{item.label}</span>
      {item.badge && <span className="si-badge" style={{ marginLeft: "auto", fontFamily: "var(--mono)", fontSize: 8, background: "var(--accent)", color: "var(--ink)", padding: "2px 6px" }}>{item.badge}</span>}
    </button>
  );

  const sectionLabel = (text) => (
    <div className="sb-label-txt" style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted2)", padding: "4px 4px 6px", marginBottom: 2 }}>{text}</div>
  );

  const userNavItems = [
    { id: "dashboard", ico: "◈", label: "Dashboard" },
    { id: "devices",   ico: "📱", label: "Devices" },
    { id: "recovery",  ico: "🔒", label: "Recovery" },
    { id: "actions",   ico: "⚡", label: "Protected Actions" },
  ];

  const adminNavItems = [
    { id: "admin",            ico: "⚙", label: "Admin Overview" },
    { id: "admin_users",      ico: "👥", label: "Users & Roles" },
    { id: "admin_audit_logs", ico: "📋", label: "Audit Logs" },
  ];

  return (
    <aside className={`app-sidebar${isOpen ? " open" : ""}`} style={{ width: 220, flexShrink: 0, background: "var(--ink-2)", borderRight: "1px solid var(--line)", display: "flex", flexDirection: "column", overflowY: "auto", overflowX: "hidden" }}>
      <div style={{ padding: "16px 20px 14px", borderBottom: "1px solid var(--line)", display: "flex", flexDirection: "column", gap: 10 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer" }} onClick={() => window.location.href = MAIN_URL}>
            <div style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent)", flexShrink: 0 }} />
            <span className="sb-mark" style={{ fontFamily: "var(--display)", fontSize: 16, letterSpacing: ".12em" }}>CRYPTON</span>
          </div>
          {onClose && (
            <button onClick={onClose} style={{ background: "none", border: "none", color: "var(--muted)", cursor: "pointer", fontSize: 18, lineHeight: 1, padding: 4 }}>✕</button>
          )}
        </div>
        <a href={MAIN_URL} style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "rgba(255,255,255,.03)", border: "1px solid var(--line)", transition: "color .2s, background .2s", textDecoration: "none" }}
          onMouseEnter={e => { e.currentTarget.style.color = "var(--paper)"; e.currentTarget.style.background = "rgba(255,255,255,.06)"; }}
          onMouseLeave={e => { e.currentTarget.style.color = "var(--muted)"; e.currentTarget.style.background = "rgba(255,255,255,.03)"; }}>
          <span style={{ fontSize: 10 }}>←</span>
          <span className="si-label">Back to Home</span>
        </a>
      </div>

      <nav style={{ padding: "16px 12px", flex: 1 }}>
        {sectionLabel("Console")}
        {userNavItems.map(navBtn)}

        {isAdmin && (
          <div style={{ marginTop: 20, borderTop: "1px solid var(--line)", paddingTop: 14 }}>
            {sectionLabel("Operator Panel")}
            {adminNavItems.map(navBtn)}
          </div>
        )}
      </nav>

      <div style={{ padding: "12px 16px", borderTop: "1px solid var(--line)" }}>
        {authUser && (
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--muted)", marginBottom: 8, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} className="si-label">
            {authUser.username}{isAdmin ? ` · ${role}` : ""}
          </div>
        )}
        {logout && (
          <button onClick={logout} style={{ width: "100%", display: "flex", alignItems: "center", gap: 8, padding: "7px 10px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "rgba(248,113,113,.05)", border: "1px solid rgba(248,113,113,.15)", cursor: "pointer", transition: "background .2s" }}
            onMouseEnter={e => e.currentTarget.style.background = "rgba(248,113,113,.12)"}
            onMouseLeave={e => e.currentTarget.style.background = "rgba(248,113,113,.05)"}>
            <span style={{ fontSize: 10 }}>↪</span>
            <span className="si-label">Sign Out</span>
          </button>
        )}
      </div>
    </aside>
  );
}

function MobileHeader({ onMenuOpen }) {
  return (
    <div className="mobile-header" style={{ alignItems: "center", justifyContent: "space-between", padding: "14px 20px", background: "var(--ink-2)", borderBottom: "1px solid var(--line)", position: "sticky", top: 0, zIndex: 100, flexShrink: 0 }}>
      <span style={{ fontFamily: "var(--display)", fontSize: 18, letterSpacing: ".12em" }}>CRYPTON</span>
      <button onClick={onMenuOpen} style={{ background: "none", border: "1px solid var(--line2)", color: "var(--paper)", cursor: "pointer", fontSize: 16, padding: "6px 10px", lineHeight: 1 }}>☰</button>
    </div>
  );
}

export default function AppShell({ active, go, children }) {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  return (
    <div style={{ display: "flex", flexDirection: "row", minHeight: "100vh", overflow: "hidden" }}>
      <div
        className={`sidebar-backdrop${sidebarOpen ? " visible" : ""}`}
        onClick={() => setSidebarOpen(false)}
        style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", zIndex: 9400 }}
      />

      <Sidebar active={active} go={go} isOpen={sidebarOpen} onClose={() => setSidebarOpen(false)} />

      <main className="app-main" style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", minWidth: 0 }}>
        <MobileHeader onMenuOpen={() => setSidebarOpen(true)} />
        <div className="breadcrumb-wrap" style={{ padding: "14px 44px 0", borderBottom: "none" }}>
          <Breadcrumb page={active} />
        </div>
        {children}
      </main>
    </div>
  );
}
