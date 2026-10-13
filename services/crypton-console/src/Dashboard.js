import { useState, useEffect } from 'react';
import { crypton } from './sdk';
import { getValidToken } from './auth';
import { BtnF } from './Buttons';
import AppShell from './AppShell';

function SkeletonBox({ h = 80, w = '100%', mb = 0 }) {
  return (
    <div style={{ height: h, width: w, background: 'var(--ink-3)', backgroundImage: 'linear-gradient(90deg, var(--ink-3) 0%, rgba(255,255,255,0.05) 50%, var(--ink-3) 100%)', backgroundSize: '200% 100%', animation: 'shimmer 1.6s ease-in-out infinite', marginBottom: mb, borderRadius: 2 }} />
  );
}

function DashboardSkeleton() {
  return (
    <div style={{ padding: '36px 44px', flex: 1 }}>
      <div style={{ marginBottom: 28, display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 1, background: 'var(--line)' }} className="stat-grid">
        {[0, 1, 2].map(i => (
          <div key={i} style={{ background: 'var(--ink-2)', padding: '28px 24px' }}>
            <SkeletonBox h={12} w="60%" mb={14} />
            <SkeletonBox h={52} w="50%" mb={10} />
            <SkeletonBox h={10} w="70%" />
          </div>
        ))}
      </div>
      <SkeletonBox h={12} w="20%" mb={16} />
      <div style={{ border: '1px solid var(--line)' }}>
        {[0, 1, 2, 3, 4].map(i => (
          <div key={i} style={{ display: 'flex', gap: 16, padding: '16px 20px', borderBottom: '1px solid var(--line)', background: 'var(--ink-2)', alignItems: 'center' }}>
            <SkeletonBox h={32} w={32} mb={0} />
            <div style={{ flex: 1 }}>
              <SkeletonBox h={13} w="55%" mb={6} />
              <SkeletonBox h={10} w="40%" mb={0} />
            </div>
            <SkeletonBox h={10} w={60} mb={0} />
          </div>
        ))}
      </div>
    </div>
  );
}

function StatCard({ l, v, i, vc, go, link }) {
  return (
    <div onClick={() => link && go(link)}
      style={{ background: "var(--ink-2)", padding: "28px 24px", position: "relative", overflow: "hidden", transition: "background .25s", cursor: link ? "pointer" : "default" }}
      onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"}
      onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 14 }}>{l}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 52, letterSpacing: ".02em", lineHeight: 1, color: vc }}>{v}</div>
      <div style={{ position: "absolute", top: 20, right: 20, fontSize: 20, opacity: .25 }}>{i}</div>
      {link && <div style={{ position: "absolute", bottom: 10, right: 14, fontFamily: "var(--mono)", fontSize: 8, color: "var(--muted2)", letterSpacing: ".06em" }}>→</div>}
    </div>
  );
}

function ActivityItem({ ico, type, title, meta, time, go, link }) {
  const icoStyle = {
    s: { borderColor: "rgba(74,222,128,.3)", background: "var(--s-success)" },
    w: { borderColor: "rgba(251,191,36,.3)", background: "var(--s-warning)" },
    i: { borderColor: "rgba(200,245,90,.2)", background: "var(--accent-dim)" },
  }[type] || {};
  return (
    <div onClick={() => link && go(link)}
      style={{ display: "flex", alignItems: "center", gap: 16, padding: "16px 20px", borderBottom: "1px solid var(--line)", background: "var(--ink-2)", transition: "background .15s", cursor: link ? "pointer" : "default" }}
      onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"}
      onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
      <div style={{ width: 32, height: 32, flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 14, border: "1px solid var(--line)", ...icoStyle }}>{ico}</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 13, fontWeight: 500 }}>{title}</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", marginTop: 2, letterSpacing: ".04em", overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{meta}</div>
      </div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)", whiteSpace: "nowrap", letterSpacing: ".04em", flexShrink: 0 }}>{time}</div>
    </div>
  );
}

export default function Dashboard({ go, toast }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [stats, setStats] = useState(null);
  const [activity, setActivity] = useState(null);

  useEffect(() => {
    if (!getValidToken()) { setLoading(false); return; }
    Promise.all([crypton.dashboard.stats(), crypton.dashboard.activity()])
      .then(([s, a]) => { setStats(s); setActivity(a); })
      .catch(() => setError(true))
      .finally(() => setLoading(false));
  }, []);

  return (
    <AppShell active="dashboard" go={go}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, borderBottom: "1px solid var(--line)" }} className="page-header">
        <div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Dashboard</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Security overview</div>
        </div>
        <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Add Passkey</BtnF>
      </div>

      {loading ? (
        <DashboardSkeleton />
      ) : error ? (
        <div style={{ padding: "60px 44px", textAlign: "center" }}>
          <div style={{ fontSize: 40, marginBottom: 16 }}>⚠</div>
          <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>Couldn't Load Dashboard</div>
          <div style={{ fontSize: 13, color: "var(--muted)" }}>The server didn't return your dashboard data. Try refreshing.</div>
        </div>
      ) : (
        <div style={{ padding: "36px 44px 60px", flex: 1 }} className="page-body">
          <div className="pg-in stat-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", marginBottom: 28, border: "1px solid var(--line)" }}>
            <StatCard l="Active Devices" v={String(stats.activeDevices)} i="📱" link="devices" go={go} />
            <StatCard l="Auth Events (24h)" v={String(stats.authEvents24h)} i="⚡" go={go} />
            <StatCard l="Security Score" v={`${stats.securityScore}%`} i="🛡" vc="var(--success)" go={go} />
          </div>

          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 16, display: "flex", alignItems: "center", gap: 14 }} className="pg-in">
            Recent Activity
            <div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          {activity.length === 0 ? (
            <div style={{ border: "1px solid var(--line)", padding: "40px", textAlign: "center", color: "var(--muted)", fontSize: 13 }}>No activity yet.</div>
          ) : (
            <div className="pg-in activity-feed" style={{ display: "flex", flexDirection: "column", border: "1px solid var(--line)" }}>
              {activity.map(a => <ActivityItem key={a.id} {...a} go={go} />)}
            </div>
          )}
        </div>
      )}
    </AppShell>
  );
}
