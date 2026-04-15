import { useState, useEffect } from 'react';
import { api } from './api';
import { getToken } from './auth';
import { MOCK_SESSIONS } from './constants';
import AppShell from './AppShell';

export default function Sessions({ go, toast }) {
  const [sessions, setSessions] = useState(MOCK_SESSIONS);

  useEffect(() => {
    if (!getToken()) return;
    api.get("/sessions").then(data => {
      if (Array.isArray(data) && data.length > 0) {
        setSessions(data.map(s => ({
          id:       s.id,
          user:     s.user,
          device:   s.device || "Unknown",
          browser:  s.browser || s.ip || "—",
          loc:      s.loc || s.ip || "—",
          started:  s.started || (s.created_at ? new Date(s.created_at).toLocaleString() : "—"),
          duration: s.duration || (s.last_active ? new Date(s.last_active).toLocaleTimeString() : "—"),
          active:   typeof s.active === 'boolean' ? s.active : s.status === "active",
        })));
      }
    }).catch(() => {});
  }, []);

  const kill = async id => {
    try {
      await api.del(`/sessions/${id}`);
      setSessions(s => s.filter(x => x.id !== id));
      toast("Session terminated immediately", "danger");
    } catch (err) {
      toast(err?.message || "Failed to terminate session", "danger");
    }
  };

  const killAll = async () => {
    try {
      await api.del("/sessions");
      setSessions([]);
      toast("All sessions terminated — users signed out", "danger");
    } catch (err) {
      toast(err?.message || "Failed to terminate all sessions", "danger");
    }
  };

  return (
    <AppShell active="sessions" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Sessions</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Active sessions · Real-time monitor</div>
        </div>
        <button onClick={killAll} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: "pointer" }}>⚡ Kill All</button>
      </div>
      <div className="page-body" style={{ padding: "28px 44px 60px" }}>
        <div className="stat-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 24 }}>
          {[
            { l: "Active Sessions",   v: sessions.filter(s => s.active).length.toString(), c: "var(--success)" },
            { l: "Idle Sessions",     v: sessions.filter(s => !s.active).length.toString(), c: "var(--warning)" },
            { l: "Total Users Online",v: [...new Set(sessions.map(s => s.user))].length.toString(), c: "var(--accent)" },
          ].map((s, i) => (
            <div key={i} style={{ background: "var(--ink-2)", padding: "22px 20px" }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>{s.l}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 48, color: s.c }}>{s.v}</div>
            </div>
          ))}
        </div>

        {sessions.length === 0 ? (
          <div style={{ border: "1px solid var(--line)", padding: "60px", textAlign: "center", background: "var(--ink-2)" }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>✓</div>
            <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>All Clear</div>
            <div style={{ fontSize: 13, color: "var(--muted)" }}>All sessions have been terminated.</div>
          </div>
        ) : (<>
          <div className="sessions-table" style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
            <div style={{ display: "grid", gridTemplateColumns: "1.5fr 1.2fr 1.2fr 1.2fr 0.8fr 0.8fr", padding: "10px 16px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
              {["User","Device","Location","Started","Duration","Action"].map(h => <span key={h} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted2)" }}>{h}</span>)}
            </div>
            {sessions.map((s, i) => (
              <div key={s.id} style={{ display: "grid", gridTemplateColumns: "1.5fr 1.2fr 1.2fr 1.2fr 0.8fr 0.8fr", padding: "14px 16px", borderBottom: i < sessions.length - 1 ? "1px solid var(--line)" : "none", background: "var(--ink-2)", alignItems: "center", transition: "background .15s" }}
                onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 500 }}>{s.user}</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", marginTop: 2 }}>{s.browser}</div>
                </div>
                <span style={{ fontSize: 12, color: "var(--muted)" }}>{s.device}</span>
                <span style={{ fontSize: 12, color: "var(--muted)" }}>{s.loc}</span>
                <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)" }}>{s.started}</span>
                <div style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
                  <div style={{ width: 6, height: 6, borderRadius: "50%", background: s.active ? "var(--success)" : "var(--warning)", boxShadow: s.active ? "0 0 6px var(--success)" : "none" }} />
                  <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: s.active ? "var(--success)" : "var(--warning)" }}>{s.duration}</span>
                </div>
                <button onClick={() => kill(s.id)} style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--danger)", background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)", padding: "5px 10px", cursor: "pointer" }}>Kill</button>
              </div>
            ))}
          </div>
          <div className="sessions-cards" style={{ display: "none", flexDirection: "column", gap: 10 }}>
            {sessions.map(s => (
              <div key={s.id} style={{ background: "var(--ink-2)", border: "1px solid var(--line)", borderLeft: `3px solid ${s.active ? "var(--success)" : "var(--warning)"}`, padding: "16px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 10 }}>
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>{s.user}</div>
                    <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", marginTop: 3 }}>{s.browser} · {s.device}</div>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
                    <div style={{ width: 6, height: 6, borderRadius: "50%", background: s.active ? "var(--success)" : "var(--warning)" }} />
                    <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: s.active ? "var(--success)" : "var(--warning)" }}>{s.duration}</span>
                  </div>
                </div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", marginBottom: 12 }}>{s.loc} · {s.started}</div>
                <button onClick={() => kill(s.id)} style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--danger)", background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)", padding: "7px 14px", cursor: "pointer" }}>Kill Session</button>
              </div>
            ))}
          </div>
        </>)}
      </div>
    </AppShell>
  );
}
