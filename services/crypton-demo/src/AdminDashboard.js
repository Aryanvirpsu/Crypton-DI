import { useState, useEffect } from 'react';
import { getAdminToken } from './auth';
import AppShell from './AppShell';
import { BtnF, BtnO } from './Buttons';

const MOCK_ADMIN_STATS = {
  totalUsers: 142,
  activeSessions: 38,
  pendingRecoveries: 2,
  threatAlerts: 1,
};

const MOCK_ADMIN_ACTIVITY = [
  { id: "aa1", ico: "👤", type: "i", title: "New user registered",       meta: "alex-macbook · Passkey enrolled", time: "4m ago",  link: "rbac" },
  { id: "aa2", ico: "⚠",  type: "w", title: "Recovery request pending",  meta: "user: jane-doe · awaiting approval", time: "22m ago", link: "recovery" },
  { id: "aa3", ico: "✓",  type: "s", title: "Policy enforcement trigger", meta: "Geo-velocity block · CN/JP hop detected", time: "1h ago",  link: "auditlogs" },
  { id: "aa4", ico: "🔒", type: "i", title: "Session revoked",           meta: "Admin action · device: work-desktop-old", time: "2h ago",  link: "sessions" },
  { id: "aa5", ico: "📋", type: "i", title: "Audit export generated",    meta: "Last 30 days · CSV · 3,241 events", time: "5h ago",  link: "auditlogs" },
];

function SkeletonBox({ h = 80, w = '100%', mb = 0 }) {
  return (
    <div style={{
      height: h, width: w,
      background: 'var(--ink-3)',
      backgroundImage: 'linear-gradient(90deg, var(--ink-3) 0%, rgba(255,255,255,0.05) 50%, var(--ink-3) 100%)',
      backgroundSize: '200% 100%',
      animation: 'shimmer 1.6s ease-in-out infinite',
      marginBottom: mb,
      borderRadius: 2,
    }} />
  );
}

function AdminSkeleton() {
  return (
    <div style={{ padding: '36px 44px', flex: 1 }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 1, background: 'var(--line)', marginBottom: 28, border: '1px solid var(--line)' }} className="stat-grid">
        {[0,1,2,3].map(i => (
          <div key={i} style={{ background: 'var(--ink-2)', padding: '24px 20px' }}>
            <SkeletonBox h={10} w="60%" mb={12} />
            <SkeletonBox h={44} w="45%" mb={8} />
            <SkeletonBox h={10} w="70%" />
          </div>
        ))}
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 1, background: 'var(--line)', marginBottom: 28 }}>
        {[0,1].map(i => (
          <div key={i} style={{ background: 'var(--ink-2)', padding: '28px 24px', border: '1px solid var(--line)' }}>
            <SkeletonBox h={14} w="50%" mb={20} />
            {[0,1,2].map(j => <SkeletonBox key={j} h={40} w="100%" mb={4} />)}
          </div>
        ))}
      </div>
      <SkeletonBox h={12} w="20%" mb={16} />
      <div style={{ border: '1px solid var(--line)' }}>
        {[0,1,2,3,4].map(i => (
          <div key={i} style={{ display: 'flex', gap: 16, padding: '16px 20px', borderBottom: '1px solid var(--line)', background: 'var(--ink-2)', alignItems: 'center' }}>
            <SkeletonBox h={32} w={32} mb={0} />
            <div style={{ flex: 1 }}>
              <SkeletonBox h={13} w="50%" mb={6} />
              <SkeletonBox h={10} w="38%" mb={0} />
            </div>
            <SkeletonBox h={10} w={55} mb={0} />
          </div>
        ))}
      </div>
    </div>
  );
}

function StatCard({ l, v, d, i, vc, go, link }) {
  return (
    <div onClick={() => link && go(link)}
      style={{ background: 'var(--ink-2)', padding: '24px 20px', position: 'relative', overflow: 'hidden', transition: 'background .25s', cursor: link ? 'pointer' : 'default' }}
      onMouseEnter={e => e.currentTarget.style.background = 'var(--ink-3)'}
      onMouseLeave={e => e.currentTarget.style.background = 'var(--ink-2)'}>
      <div style={{ fontFamily: 'var(--mono)', fontSize: 9, letterSpacing: '.12em', textTransform: 'uppercase', color: 'var(--muted)', marginBottom: 12 }}>{l}</div>
      <div style={{ fontFamily: 'var(--display)', fontSize: 44, letterSpacing: '.02em', lineHeight: 1, color: vc || 'var(--paper)' }}>{v}</div>
      <div style={{ fontFamily: 'var(--mono)', fontSize: 9, color: 'var(--accent)', letterSpacing: '.06em', marginTop: 8 }}>{d}</div>
      <div style={{ position: 'absolute', top: 16, right: 16, fontSize: 18, opacity: .22 }}>{i}</div>
      {link && <div style={{ position: 'absolute', bottom: 8, right: 12, fontFamily: 'var(--mono)', fontSize: 8, color: 'var(--muted2)', letterSpacing: '.06em' }}>→</div>}
    </div>
  );
}

