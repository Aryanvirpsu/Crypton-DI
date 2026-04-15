import { useState, useEffect } from 'react';
import { getToken } from './auth';
import { api } from './api';
import { MOCK_DASHBOARD_STATS, MOCK_ACTIVITY } from './constants';
import { useReveal } from './hooks';
import { BtnF, BtnO } from './Buttons';
import AppShell from './AppShell';

const ORB_DATA = [
  { title: "All Secure",       desc: "All enrolled devices active and verified. No suspicious activity detected in the last 24 hours. Last sweep: 2 minutes ago.", cls: "",  ico: "🛡", type: "success" },
  { title: "Action Required",  desc: "2 devices have not authenticated recently. Review inactive device access policies.",                                          cls: "w", ico: "⚠", type: "warning" },
  { title: "Threat Detected",  desc: "Suspicious authentication attempt detected on Work Desktop. Immediate review recommended.",                                  cls: "r", ico: "🚨", type: "danger" },
];

function StatCard({ l, v, d, i, vc, go, link }) {
  return (
    <div className="stat-c" onClick={() => link && go(link)} style={{ background: "var(--ink-2)", padding: "28px 24px", position: "relative", overflow: "hidden", transition: "background .25s", cursor: link ? "pointer" : "default" }}
      onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 14 }}>{l}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 52, letterSpacing: ".02em", lineHeight: 1, color: vc }}>{v}</div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".06em", marginTop: 8 }}>{d}</div>
      <div style={{ position: "absolute", top: 20, right: 20, fontSize: 20, opacity: .25 }}>{i}</div>
      {link && <div style={{ position: "absolute", bottom: 10, right: 14, fontFamily: "var(--mono)", fontSize: 8, color: "var(--muted2)", letterSpacing: ".06em" }}>→</div>}
    </div>
  );
}

function ActivityItem({ ico, t, title, meta, time, go, link }) {
  const icoStyle = {
    s: { borderColor: "rgba(74,222,128,.3)",  background: "var(--s-success)" },
    w: { borderColor: "rgba(251,191,36,.3)",  background: "var(--s-warning)" },
    i: { borderColor: "rgba(200,245,90,.2)",  background: "var(--accent-dim)" },
  }[t] || {};
  return (
    <div onClick={() => link && go(link)} style={{ display: "flex", alignItems: "center", gap: 16, padding: "16px 20px", borderBottom: "1px solid var(--line)", background: "var(--ink-2)", transition: "background .15s", cursor: link ? "pointer" : "default" }}
      onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
      <div style={{ width: 32, height: 32, flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 14, border: "1px solid var(--line)", ...icoStyle }}>{ico}</div>
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 13, fontWeight: 500 }}>{title}</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", marginTop: 2, letterSpacing: ".04em" }}>{meta}</div>
      </div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)", whiteSpace: "nowrap", letterSpacing: ".04em" }}>{time}</div>
    </div>
  );
}

