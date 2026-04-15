import { useState, useEffect, useRef } from 'react';
import AppShell from './AppShell';

function NetworkCanvas() {
  const ref = useRef(null);
  useEffect(() => {
    const canvas = ref.current; if (!canvas) return;
    const ctx = canvas.getContext("2d");
    canvas.width = canvas.offsetWidth; canvas.height = 220;
    const W = canvas.width, H = canvas.height;
    const nodes = [
      { x: W * .18, y: H * .5,  label: "MacBook Pro",  c: "#4ADE80" },
      { x: W * .42, y: H * .2,  label: "iPhone 15",    c: "#4ADE80" },
      { x: W * .75, y: H * .45, label: "iPad Air",     c: "#4ADE80" },
      { x: W * .5,  y: H * .72, label: "Server",       c: "#C8F55A" },
      { x: W * .3,  y: H * .38, label: "Work Desktop", c: "#5A5550" },
    ];
    const edges = [[0, 3], [1, 3], [2, 3], [4, 3], [0, 1]];
    let t = 0, raf;
    const draw = () => {
      ctx.clearRect(0, 0, W, H);
      edges.forEach(([a, b]) => {
        ctx.beginPath(); ctx.moveTo(nodes[a].x, nodes[a].y); ctx.lineTo(nodes[b].x, nodes[b].y);
        ctx.strokeStyle = "rgba(244,241,236,0.06)"; ctx.lineWidth = 1; ctx.stroke();
        const p = (Math.sin(t * .025 + a) + 1) / 2;
        const px = nodes[a].x + (nodes[b].x - nodes[a].x) * p, py = nodes[a].y + (nodes[b].y - nodes[a].y) * p;
        ctx.beginPath(); ctx.arc(px, py, 2.5, 0, Math.PI * 2); ctx.fillStyle = "rgba(200,245,90,.7)"; ctx.fill();
      });
      nodes.forEach(n => {
        const g = ctx.createRadialGradient(n.x, n.y, 0, n.x, n.y, 22);
        g.addColorStop(0, n.c + "30"); g.addColorStop(1, "transparent");
        ctx.beginPath(); ctx.arc(n.x, n.y, 22, 0, Math.PI * 2); ctx.fillStyle = g; ctx.fill();
        ctx.beginPath(); ctx.arc(n.x, n.y, 6, 0, Math.PI * 2); ctx.fillStyle = n.c; ctx.fill();
        ctx.fillStyle = "rgba(244,241,236,.5)"; ctx.font = "10px DM Mono, monospace"; ctx.textAlign = "center";
        ctx.fillText(n.label, n.x, n.y + 22);
      });
      t++; raf = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, []);
  return <canvas ref={ref} style={{ display: "block", width: "100%", height: 220 }} height={220} />;
}

function AdminCard({ ico, t, b, link, go, toast }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)}
      onClick={() => link ? go(link) : toast(t, "info")}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", position: "relative", overflow: "hidden" }}>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ fontSize: 28, marginBottom: 18 }}>{ico}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>{t}</div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{b}</div>
      {link && <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", marginTop: 14, letterSpacing: ".08em" }}>Open →</div>}
    </div>
  );
}

export default function Admin({ go, toast }) {
  return (
    <AppShell active="admin" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Admin</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>System management · Analytics</div>
      </div>
      <div className="page-body" style={{ padding: "36px 44px 60px" }}>
        <div className="pg-in" style={{ border: "1px solid var(--line)", background: "var(--ink-2)", marginBottom: 28, overflow: "hidden" }}>
          <div style={{ padding: "18px 22px", borderBottom: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)" }}>Network Topology — Live Trust Graph</span>
            <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--success)", letterSpacing: ".06em" }}>● 3 nodes active</span>
          </div>
          <NetworkCanvas />
        </div>
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 28 }}>
          {[
            { ico: "📜", t: "Policy Editor",   b: "Visual rule builder for access policies. Configure deny-by-default and step-up verification rules.", link: "policy" },
            { ico: "👥", t: "User Directory",  b: "Searchable user list with device counts, last active timestamps, and trust scores.", link: "rbac" },
            { ico: "🗂", t: "Audit Log",       b: "Full cryptographic event stream. Export to CSV or JSON for compliance reporting.", link: "auditlogs" },
            { ico: "📊", t: "System Health",   b: "Uptime monitoring, response times, WebAuthn success rates, error dashboards.", link: "risk" },
          ].map(c => <AdminCard key={c.t} {...c} go={go} toast={toast} />)}
        </div>
        <div className="pg-in" style={{ border: "1px solid rgba(248,113,113,.15)", background: "var(--ink-2)", padding: 28, cursor: "pointer" }}
          onClick={() => toast("Emergency lockdown requires multi-device confirmation", "danger")}>
          <div style={{ fontSize: 28, marginBottom: 18 }}>🚨</div>
          <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>Danger Zone</div>
          <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>Global device revocation and emergency system lockdown. Requires multi-device cryptographic confirmation.</div>
        </div>
      </div>
    </AppShell>
  );
}