function QuickLink({ ico, label, sub, onClick }) {
  return (
    <button onClick={onClick}
      style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '14px 16px', background: 'var(--ink-3)', border: '1px solid var(--line)', width: '100%', cursor: 'pointer', textAlign: 'left', transition: 'background .15s' }}
      onMouseEnter={e => e.currentTarget.style.background = 'var(--ink-4)'}
      onMouseLeave={e => e.currentTarget.style.background = 'var(--ink-3)'}>
      <span style={{ fontSize: 18, width: 24, flexShrink: 0 }}>{ico}</span>
      <div>
        <div style={{ fontFamily: 'var(--body)', fontSize: 13, color: 'var(--paper)', fontWeight: 500 }}>{label}</div>
        <div style={{ fontFamily: 'var(--mono)', fontSize: 9, color: 'var(--muted)', letterSpacing: '.04em', marginTop: 2 }}>{sub}</div>
      </div>
      <span style={{ marginLeft: 'auto', fontFamily: 'var(--mono)', fontSize: 10, color: 'var(--muted2)' }}>→</span>
    </button>
  );
}

function ActivityItem({ ico, t, title, meta, time, go, link }) {
  const icoStyle = {
    s: { borderColor: 'rgba(74,222,128,.3)',  background: 'var(--s-success)' },
    w: { borderColor: 'rgba(251,191,36,.3)',  background: 'var(--s-warning)' },
    i: { borderColor: 'rgba(200,245,90,.2)',  background: 'var(--accent-dim)' },
  }[t] || {};
  return (
    <div onClick={() => link && go(link)}
      style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '14px 20px', borderBottom: '1px solid var(--line)', background: 'var(--ink-2)', transition: 'background .15s', cursor: link ? 'pointer' : 'default' }}
      onMouseEnter={e => e.currentTarget.style.background = 'var(--ink-3)'}
      onMouseLeave={e => e.currentTarget.style.background = 'var(--ink-2)'}>
      <div style={{ width: 32, height: 32, flexShrink: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 13, border: '1px solid var(--line)', ...icoStyle }}>{ico}</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 13, fontWeight: 500 }}>{title}</div>
        <div style={{ fontFamily: 'var(--mono)', fontSize: 10, color: 'var(--muted)', marginTop: 2, letterSpacing: '.04em', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{meta}</div>
      </div>
      <div style={{ fontFamily: 'var(--mono)', fontSize: 10, color: 'var(--muted2)', whiteSpace: 'nowrap', flexShrink: 0 }}>{time}</div>
    </div>
  );
}

