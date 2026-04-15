import { useState, useEffect } from "react";
import { adminApi } from './api';
import { getAdminToken } from './auth';
import { BtnF } from './Buttons';
import AppShell from './AppShell';

export default function AuditLogs({ go, toast }) {
  const [logs, setLogs] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("ALL");

  useEffect(() => {
    if (!getAdminToken()) { setLoading(false); return; }
    adminApi.get("/audit-logs").then(data => {
      if (data && Array.isArray(data.logs)) setLogs(data.logs);
    }).catch(() => {}).finally(() => setLoading(false));
  }, []);

  const filters = ["ALL", "login", "register", "device_revoke", "device_mark_lost", "logout"];
  const filtered = logs.filter(r =>
    (filter === "ALL" || r.event_type === filter) &&
    (search === "" || (r.event_type || "").toLowerCase().includes(search.toLowerCase()))
  );

  const typeColor = { success: "var(--success)", danger: "var(--danger)", warning: "var(--warning)", info: "var(--accent)" };

  const exportCSV = () => {
    const rows = [["actor","event_type","status","credential_id","created_at","metadata"], ...logs.map(r => [r.actor || "", r.event_type, r.status, r.credential_id || "", r.created_at || "", JSON.stringify(r.metadata || {})])];
    const csv = rows.map(r => r.join(",")).join("\n");
    const a = document.createElement("a"); a.href = "data:text/csv," + encodeURIComponent(csv); a.download = "crypton-audit.csv"; a.click();
    toast("Audit log exported as CSV", "success");
  };

  return (
    <AppShell active="auditlogs" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Audit Logs</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Cryptographic event stream</div></div>
        <BtnF onClick={exportCSV} style={{ padding: "8px 16px", fontSize: 9 }}>↓ Export CSV</BtnF>
      </div>
      <div className="page-body" style={{ padding: "28px 44px 60px" }}>
        {/* Search + Filter */}
        <div style={{ display: "flex", gap: 12, marginBottom: 20, flexWrap: "wrap" }}>
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search actor, action..."
            style={{ flex: 1, minWidth: 200, padding: "10px 14px", background: "var(--ink-3)", border: "1px solid var(--line2)", color: "var(--paper)", fontFamily: "var(--body)", fontSize: 13, outline: "none" }} />
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {["ALL", "login", "register", "device_revoke", "device_mark_lost", "logout"].map(f => (
              <button key={f} onClick={() => setFilter(f)} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "8px 14px", border: `1px solid ${filter === f ? "var(--accent)" : "var(--line2)"}`, background: filter === f ? "var(--accent-dim)" : "none", color: filter === f ? "var(--accent)" : "var(--muted)", cursor: "pointer", transition: "all .2s" }}>{f === "ALL" ? "ALL" : f.toUpperCase()}</button>
            ))}
          </div>
        </div>

        {loading ? (
          <div style={{ textAlign: "center", padding: "60px 0", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>Loading audit logs...</div>
        ) : filtered.length === 0 ? (
          <div style={{ border: "1px solid var(--line)", padding: "60px", textAlign: "center", background: "var(--ink-2)" }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>📋</div>
            <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>No Logs</div>
            <div style={{ fontSize: 13, color: "var(--muted)" }}>Audit events will appear here after actions are performed.</div>
          </div>
        ) : (
          <>
            <div className="audit-table" style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
              <div style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1.5fr 1fr 1fr", padding: "10px 16px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
                {["Actor", "Action", "Credential ID", "Outcome", "Time"].map(h => <span key={h} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted2)" }}>{h}</span>)}
              </div>
              {filtered.map((r, i) => (
                <div key={r.id || i} style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1.5fr 1fr 1fr", padding: "13px 16px", borderBottom: i < filtered.length - 1 ? "1px solid var(--line)" : "none", background: i % 2 === 0 ? "var(--ink-2)" : "var(--ink)", alignItems: "center", transition: "background .15s" }}
                  onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = i % 2 === 0 ? "var(--ink-2)" : "var(--ink)"}>
                  <span style={{ fontSize: 12 }}>{r.actor || "—"}</span>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: typeColor[r.status] || "var(--accent)", letterSpacing: ".05em" }}>{r.event_type}</span>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{r.credential_id ? r.credential_id.slice(0, 8) + "..." : "—"}</span>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: r.status === "success" ? "var(--success)" : "var(--danger)" }}>{r.status}</span>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{r.created_at || "—"}</span>
                </div>
              ))}
            </div>
            {/* Mobile cards */}
            <div className="audit-cards" style={{ display: "none", flexDirection: "column", gap: 8 }}>
              {filtered.map((r, i) => (
                <div key={r.id || i} style={{ background: "var(--ink-2)", border: "1px solid var(--line)", borderLeft: `3px solid ${typeColor[r.status] || "var(--accent)"}`, padding: "14px 16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 8 }}>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: typeColor[r.status] || "var(--accent)", letterSpacing: ".05em" }}>{r.event_type}</span>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)" }}>{r.created_at || "—"}</span>
                  </div>
                  <div style={{ fontSize: 12, marginBottom: 4 }}>{r.actor || r.event_type}</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", display: "flex", gap: 12, flexWrap: "wrap" }}>
                    <span>{r.status}</span>
                  </div>
                </div>
              ))}
            </div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)", marginTop: 12, letterSpacing: ".06em" }}>{filtered.length} of {logs.length} events shown</div>
          </>
        )}
      </div>
    </AppShell>
  );
}
