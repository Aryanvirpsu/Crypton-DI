import { useState, useEffect, useCallback } from 'react';
import { crypton, CryptonError } from './sdk';
import AppShell from './AppShell';

const ROLES = ["user", "admin", "super_admin"];

export default function AdminUsers({ go, toast }) {
  const [loading, setLoading] = useState(true);
  const [forbidden, setForbidden] = useState(false);
  const [users, setUsers] = useState([]);
  const [busyId, setBusyId] = useState(null);

  const load = useCallback(() => {
    setLoading(true);
    setForbidden(false);
    crypton.admin.listUsers()
      .then(setUsers)
      .catch(err => {
        if (CryptonError.is(err) && err.code === 'admin_authorization_required') setForbidden(true);
        else toast(`Failed to load users: ${err.message}`, 'danger');
      })
      .finally(() => setLoading(false));
  }, [toast]);

  useEffect(() => { load(); }, [load]);

  const changeRole = async (id, role) => {
    setBusyId(id);
    try {
      await crypton.admin.updateUserRole(id, role);
      toast('Role updated', 'success');
      load();
    } catch (err) {
      // e.g. "cannot_demote_last_admin" from the backend safeguard
      toast(`Update failed: ${err.message}`, 'danger');
    } finally {
      setBusyId(null);
    }
  };

  return (
    <AppShell active="admin_users" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Users &amp; Roles</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Every user, real role, from the database</div>
      </div>

      <div className="page-body" style={{ padding: "28px 44px 60px" }}>
        {loading ? (
          <div style={{ textAlign: "center", padding: "60px 0", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>Loading...</div>
        ) : forbidden ? (
          <div style={{ border: "1px solid var(--line)", padding: "60px", textAlign: "center", background: "var(--ink-2)" }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>🔒</div>
            <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>Admin Access Required</div>
            <div style={{ fontSize: 13, color: "var(--muted)" }}>Go to <button onClick={() => go("admin")} style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", fontSize: 13, textDecoration: "underline" }}>Admin Overview</button> to claim admin access on a fresh deployment.</div>
          </div>
        ) : (
          <div style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
            <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr 1fr 1fr", padding: "10px 16px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
              {["Username", "Role", "Devices", "Created"].map(h => (
                <span key={h} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted2)" }}>{h}</span>
              ))}
            </div>
            {users.map((u, i) => (
              <div key={u.id} style={{ display: "grid", gridTemplateColumns: "2fr 1fr 1fr 1fr", padding: "12px 16px", borderBottom: i < users.length - 1 ? "1px solid var(--line)" : "none", alignItems: "center", background: i % 2 === 0 ? "var(--ink-2)" : "var(--ink)" }}>
                <span style={{ fontSize: 13 }}>{u.username}</span>
                <select value={u.role} disabled={busyId === u.id} onChange={e => changeRole(u.id, e.target.value)}
                  style={{ fontFamily: "var(--mono)", fontSize: 11, background: "var(--ink-3)", color: "var(--paper)", border: "1px solid var(--line2)", padding: "5px 8px", width: "fit-content" }}>
                  {ROLES.map(r => <option key={r} value={r}>{r}</option>)}
                </select>
                <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>{u.device_count}</span>
                <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{u.created_at || "—"}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </AppShell>
  );
}