export default function AdminDashboard({ go, toast }) {
  const [loading, setLoading] = useState(true);
  const [stats, setStats] = useState(MOCK_ADMIN_STATS);
  const [activity, setActivity] = useState(MOCK_ADMIN_ACTIVITY);

  useEffect(() => {
    const tok = getAdminToken();
    if (!tok) { setLoading(false); return; }
    const load = async () => {
      try {
        const [sData, aData] = await Promise.allSettled([
          fetch('/admin/dashboard/stats',    { headers: { Authorization: `Bearer ${tok}` } }).then(r => r.ok ? r.json() : null),
          fetch('/admin/dashboard/activity', { headers: { Authorization: `Bearer ${tok}` } }).then(r => r.ok ? r.json() : null),
        ]);
        if (sData.status === 'fulfilled' && sData.value && 'totalUsers' in sData.value) setStats(sData.value);
        if (aData.status === 'fulfilled' && Array.isArray(aData.value) && aData.value.length > 0) setActivity(aData.value);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  const statCards = [
    { l: 'Total Users',       v: String(stats.totalUsers),        d: '↑ 12 this month', i: '👤', link: 'rbac' },
    { l: 'Active Sessions',   v: String(stats.activeSessions),    d: 'Across all devices', i: '👁', link: 'sessions' },
    { l: 'Pending Recovery',  v: String(stats.pendingRecoveries), d: stats.pendingRecoveries > 0 ? 'Needs review' : 'All clear', i: '🔑', vc: stats.pendingRecoveries > 0 ? 'var(--warning)' : 'var(--success)', link: 'recovery' },
    { l: 'Threat Alerts',     v: String(stats.threatAlerts),      d: stats.threatAlerts > 0 ? 'Investigate now' : 'No alerts', i: '🚨', vc: stats.threatAlerts > 0 ? 'var(--danger)' : 'var(--success)', link: 'risk' },
  ];

  const quickLinks = [
    { ico: '📋', label: 'Audit Logs',    sub: 'Review all auth events',          id: 'auditlogs' },
    { ico: '👥', label: 'Users & Roles', sub: 'Manage RBAC assignments',          id: 'rbac' },
    { ico: '👁', label: 'Sessions',      sub: 'View and revoke active sessions',  id: 'sessions' },
    { ico: '🔑', label: 'Recovery',      sub: 'Approve recovery requests',        id: 'recovery' },
    { ico: '⚡', label: 'Policy Engine', sub: 'Configure auth policies',          id: 'policy' },
    { ico: '🌐', label: 'Risk Intel',    sub: 'Threat signals and anomalies',     id: 'risk' },
  ];

  return (
    <AppShell active="admin" go={go}>
      <div style={{ padding: '36px 44px 28px', display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 16, borderBottom: '1px solid var(--line)' }} className="page-header">
        <div>
          <div style={{ fontFamily: 'var(--display)', fontSize: 36, letterSpacing: '.06em', textTransform: 'uppercase' }}>Admin Dashboard</div>
          <div style={{ fontFamily: 'var(--mono)', fontSize: 9, letterSpacing: '.1em', textTransform: 'uppercase', color: 'var(--muted)', marginTop: 6 }}>Operator overview</div>
        </div>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          <BtnF onClick={() => go('auditlogs')} style={{ padding: '8px 16px', fontSize: 9 }}>View Audit Logs →</BtnF>
          <BtnO onClick={() => go('dashboard')} style={{ padding: '8px 16px', fontSize: 9 }}>User Dashboard</BtnO>
        </div>
      </div>

      {loading ? (
        <AdminSkeleton />
      ) : (
        <div style={{ padding: '36px 44px 60px', flex: 1 }} className="page-body">
          {/* Stat cards */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 1, background: 'var(--line)', marginBottom: 28, border: '1px solid var(--line)' }} className="stat-grid">
            {statCards.map((s, i) => <StatCard key={i} {...s} go={go} />)}
          </div>

          {/* Quick links + recent activity */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 1, background: 'var(--line)', marginBottom: 28, border: '1px solid var(--line)' }}>
            <div style={{ background: 'var(--ink-2)', padding: '28px 24px' }}>
              <div style={{ fontFamily: 'var(--mono)', fontSize: 9, letterSpacing: '.12em', textTransform: 'uppercase', color: 'var(--muted)', marginBottom: 16 }}>Quick Access</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                {quickLinks.map(ql => (
                  <QuickLink key={ql.id} ico={ql.ico} label={ql.label} sub={ql.sub} onClick={() => go(ql.id)} />
                ))}
              </div>
            </div>
            <div style={{ background: 'var(--ink-2)', padding: '28px 24px' }}>
              <div style={{ fontFamily: 'var(--mono)', fontSize: 9, letterSpacing: '.12em', textTransform: 'uppercase', color: 'var(--muted)', marginBottom: 16 }}>System Status</div>
              {[
                { l: 'Auth Service',  v: 'Operational',  c: 'var(--success)' },
                { l: 'Gateway',       v: 'Operational',  c: 'var(--success)' },
                { l: 'Database',      v: 'Healthy',       c: 'var(--success)' },
                { l: 'Redis Cache',   v: 'Healthy',       c: 'var(--success)' },
                { l: 'Incident',      v: 'None Active',   c: 'var(--muted)' },
              ].map(({ l, v, c }) => (
                <div key={l} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '10px 0', borderBottom: '1px solid var(--line)' }}>
                  <span style={{ fontFamily: 'var(--mono)', fontSize: 10, color: 'var(--muted)', letterSpacing: '.04em' }}>{l}</span>
                  <span style={{ fontFamily: 'var(--mono)', fontSize: 10, color: c, letterSpacing: '.04em' }}>{v}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Recent admin activity */}
          <div style={{ fontFamily: 'var(--mono)', fontSize: 10, letterSpacing: '.12em', textTransform: 'uppercase', color: 'var(--muted)', marginBottom: 16, display: 'flex', alignItems: 'center', gap: 14 }}>
            Recent Admin Events
            <div style={{ flex: 1, height: 1, background: 'var(--line)' }} />
            <button onClick={() => go('auditlogs')} style={{ fontFamily: 'var(--mono)', fontSize: 9, color: 'var(--accent)', background: 'none', border: 'none', cursor: 'pointer', letterSpacing: '.08em' }}>View all →</button>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', border: '1px solid var(--line)' }}>
            {activity.map(a => <ActivityItem key={a.id} {...a} t={a.type} go={go} />)}
          </div>
        </div>
      )}
    </AppShell>
  );
}