export default function Dashboard({ go, toast }) {
  const [orbIdx, setOrbIdx] = useState(0);
  const orb = ORB_DATA[orbIdx];
  const [stats, setStats] = useState(MOCK_DASHBOARD_STATS);
  const [activity, setActivity] = useState(MOCK_ACTIVITY);

  useReveal([orbIdx]);

  useEffect(() => {
    if (!getToken()) return;
    api.get("/dashboard/stats").then(data => {
      if (data && typeof data === 'object' && 'activeDevices' in data) setStats(data);
    }).catch(err => console.warn("Dashboard stats fetch failed, using mock:", err));
    api.get("/dashboard/activity").then(data => {
      if (Array.isArray(data) && data.length > 0) setActivity(data);
    }).catch(err => console.warn("Dashboard activity fetch failed, using mock:", err));
  }, []);

  const setOrb = i => {
    setOrbIdx(i);
    const msgs = ["All systems secure ✓", "Action recommended — review inactive devices", "ALERT: Threat detected — investigate immediately"];
    toast(msgs[i], ORB_DATA[i].type);
  };

  const orbStyle = orb.cls === "w"
    ? { background: "radial-gradient(circle at 35% 35%,rgba(251,191,36,.85),rgba(251,191,36,.35),transparent)", animation: "orbWarn 1.5s ease-in-out infinite" }
    : orb.cls === "r"
      ? { background: "radial-gradient(circle at 35% 35%,rgba(248,113,113,.85),rgba(248,113,113,.35),transparent)", animation: "orbDanger .9s ease-in-out infinite" }
      : { background: "radial-gradient(circle at 35% 35%,rgba(74,222,128,.85),rgba(74,222,128,.35),rgba(5,150,105,.1))", animation: "orbPulse 3s ease-in-out infinite" };

  return (
    <AppShell active="dashboard" go={go}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, borderBottom: "1px solid var(--line)" }} className="page-header">
        <div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Dashboard</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Security overview</div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Add Passkey</BtnF>
          <BtnO onClick={() => go("admin_login")} style={{ padding: "8px 16px", fontSize: 9 }}>Admin Dashboard →</BtnO>
        </div>
      </div>
      <div style={{ padding: "36px 44px 60px", flex: 1 }} className="page-body">
        <div className="pg-in stat-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", marginBottom: 28, border: "1px solid var(--line)" }}>
          {[
            { l: "Active Devices",    v: String(stats.activeDevices  ?? MOCK_DASHBOARD_STATS.activeDevices),  d: "↑ 1 this week",  i: "📱", link: "devices" },
            { l: "Auth Events (24h)", v: String(stats.authEvents24h  ?? MOCK_DASHBOARD_STATS.authEvents24h),  d: "All verified",   i: "⚡", link: null },
            { l: "Security Score",    v: `${stats.securityScore ?? MOCK_DASHBOARD_STATS.securityScore}%`,     d: "No issues found",i: "🛡", vc: "var(--success)", link: null },
          ].map((s, i) => <StatCard key={i} {...s} go={go} />)}
        </div>

        <div className="pg-in orb-grid" style={{ display: "grid", gridTemplateColumns: "auto 1fr", border: "1px solid var(--line)", marginBottom: 28, background: "var(--ink-2)" }}>
          <div className="orb-vis" onClick={() => setOrb((orbIdx + 1) % 3)} style={{ width: 180, display: "flex", alignItems: "center", justifyContent: "center", padding: 40, borderRight: "1px solid var(--line)", cursor: "pointer", position: "relative" }}>
            <div className="orb-pulse r1" style={{ position: "absolute" }} />
            <div className="orb-pulse r2" style={{ position: "absolute" }} />
            <div style={{ width: 96, height: 96, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 32, position: "relative", zIndex: 1, ...orbStyle }}>{orb.ico}</div>
          </div>
          <div style={{ padding: "36px 40px", display: "flex", flexDirection: "column", justifyContent: "center" }}>
            <div style={{ fontFamily: "var(--display)", fontSize: 32, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 10 }}>{orb.title}</div>
            <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 24, maxWidth: 500 }}>{orb.desc}</p>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {["Secure", "Warning", "Alert"].map((m, i) => (
                <button key={m} onClick={() => setOrb(i)} style={{
                  fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase",
                  padding: "7px 14px", border: `1px solid ${orbIdx === i ? "var(--accent)" : "var(--line2)"}`,
                  color: orbIdx === i ? "var(--accent)" : "var(--muted)", background: orbIdx === i ? "var(--accent-dim)" : "none", cursor: "pointer", transition: "all .2s"
                }}>{m}</button>
              ))}
              <button onClick={() => go("devices")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", padding: "7px 14px", border: "1px solid rgba(248,113,113,.3)", color: "var(--danger)", background: "var(--s-danger)", cursor: "pointer", marginLeft: "auto" }}>Manage Sessions</button>
            </div>
          </div>
        </div>

        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 16, display: "flex", alignItems: "center", gap: 14 }} className="pg-in">
          Recent Activity
          <div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <div className="pg-in" style={{ display: "flex", flexDirection: "column", border: "1px solid var(--line)" }}>
          {activity.map((a) => <ActivityItem key={a.id} {...a} t={a.type || a.status} go={go} />)}
        </div>
      </div>
    </AppShell>
  );
}
