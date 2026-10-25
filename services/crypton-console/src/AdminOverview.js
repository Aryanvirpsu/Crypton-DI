import { useState, useEffect, useCallback } from 'react';
import { crypton, CryptonError } from './sdk';
import { BtnF } from './Buttons';
import AppShell from './AppShell';

/**
 * Admin dashboard. If the backend 403s (no role yet, or role isn't
 * admin/super_admin), this shows a real "access required" state with a
 * bootstrap affordance rather than pretending to be a normal empty page.
 * Bootstrap only succeeds while the deployment has zero admins — see
 * crypton-id/src/routes/admin.rs::admin_bootstrap.
 */
export default function AdminOverview({ go, toast, isAdmin }) {
  const [loading, setLoading] = useState(true);
  const [forbidden, setForbidden] = useState(false);
  const [stats, setStats] = useState(null);
  const [activity, setActivity] = useState(null);
  const [bootstrapping, setBootstrapping] = useState(false);

  const load = useCallback(() => {
    setLoading(true);
    setForbidden(false);
    Promise.all([crypton.admin.stats(), crypton.admin.activity()])
      .then(([s, a]) => { setStats(s); setActivity(a); })
      .catch(err => {
        if (CryptonError.is(err) && err.code === 'admin_authorization_required') {
          setForbidden(true);
        } else {
          toast(`Failed to load admin dashboard: ${err.message}`, 'danger');
        }
      })
      .finally(() => setLoading(false));
  }, [toast]);

  useEffect(() => { load(); }, [load]);

  const bootstrap = async () => {
    setBootstrapping(true);
    try {
      await crypton.admin.bootstrap();
      toast('You are now an admin', 'success');
      window.location.reload(); // simplest way to refresh /me + nav state
    } catch (err) {
      toast(`Bootstrap failed: ${err.message}`, 'danger');
    } finally {
      setBootstrapping(false);
    }
  };

  return (
    <AppShell active="admin" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Admin Overview</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Organization-wide stats</div>
      </div>

      <div className="page-body" style={{ padding: "36px 44px 60px" }}>
        {loading ? (
          <div style={{ textAlign: "center", padding: "60px 0", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>Loading...</div>
        ) : forbidden ? (
          <div style={{ border: "1px solid var(--line)", padding: "60px", textAlign: "center", background: "var(--ink-2)" }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>🔒</div>
            <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>Admin Access Required</div>
            <div style={{ fontSize: 13, color: "var(--muted)", marginBottom: 24, maxWidth: 440, marginLeft: "auto", marginRight: "auto" }}>
              {isAdmin
                ? "Your account has an admin role but the request was still refused — check the server logs."
                : "Your account doesn't have admin access. If this is a fresh deployment with no admins yet, you can claim it below — this only works once."}
            </div>
            {!isAdmin && (
              <BtnF onClick={bootstrap} style={{ opacity: bootstrapping ? 0.5 : 1, pointerEvents: bootstrapping ? "none" : "auto" }}>
                {bootstrapping ? "Claiming..." : "Become the First Admin →"}
              </BtnF>
            )}
          </div>
        ) : (
          <>
            <div className="pg-in stat-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", marginBottom: 28, border: "1px solid var(--line)" }}>
              {[
                { l: "Total Users", v: stats.totalUsers },
                { l: "Active Credentials", v: stats.activeCredentials },
                { l: "Lost Credentials", v: stats.lostCredentials },
                { l: "Revoked Credentials", v: stats.revokedCredentials },
                { l: "Auth Events (24h)", v: stats.authEvents24h },
                { l: "Failed Auths (24h)", v: stats.failedAuths24h },
              ].map(s => (
                <div key={s.l} style={{ background: "var(--ink-2)", padding: "28px 24px" }}>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 14 }}>{s.l}</div>
                  <div style={{ fontFamily: "var(--display)", fontSize: 44, letterSpacing: ".02em", lineHeight: 1 }}>{s.v}</div>
                </div>
              ))}
            </div>

            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 16 }}>Recent Activity (all users)</div>
            {activity.length === 0 ? (
              <div style={{ border: "1px solid var(--line)", padding: "40px", textAlign: "center", color: "var(--muted)", fontSize: 13 }}>No activity yet.</div>
            ) : (
              <div style={{ border: "1px solid var(--line)" }}>
                {activity.map(a => (
                  <div key={a.id} style={{ display: "flex", alignItems: "center", gap: 16, padding: "14px 18px", borderBottom: "1px solid var(--line)" }}>
                    <span style={{ fontSize: 14 }}>{a.ico}</span>
                    <span style={{ flex: 1, fontSize: 13 }}>{a.title} — <span style={{ color: "var(--muted)" }}>{a.meta}</span></span>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{a.time}</span>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </AppShell>
  );
}
