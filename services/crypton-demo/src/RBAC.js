import { useState, useEffect } from 'react';
import { adminApi } from './api';
import { getAdminToken } from './auth';
import { MOCK_USERS, ROLES, ROLE_COLORS, ROLE_PERMS } from './constants';
import { BtnF } from './Buttons';
import AppShell from './AppShell';

const ROLE_MAP = {
  super_admin: "Super Admin", superadmin: "Super Admin",
  admin:       "Admin",
  analyst:     "Security Analyst", security_analyst: "Security Analyst",
  member:      "Viewer", viewer: "Viewer",
};
const normalizeRole = r => ROLE_MAP[r?.toLowerCase().replace(/\s+/g, "_")] || r || "Viewer";

export default function RBAC({ go, toast }) {
  const [users, setUsers] = useState(MOCK_USERS);
  const [selectedUser, setSelectedUser] = useState(null);

  useEffect(() => {
    if (!getAdminToken()) return;
    adminApi.get("/users").then(data => {
      if (Array.isArray(data) && data.length > 0) {
        setUsers(data.map(u => ({
          ...u,
          name: u.name || u.email,
          role: normalizeRole(u.role),
        })));
      }
    }).catch(() => {});
  }, []);

  const changeRole = async (id, newRole) => {
    try { await adminApi.patch(`/users/${id}/role`, { role: newRole }); } catch {}
    setUsers(u => u.map(x => x.id === id ? { ...x, role: newRole } : x));
    toast(`Role updated to ${newRole}`, "success");
    setSelectedUser(null);
  };

  return (
    <AppShell active="rbac" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Users & Roles</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Role-based access control · Least privilege</div>
        </div>
        <BtnF onClick={() => toast("Invite flow — full implementation pending", "info")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Invite User</BtnF>
      </div>
      <div className="page-body" style={{ padding: "28px 44px 60px" }}>

        {selectedUser && (
          <div onClick={() => setSelectedUser(null)} style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.85)", backdropFilter: "blur(12px)", display: "flex", alignItems: "center", justifyContent: "center" }}>
            <div onClick={e => e.stopPropagation()} className="modal-anim" style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 40, maxWidth: 420, width: "90%", position: "relative" }}>
              <button onClick={() => setSelectedUser(null)} style={{ position: "absolute", top: 16, right: 16, background: "none", border: "none", color: "var(--muted)", fontSize: 16, cursor: "pointer", fontFamily: "var(--mono)" }}>×</button>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 14 }}>// Assign Role</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 32, letterSpacing: ".04em", textTransform: "uppercase", lineHeight: .95, marginBottom: 20 }}>{selectedUser.name}</div>
              <div style={{ fontSize: 12, color: "var(--muted)", marginBottom: 24 }}>{selectedUser.email}</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {ROLES.map(r => (
                  <button key={r} onClick={() => changeRole(selectedUser.id, r)} style={{
                    padding: "12px 16px", display: "flex", alignItems: "center", justifyContent: "space-between",
                    background: selectedUser.role === r ? "rgba(200,245,90,.07)" : "var(--ink-3)",
                    border: `1px solid ${selectedUser.role === r ? "var(--accent)" : "var(--line)"}`,
                    cursor: "pointer", transition: "all .2s"
                  }}>
                    <span style={{ fontSize: 13, color: "var(--paper)" }}>{r}</span>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: ROLE_COLORS[r] }}>
                      {ROLE_PERMS[r].length} permissions {selectedUser.role === r ? "· current" : ""}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 24 }}>
          {ROLES.map(r => (
            <div key={r} style={{ background: "var(--ink-2)", padding: "18px 16px" }}>
              <div style={{ fontFamily: "var(--display)", fontSize: 16, letterSpacing: ".06em", textTransform: "uppercase", color: ROLE_COLORS[r], marginBottom: 10 }}>{r}</div>
              {ROLE_PERMS[r].map((p, i) => <div key={i} style={{ fontSize: 11, color: "var(--muted)", marginBottom: 4, display: "flex", gap: 6 }}><span style={{ color: "var(--accent)" }}>·</span>{p}</div>)}
            </div>
          ))}
        </div>

        <div className="rbac-grid" style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
          <div style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1fr 0.8fr 0.8fr", padding: "10px 16px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
            {["User","Role","Devices","Last Active","Action"].map(h => <span key={h} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted2)" }}>{h}</span>)}
          </div>
          {users.map((u, i) => (
            <div key={u.id} style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1fr 0.8fr 0.8fr", padding: "14px 16px", borderBottom: i < users.length - 1 ? "1px solid var(--line)" : "none", background: "var(--ink-2)", alignItems: "center", transition: "background .15s" }}
              onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <div style={{ width: 30, height: 30, background: "var(--ink-3)", border: "1px solid var(--line2)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--display)", fontSize: 13, color: "var(--accent)", flexShrink: 0 }}>{u.avatar || (u.email || u.name || "?")[0].toUpperCase()}</div>
                <div>
                  <div style={{ fontSize: 13, fontWeight: 500 }}>{u.name || u.email}</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)" }}>{u.email}</div>
                </div>
              </div>
              <div style={{ display: "inline-flex", padding: "3px 10px", background: `${ROLE_COLORS[u.role] || "var(--muted)"}15`, color: ROLE_COLORS[u.role] || "var(--muted)", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", width: "fit-content" }}>{u.role}</div>
              <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>{u.devices} enrolled</span>
              <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{u.lastActive || u.last_active || "—"}</span>
              <button onClick={() => setSelectedUser(u)} style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", background: "var(--accent-dim)", border: "1px solid rgba(200,245,90,.2)", padding: "5px 10px", cursor: "pointer" }}>Edit Role</button>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}
