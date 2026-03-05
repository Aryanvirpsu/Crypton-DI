import { useState, useEffect, useRef, useCallback } from "react";

// ── MOCK DATA (replace with real API calls when backend endpoints exist) ──────
const MOCK_AUDIT_LOGS = [
  { id: "evt_001", actor: "aryan@crypton.io", action: "LOGIN", device: "MacBook Pro", ip: "192.168.1.1", loc: "San Francisco, CA", time: "2m ago", type: "success" },
  { id: "evt_002", actor: "aryan@crypton.io", action: "DEVICE_ENROLL", device: "iPhone 15 Pro", ip: "192.168.1.1", loc: "San Francisco, CA", time: "1h ago", type: "info" },
  { id: "evt_003", actor: "admin@crypton.io", action: "ROLE_CHANGE", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "3h ago", type: "warning" },
  { id: "evt_004", actor: "aryan@crypton.io", action: "LOGIN_BLOCKED", device: "Unknown", ip: "185.220.101.4", loc: "Tokyo, JP", time: "6h ago", type: "danger" },
  { id: "evt_005", actor: "admin@crypton.io", action: "POLICY_UPDATE", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "8h ago", type: "info" },
  { id: "evt_006", actor: "aryan@crypton.io", action: "DEVICE_REVOKE", device: "Work Desktop", ip: "192.168.1.1", loc: "San Francisco, CA", time: "1d ago", type: "danger" },
  { id: "evt_007", actor: "sarah@crypton.io", action: "LOGIN", device: "iPad Air", ip: "74.125.24.100", loc: "Austin, TX", time: "1d ago", type: "success" },
  { id: "evt_008", actor: "admin@crypton.io", action: "PASSKEY_REVOKE", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "2d ago", type: "danger" },
  { id: "evt_009", actor: "sarah@crypton.io", action: "LOGIN", device: "iPhone 14", ip: "74.125.24.100", loc: "Austin, TX", time: "2d ago", type: "success" },
  { id: "evt_010", actor: "aryan@crypton.io", action: "LOGIN", device: "MacBook Pro", ip: "192.168.1.1", loc: "San Francisco, CA", time: "3d ago", type: "success" },
];
const MOCK_RISK_USERS = [
  { id: "usr_001", user: "aryan@crypton.io", score: 82, level: "HIGH", device: "Unknown Device", ip: "185.220.101.4", loc: "Tokyo, JP", time: "02:14 AM", reasons: ["New device detected", "Foreign IP address", "Unusual login time", "Geo-velocity anomaly"] },
  { id: "usr_002", user: "sarah@crypton.io", score: 34, level: "LOW", device: "iPhone 14", ip: "74.125.24.100", loc: "Austin, TX", time: "09:32 AM", reasons: ["Known device", "Familiar location"] },
  { id: "usr_003", user: "admin@crypton.io", score: 61, level: "MEDIUM", device: "MacBook Pro", ip: "10.0.0.5", loc: "New York, NY", time: "11:58 PM", reasons: ["Unusual login time", "Multiple failed attempts"] },
];
const MOCK_RISK_FEED = [
  { id: "feed_001", ico: "🚨", type: "danger", msg: "Geo-velocity alert: 8,400km in 3 hours", time: "2m ago" },
  { id: "feed_002", ico: "⚠", type: "warning", msg: "TOR exit node detected — IP 185.220.101.4", time: "14m ago" },
  { id: "feed_003", ico: "⚠", type: "warning", msg: "3 failed passkey attempts — admin@crypton.io", time: "1h ago" },
  { id: "feed_004", ico: "🛡", type: "info", msg: "Device reputation verified — MacBook Pro", time: "2h ago" },
  { id: "feed_005", ico: "✓", type: "success", msg: "Behavioral baseline updated — sarah@crypton.io", time: "4h ago" },
];
const MOCK_SESSIONS = [
  { id: "ses_001", user: "aryan@crypton.io", device: "MacBook Pro", browser: "Chrome 122", loc: "San Francisco, CA", ip: "192.168.1.1", started: "Today, 09:14 AM", duration: "4h 32m", active: true },
  { id: "ses_002", user: "aryan@crypton.io", device: "iPhone 15 Pro", browser: "Safari Mobile", loc: "San Francisco, CA", ip: "192.168.1.2", started: "Today, 11:02 AM", duration: "2h 44m", active: true },
  { id: "ses_003", user: "sarah@crypton.io", device: "iPad Air", browser: "Safari", loc: "Austin, TX", ip: "74.125.24.100", started: "Today, 08:30 AM", duration: "5h 16m", active: true },
  { id: "ses_004", user: "admin@crypton.io", device: "MacBook Pro", browser: "Firefox 123", loc: "New York, NY", ip: "10.0.0.5", started: "Yesterday, 11:58 PM", duration: "Idle 8h", active: false },
];
const MOCK_USERS = [
  { id: "usr_001", name: "Aryan Vir", email: "aryan@crypton.io", role: "Super Admin", devices: 2, lastActive: "2m ago", avatar: "A" },
  { id: "usr_002", name: "Admin User", email: "admin@crypton.io", role: "Admin", devices: 1, lastActive: "1h ago", avatar: "AU" },
  { id: "usr_003", name: "Sarah Kim", email: "sarah@crypton.io", role: "Security Analyst", devices: 2, lastActive: "3h ago", avatar: "S" },
  { id: "usr_004", name: "Dev Read", email: "dev@crypton.io", role: "Viewer", devices: 1, lastActive: "2d ago", avatar: "D" },
];
const MOCK_DASHBOARD_STATS = { activeDevices: 3, authEvents24h: 47, securityScore: 98 };
const MOCK_ACTIVITY = [
  { id: "act_001", ico: "✓", type: "s", title: "Authentication successful", meta: "MacBook Pro · Chrome · San Francisco, CA", time: "2m ago", link: "auditlogs" },
  { id: "act_002", ico: "📱", type: "i", title: "New device enrolled", meta: "iPhone 15 Pro · Passkey created", time: "1h ago", link: "devices" },
  { id: "act_003", ico: "✓", type: "s", title: "Authentication successful", meta: "iPad Air · Safari · New York, NY", time: "3h ago", link: "auditlogs" },
  { id: "act_004", ico: "⚠", type: "w", title: "Unrecognized device blocked", meta: "Unknown · Tokyo, JP · Request denied", time: "6h ago", link: "risk" },
  { id: "act_005", ico: "🔒", type: "i", title: "Security sweep completed", meta: "All 3 devices verified · Zero anomalies", time: "12h ago", link: "sessions" },
];
const MOCK_ORG = { orgName: "Crypton Labs", domain: "crypton.io", domainVerified: true, mfaEnforced: true, sessionTimeoutHours: 8, allowedCountries: ["US", "CA", "GB", "DE", "AU"] };
const MOCK_POLICIES = [
  { id: "geo_block", label: "Block High-Risk Countries", desc: "Deny auth from CN, RU, KP, IR and other flagged regions", active: true, cat: "geo" },
  { id: "stepup_risk", label: "Step-Up Auth if Risk > 70", desc: "Require additional verification when risk score exceeds threshold", active: true, cat: "risk" },
  { id: "tor_block", label: "Block TOR / VPN IPs", desc: "Reject requests from known TOR exit nodes and datacenter IPs", active: true, cat: "network" },
  { id: "geo_velocity", label: "Geo-Velocity Protection", desc: "Block impossible travel — flag logins from multiple continents within 4h", active: false, cat: "geo" },
  { id: "device_trust", label: "Device Trust Duration — 30 days", desc: "Re-verify device passkey after 30 days of inactivity", active: true, cat: "device" },
  { id: "failed_attempts", label: "Lock After 5 Failed Attempts", desc: "Temporary 15-minute lockout after 5 consecutive failed auth attempts", active: true, cat: "auth" },
  { id: "off_hours", label: "Notify on Off-Hours Login", desc: "Send alert when users authenticate outside 06:00–22:00 local time", active: false, cat: "auth" },
  { id: "new_device", label: "Require Approval for New Devices", desc: "Admin must approve new device enrollment via existing trusted device", active: false, cat: "device" },
];

/* ─── FONTS ─── */
const FontLink = () => (
  <style>{`
    @import url('https://fonts.googleapis.com/css2?family=Bebas+Neue&family=DM+Serif+Display:ital@0;1&family=DM+Mono:wght@400;500&family=Geist:wght@300;400;500&display=swap');
    :root {
      --ink:#0A0A0A;--ink-2:#111111;--ink-3:#161616;--ink-4:#1C1C1C;
      --paper:#F4F1EC;--off:#E8E4DC;--muted:#7A7570;--muted2:#5A5550;
      --line:rgba(244,241,236,0.07);--line2:rgba(244,241,236,0.13);
      --accent:#C8F55A;--accent-dim:rgba(200,245,90,0.1);
      --success:#4ADE80;--warning:#FBBF24;--danger:#F87171;
      --s-success:rgba(74,222,128,0.1);--s-warning:rgba(251,191,36,0.1);--s-danger:rgba(248,113,113,0.1);
      --serif:'DM Serif Display',serif;
      --display:'Bebas Neue',sans-serif;
      --mono:'DM Mono',monospace;
      --body:'Geist',sans-serif;
    }
    *,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
    html{scroll-behavior:smooth}
    body{font-family:var(--body);background:var(--ink);color:var(--paper);overflow-x:hidden;line-height:1.5;cursor:auto}
    ::selection{background:var(--accent);color:var(--ink)}
    ::-webkit-scrollbar{width:2px}::-webkit-scrollbar-track{background:var(--ink)}::-webkit-scrollbar-thumb{background:var(--accent)}

    .grain{position:fixed;inset:0;z-index:9000;pointer-events:none;background-image:url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");opacity:.035}

    /* page transition */
    .pg-in{animation:pgIn .4s cubic-bezier(.16,1,.3,1)}
    @keyframes pgIn{from{opacity:0;transform:translateY(14px)}to{opacity:1;transform:translateY(0)}}

    /* landing hero */
    @keyframes up{to{transform:translateY(0)}}
    .hero-line{transform:translateY(110%);animation:up 1s cubic-bezier(.16,1,.3,1) forwards}
    .hero-line-2{animation-delay:.07s}
    @keyframes sp{0%{transform:translateY(-100%)}100%{transform:translateY(200%)}}
    @keyframes tick{from{transform:translateX(0)}to{transform:translateX(-50%)}}
    .ticker-track{display:inline-flex;animation:tick 28s linear infinite}
    @keyframes fg{0%,100%{transform:translateY(0) rotate(-3deg)}50%{transform:translateY(-14px) rotate(3deg)}}
    .fv-glyph{animation:fg 7s ease-in-out infinite}

    /* landing nav */
    .landing-nav{position:fixed;top:0;left:0;right:0;z-index:1000;display:flex;align-items:center;justify-content:space-between;padding:24px 52px;transition:background .3s,backdrop-filter .3s,padding .3s,border-color .3s;border-bottom:1px solid transparent}
    .landing-nav.scrolled{background:rgba(10,10,10,0.88);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);padding:16px 52px;border-color:var(--line)}

    /* orb */
    @keyframes orbPulse{0%,100%{box-shadow:0 0 20px rgba(74,222,128,.2),0 0 60px rgba(74,222,128,.08);transform:scale(1)}50%{box-shadow:0 0 40px rgba(74,222,128,.35),0 0 80px rgba(74,222,128,.12);transform:scale(1.04)}}
    @keyframes orbWarn{0%,100%{box-shadow:0 0 20px rgba(251,191,36,.25);transform:scale(1)}50%{box-shadow:0 0 50px rgba(251,191,36,.5);transform:scale(1.06)}}
    @keyframes orbDanger{0%,100%{box-shadow:0 0 30px rgba(248,113,113,.4);transform:scale(1)}50%{box-shadow:0 0 60px rgba(248,113,113,.7);transform:scale(1.08)}}
    @keyframes rExpand{0%{opacity:.6}100%{opacity:0;transform:scale(1.1)}}
    .orb-pulse{position:absolute;border-radius:50%;border:1px solid rgba(74,222,128,.15)}
    .orb-pulse.r1{width:120px;height:120px;animation:rExpand 3.5s ease-out infinite;animation-delay:1.2s}
    .orb-pulse.r2{width:140px;height:140px;animation:rExpand 3.5s ease-out infinite;animation-delay:2.4s}

    /* reveal */
    .rv{opacity:0;transform:translateY(36px);transition:opacity .75s ease,transform .75s cubic-bezier(.16,1,.3,1)}
    .rv.in{opacity:1;transform:translateY(0)}
    .rv-1{transition-delay:.1s}.rv-2{transition-delay:.2s}

    /* fi */
    .fi{opacity:0;transform:translateY(20px);transition:opacity .5s ease,transform .5s cubic-bezier(.16,1,.3,1)}
    .fi.in{opacity:1;transform:translateY(0)}

    /* vault */
    @keyframes vFloat{0%,100%{transform:translateY(0) rotateX(5deg)}50%{transform:translateY(-10px) rotateX(5deg)}}

    /* modal anim */
    @keyframes mIn{from{opacity:0;transform:scale(.96) translateY(18px)}to{opacity:1;transform:scale(1) translateY(0)}}
    .modal-anim{animation:mIn .4s cubic-bezier(.16,1,.3,1)}

    /* toast */
    @keyframes tIn{from{transform:translateX(100%);opacity:0}to{transform:translateX(0);opacity:1}}
    @keyframes tOut{to{transform:translateX(100%);opacity:0}}
    .toast-in{animation:tIn .4s cubic-bezier(.16,1,.3,1)}
    .toast-out{animation:tOut .3s ease forwards}

    /* register vis */
    .rv-device{width:140px;height:240px;background:linear-gradient(160deg,var(--ink-3),#060606);border:1px solid rgba(200,245,90,.2);display:flex;align-items:center;justify-content:center;font-size:52px;position:relative;transform:perspective(500px) rotateY(-12deg) rotateX(8deg);transition:transform .7s cubic-bezier(.16,1,.3,1),box-shadow .5s;box-shadow:0 40px 80px rgba(0,0,0,.7)}
    .rv-device.s2{transform:perspective(500px) rotateY(0deg) rotateX(5deg)}
    .rv-device.s3{transform:perspective(500px) rotateY(12deg) rotateX(3deg)}
    .rv-key{position:absolute;top:16%;right:-12px;font-size:24px;opacity:0;transform:scale(.5) translateY(10px);transition:all .5s cubic-bezier(.16,1,.3,1)}
    .rv-device.s2 .rv-key,.rv-device.s3 .rv-key{opacity:1;transform:scale(1) translateY(0)}
    .rv-shield{position:absolute;bottom:10%;font-size:44px;opacity:0;transform:scale(.5);transition:all .6s cubic-bezier(.16,1,.3,1)}
    .rv-device.s3 .rv-shield{opacity:1;transform:scale(1)}
    .rv-glow{position:absolute;inset:-20px;opacity:0;transition:opacity .5s}
    .rv-device.s2 .rv-glow{background:radial-gradient(ellipse,rgba(200,245,90,.12),transparent);opacity:1}
    .rv-device.s3 .rv-glow{background:radial-gradient(ellipse,rgba(74,222,128,.15),transparent);opacity:1}

    /* dcard 3d */
    .dcard:hover .dmodel{transform:rotateY(8deg) rotateX(-4deg) scale(1.05)}
    .dmodel{transition:transform .3s}

    /* feat item */
    .feat-item{transition:padding-left .3s}
    .feat-item:hover{padding-left:10px}

    /* stat hover */
    .stat-c{position:relative;overflow:hidden}
    .stat-c::before{content:'';position:absolute;top:0;left:0;right:0;height:2px;background:var(--accent);transform:scaleX(0);transform-origin:left;transition:transform .5s cubic-bezier(.16,1,.3,1)}
    .stat-c:hover::before{transform:scaleX(1)}

    /* ── MOBILE NAV HAMBURGER ── */
    .mob-menu-btn{display:none;flex-direction:column;gap:5px;background:none;border:none;cursor:pointer;padding:8px;z-index:1100}
    .mob-menu-btn span{display:block;width:22px;height:1.5px;background:var(--paper);transition:all .3s}
    .mob-menu-btn.open span:nth-child(1){transform:translateY(6.5px) rotate(45deg)}
    .mob-menu-btn.open span:nth-child(2){opacity:0}
    .mob-menu-btn.open span:nth-child(3){transform:translateY(-6.5px) rotate(-45deg)}
    .mob-drawer{position:fixed;inset:0;top:0;background:rgba(10,10,10,0.97);backdrop-filter:blur(20px);z-index:1050;display:flex;flex-direction:column;padding:100px 32px 40px;transform:translateX(100%);transition:transform .4s cubic-bezier(.16,1,.3,1)}
    .mob-drawer.open{transform:translateX(0)}
    .mob-drawer a,.mob-drawer button.mob-link{font-family:var(--display);font-size:clamp(36px,10vw,52px);letter-spacing:.06em;text-transform:uppercase;color:var(--paper);text-decoration:none;background:none;border:none;cursor:pointer;display:block;padding:10px 0;border-bottom:1px solid var(--line);text-align:left;transition:color .2s}
    .mob-drawer-ctas{display:flex;flex-direction:column;gap:12px;margin-top:32px}

    /* ── BOTTOM TAB BAR (mobile app shell) ── */
    .bottom-tabs{display:none;position:fixed;bottom:0;left:0;right:0;z-index:800;background:var(--ink-2);border-top:1px solid var(--line);padding:8px 0 max(8px,env(safe-area-inset-bottom))}
    .bottom-tabs-inner{display:flex;justify-content:space-around;align-items:center}
    .tab-btn{display:flex;flex-direction:column;align-items:center;gap:3px;background:none;border:none;cursor:pointer;padding:4px 8px;min-width:52px;color:var(--muted);transition:color .15s}
    .tab-btn.active{color:var(--accent)}
    .tab-btn span:first-child{font-size:18px;line-height:1}
    .tab-btn span:last-child{font-family:var(--mono);font-size:8px;letter-spacing:.06em;text-transform:uppercase}

    @media(max-width:1023px){
      .nav-links-wrap{display:none}
      .landing-nav{padding:20px 24px!important}
      .landing-nav.scrolled{padding:14px 24px!important}
      .hero-pad,.section-pad{padding-left:24px!important;padding-right:24px!important}
      .sidebar{width:56px}
      .sb-mark,.si-label,.sb-label-txt,.user-name,.user-role{display:none}
      .si{justify-content:center}
      .si-badge{display:none}
    }
    @media(max-width:767px){
      .mob-menu-btn{display:flex}
      .landing-nav .nav-desktop-btns{display:none}
      .sidebar{display:none!important}
      .bottom-tabs{display:block}
      .app-main{padding-bottom:72px!important}
      .hero-line{font-size:clamp(58px,17vw,110px)!important}
      .manifesto-grid{grid-template-columns:1fr!important;gap:40px!important}
      .protocol-grid{grid-template-columns:1fr!important}
      .features-grid{grid-template-columns:1fr!important}
      .features-sticky{display:none!important}
      .features-list{padding-right:0!important;border-right:none!important}
      .stats-grid{grid-template-columns:1fr 1fr!important}
      .dev-grid{grid-template-columns:1fr!important}
      .dev-left{padding:40px 0 0!important;border-right:none!important}
      .dev-right{padding:0 0 40px!important}
      .footer-grid{grid-template-columns:1fr 1fr!important;gap:32px!important}
      .footer-brand{grid-column:1/-1}
      .hiw-grid{grid-template-columns:1fr!important}
      .stat-grid{grid-template-columns:1fr!important}
      .orb-grid{grid-template-columns:1fr!important}
      .orb-vis{display:none!important}
      .page-header{padding:20px 20px 16px!important}
      .page-body{padding:20px 20px 80px!important}
      .audit-table{display:none!important}
      .audit-cards{display:flex!important}
      .sessions-table{display:none!important}
      .sessions-cards{display:flex!important}
      .risk-grid{grid-template-columns:1fr!important}
      .rbac-grid{grid-template-columns:1fr!important}
      .register-grid{grid-template-columns:1fr!important}
      .register-vis{display:none!important}
      .register-form{padding:48px 24px!important}
      .breadcrumb-wrap{padding:10px 20px 0!important}
    }
    @media(max-width:400px){
      .hero-line{font-size:clamp(48px,15vw,80px)!important}
      .stats-grid{grid-template-columns:1fr!important}
    }
    @media(hover:none){
      .feat-item:hover{padding-left:0}
      .stat-c:hover::before{transform:scaleX(0)}
    }
    @media(prefers-reduced-motion:reduce){*,*::before,*::after{animation:none!important;transition:none!important}}
  `}</style>
);

/* ─── BASE64URL UTILS ─── */
const b64url = {
  decode(str) {
    const s = str.replace(/-/g, '+').replace(/_/g, '/');
    const pad = s.length % 4;
    const padded = pad ? s + '='.repeat(4 - pad) : s;
    const raw = atob(padded);
    return Uint8Array.from(raw, c => c.charCodeAt(0)).buffer;
  },
  encode(buf) {
    const bytes = new Uint8Array(buf);
    let str = '';
    bytes.forEach(b => { str += String.fromCharCode(b); });
    return btoa(str).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  }
};

/* ─── API ─── */
const API = {
  async registerStart(username, display_name) {
    const r = await fetch('/auth/register/start', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ username, display_name }) });
    if (!r.ok) { const t = await r.text(); throw new Error(t); }
    return r.json();
  },
  async registerFinish(challenge_id, attestation) {
    const r = await fetch('/auth/register/finish', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ challenge_id, attestation }) });
    if (!r.ok) { const t = await r.text(); throw new Error(t); }
    return r.json();
  },
  async loginStart(username) {
    const r = await fetch('/auth/login/start', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ username }) });
    if (!r.ok) { const t = await r.text(); throw new Error(t); }
    return r.json();
  },
  async loginFinish(challenge_id, assertion) {
    const r = await fetch('/auth/login/finish', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ challenge_id, assertion }) });
    if (!r.ok) { const t = await r.text(); throw new Error(t); }
    return r.json();
  },
  async listDevices(token) {
    const r = await fetch('/devices', { headers: { 'Authorization': `Bearer ${token}` } });
    if (!r.ok) { const t = await r.text(); throw new Error(t); }
    return r.json();
  },
  async revokeDevice(token, id) {
    const r = await fetch(`/devices/${id}`, { method: 'DELETE', headers: { 'Authorization': `Bearer ${token}` } });
    if (!r.ok) { const t = await r.text(); throw new Error(t); }
    return r.json();
  }
};

/* ─── TOAST ─── */
let toastId = 0;
function useToasts() {
  const [toasts, setToasts] = useState([]);
  const add = useCallback((msg, type = "info") => {
    const id = ++toastId;
    setToasts(t => [...t, { id, msg, type }]);
    setTimeout(() => setToasts(t => t.map(x => x.id === id ? { ...x, out: true } : x)), 3200);
    setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 3500);
  }, []);
  return [toasts, add];
}

function ToastStack({ toasts }) {
  return (
    <div style={{ position: "fixed", bottom: 24, right: 24, zIndex: 8000, display: "flex", flexDirection: "column", gap: 6 }}>
      {toasts.map(t => (
        <div key={t.id} className={t.out ? "toast-out" : "toast-in"} style={{
          background: "var(--ink-2)", border: "1px solid var(--line)",
          borderLeft: `2px solid ${t.type === "danger" ? "var(--danger)" : t.type === "success" ? "var(--success)" : "var(--accent)"}`,
          padding: "11px 16px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em",
          color: "var(--paper)", minWidth: 220
        }}>// {t.msg}</div>
      ))}
    </div>
  );
}

/* ─── SCROLL REVEAL ─── */
/* ─── EASING UTILS (used by sphere intro) ─── */
const easeOutCubic = t => 1 - Math.pow(1 - t, 3);
const easeInOutCubic = t => t < .5 ? 4*t*t*t : 1 - Math.pow(-2*t+2,3)/2;
const clampT = (v,a,b) => Math.max(a, Math.min(b, v));

/* ─── SPHERE INTRO ─── */
function useSphereIntro() {
  const canvasRef = useRef(null);
  const stateRef = useRef({
    W: 0, H: 0, rotation: 0, rotSpeed: 0.004, sphereAlpha: 0,
    transitionT: 0, phase: "idle", frame: 0, arcs: [], arcTimer: 0, mouse: { x: 0, y: 0 },
  });
  const introRef = useRef(null);
  const heroRef  = useRef(null);
  const navRef   = useRef(null);
  const isMobile = () => window.innerWidth <= 767;
  const globeRef = useRef(null);
  if (!globeRef.current) {
    const dots = [];
    const latStep = isMobile() ? 14 : 10, lonStep = isMobile() ? 14 : 10;
    for (let lat = -80; lat <= 80; lat += latStep)
      for (let lon = 0; lon < 360; lon += lonStep)
        dots.push({ phi:(lat*Math.PI)/180, theta:(lon*Math.PI)/180, size:Math.random()*1.2+0.4, brightness:Math.random()*.5+.5, pulse:Math.random()*Math.PI*2, pulseSpeed:.02+Math.random()*.03 });
    const rings = [
      { tilt:.3,  speed:.007,  angle:0,   r:1.18, opacity:.35, dash:[8,6]  },
      { tilt:-.5, speed:-.005, angle:1.2, r:1.28, opacity:.25, dash:[4,10] },
      { tilt:.9,  speed:.009,  angle:2.4, r:1.12, opacity:.2,  dash:[12,8] },
    ];
    const pCount = isMobile() ? 30 : 60;
    const particles = Array.from({ length: pCount }, () => ({
      ring:Math.floor(Math.random()*3), angle:Math.random()*Math.PI*2,
      speed:(Math.random()*.008+.004)*(Math.random()>.5?1:-1),
      size:Math.random()*2+1, brightness:Math.random(), color:Math.random()>.5?"#C8F55A":"#4ADE80",
    }));
    globeRef.current = { dots, rings, particles };
  }
  const getSphereParams = (s) => {
    const { W, H, transitionT } = s;
    const mobile = W <= 767;
    const introR = Math.min(W,H)*(mobile?.36:.38), settledR = Math.min(W,H)*(mobile?.38:.32);
    const iCx=W/2, iCy=H/2, sCx=mobile?W/2:W*.62, sCy=mobile?H*.38:H*.48;
    const t = easeInOutCubic(transitionT);
    return { r:introR+(settledR-introR)*t, cx:iCx+(sCx-iCx)*t, cy:iCy+(sCy-iCy)*t };
  };
  const project = (phi,theta,rot,cx,cy,r) => {
    const x3=Math.cos(phi)*Math.sin(theta+rot), y3=Math.sin(phi), z3=Math.cos(phi)*Math.cos(theta+rot);
    const p=2.8, sc=p/(p+z3*.4);
    return { x:cx+x3*r*sc, y:cy-y3*r*sc, z:z3, sc };
  };
  const lerpSphere = (a,b,t,rot,cx,cy,r) => {
    const ax=Math.cos(a.phi)*Math.sin(a.theta), ay=Math.sin(a.phi), az=Math.cos(a.phi)*Math.cos(a.theta);
    const bx=Math.cos(b.phi)*Math.sin(b.theta), by=Math.sin(b.phi), bz=Math.cos(b.phi)*Math.cos(b.theta);
    const dot=Math.min(1,ax*bx+ay*by+az*bz), omega=Math.acos(dot);
    let rx,ry,rz;
    if(omega<.001){rx=ax+t*(bx-ax);ry=ay+t*(by-ay);rz=az+t*(bz-az);}
    else{const s=Math.sin(omega),sa=Math.sin((1-t)*omega)/s,sb=Math.sin(t*omega)/s;rx=sa*ax+sb*bx;ry=sa*ay+sb*by;rz=sa*az+sb*bz;}
    const lift=1.06+Math.sin(t*Math.PI)*.12; rx*=lift;ry*=lift;rz*=lift;
    const p=2.8,sc=p/(p+rz*.4);
    return { x:cx+rx*r*sc, y:cy-ry*r*sc, z:rz, sc };
  };
  const spawnArc = (s) => {
    const d=globeRef.current.dots;
    const a=d[Math.floor(Math.random()*d.length)], b=d[Math.floor(Math.random()*d.length)];
    s.arcs.push({ from:{phi:a.phi,theta:a.theta}, to:{phi:b.phi,theta:b.theta}, progress:0, speed:.008+Math.random()*.006, life:1, fadeSpeed:.012 });
  };
  const drawFrame = (ctx,s) => {
    const { W, H, rotation, sphereAlpha, arcs } = s;
    const { dots, rings, particles } = globeRef.current;
    const { r, cx, cy } = getSphereParams(s);
    ctx.clearRect(0,0,W,H);
    const bg=ctx.createRadialGradient(cx,cy,0,cx,cy,r*1.8);
    bg.addColorStop(0,`rgba(16,28,16,${.5*sphereAlpha})`); bg.addColorStop(1,"transparent");
    ctx.fillStyle=bg; ctx.fillRect(0,0,W,H);
    const sg=ctx.createRadialGradient(cx,cy,0,cx,cy,r*1.1);
    sg.addColorStop(0,`rgba(74,222,128,${.04*sphereAlpha})`); sg.addColorStop(1,"transparent");
    ctx.fillStyle=sg; ctx.beginPath(); ctx.arc(cx,cy,r*1.1,0,Math.PI*2); ctx.fill();
    rings.forEach(ring => {
      ring.angle+=ring.speed;
      const steps=120, pts=[];
      for(let i=0;i<=steps;i++){
        const a=(i/steps)*Math.PI*2+ring.angle;
        const rx2=Math.cos(a)*ring.r, ry2=Math.sin(a)*Math.cos(ring.tilt)*ring.r, rz2=Math.sin(a)*Math.sin(ring.tilt)*ring.r;
        const rX=rx2*Math.cos(rotation)+rz2*Math.sin(rotation), rZ=-rx2*Math.sin(rotation)+rz2*Math.cos(rotation);
        const p2=2.8,sc2=p2/(p2+rZ*.4); pts.push({ x:cx+rX*r*sc2, y:cy-ry2*r*sc2 });
      }
      ctx.save(); ctx.globalAlpha=sphereAlpha; ctx.setLineDash(ring.dash);
      ctx.strokeStyle=`rgba(200,245,90,${ring.opacity})`; ctx.lineWidth=.8;
      ctx.beginPath(); pts.forEach((pt,i)=>i===0?ctx.moveTo(pt.x,pt.y):ctx.lineTo(pt.x,pt.y)); ctx.stroke(); ctx.restore();
      const da=Math.cos(ring.angle)*ring.r, db=Math.sin(ring.angle)*Math.cos(ring.tilt)*ring.r, dc=Math.sin(ring.angle)*Math.sin(ring.tilt)*ring.r;
      const dX=da*Math.cos(rotation)+dc*Math.sin(rotation), dZ=-da*Math.sin(rotation)+dc*Math.cos(rotation);
      const p3=2.8,dSc=p3/(p3+dZ*.4), px=cx+dX*r*dSc, py=cy-db*r*dSc;
      const g2=ctx.createRadialGradient(px,py,0,px,py,6*dSc);
      g2.addColorStop(0,"rgba(200,245,90,0.9)"); g2.addColorStop(1,"transparent");
      ctx.save(); ctx.globalAlpha=sphereAlpha; ctx.beginPath(); ctx.arc(px,py,5*dSc,0,Math.PI*2); ctx.fillStyle=g2; ctx.fill(); ctx.restore();
    });
    const visible=[];
    dots.forEach(dot=>{ dot.pulse+=dot.pulseSpeed; const p4=project(dot.phi,dot.theta,rotation,cx,cy,r); if(p4.z>-0.1) visible.push({...p4,dot}); });
    visible.sort((a,b2)=>a.z-b2.z);
    visible.forEach(({x,y,z,sc,dot})=>{
      const df=(z+1)/2, pulse=.6+.4*Math.sin(dot.pulse), alpha=df*dot.brightness*pulse*sphereAlpha;
      const size=dot.size*sc*(.5+df*.8), isAcc=dot.brightness>.8&&df>.7;
      const color=isAcc?`rgba(200,245,90,${alpha})`:`rgba(74,222,128,${alpha*.7})`;
      if(size>.3){
        const g=ctx.createRadialGradient(x,y,0,x,y,size*2);
        g.addColorStop(0,color); g.addColorStop(1,"transparent");
        ctx.beginPath(); ctx.arc(x,y,size*2,0,Math.PI*2); ctx.fillStyle=g; ctx.fill();
        ctx.beginPath(); ctx.arc(x,y,size,0,Math.PI*2); ctx.fillStyle=color; ctx.fill();
      }
    });
    if(sphereAlpha>.4){
      s.arcTimer++;
      if(s.arcTimer>80){ spawnArc(s); s.arcTimer=0; }
      for(let i=arcs.length-1;i>=0;i--){
        const arc=arcs[i]; arc.progress=Math.min(1,arc.progress+arc.speed);
        if(arc.progress>=1) arc.life-=arc.fadeSpeed;
        if(arc.life<=0){ arcs.splice(i,1); continue; }
        const steps=40, du=Math.floor(arc.progress*steps);
        ctx.save(); ctx.globalAlpha=arc.life*.6*sphereAlpha; let prev=null;
        for(let k=0;k<=du;k++){
          const t2=k/steps, pt=lerpSphere(arc.from,arc.to,t2,rotation,cx,cy,r);
          if(pt.z<-.05){prev=null;continue;}
          if(prev&&prev.z>-.05){
            const sa=(1-Math.abs(t2-.5)*2)*.9;
            ctx.strokeStyle=`rgba(200,245,90,${sa})`; ctx.lineWidth=1.2*pt.sc;
            ctx.beginPath(); ctx.moveTo(prev.x,prev.y); ctx.lineTo(pt.x,pt.y); ctx.stroke();
          }
          if(k===du){
            const g=ctx.createRadialGradient(pt.x,pt.y,0,pt.x,pt.y,6);
            g.addColorStop(0,"rgba(200,245,90,1)"); g.addColorStop(1,"transparent");
            ctx.fillStyle=g; ctx.beginPath(); ctx.arc(pt.x,pt.y,5,0,Math.PI*2); ctx.fill();
          }
          prev=pt;
        }
        ctx.restore();
      }
    }
    particles.forEach(p5=>{ p5.angle+=p5.speed; const ring=rings[p5.ring]; const rx3=Math.cos(p5.angle)*ring.r, ry3=Math.sin(p5.angle)*Math.cos(ring.tilt)*ring.r, rz3=Math.sin(p5.angle)*Math.sin(ring.tilt)*ring.r; const pX=rx3*Math.cos(rotation)+rz3*Math.sin(rotation), pZ=-rx3*Math.sin(rotation)+rz3*Math.cos(rotation); const pp=2.8,pSc=pp/(pp+pZ*.4), ppx=cx+pX*r*pSc, ppy=cy-ry3*r*pSc; const df=(pZ+1)/2, alpha=(.4+p5.brightness*.6)*df*sphereAlpha; ctx.globalAlpha=alpha; ctx.beginPath(); ctx.arc(ppx,ppy,p5.size*pSc*.8,0,Math.PI*2); ctx.fillStyle=p5.color; ctx.fill(); ctx.globalAlpha=1; });
    const mx=(s.mouse.x-W/2)/W; const targetRS=.004+mx*.003; s.rotSpeed+=(targetRS-s.rotSpeed)*.05; s.rotation+=s.rotSpeed; s.frame++;
  };
  const animVal = (setter,from,to,duration,ease=easeOutCubic) => new Promise(resolve=>{
    const start=performance.now();
    const tick=()=>{ const t=clampT((performance.now()-start)/duration,0,1); setter(from+(to-from)*ease(t)); if(t<1) requestAnimationFrame(tick); else { setter(to); resolve(); } };
    requestAnimationFrame(tick);
  });
  const anim = (el,kf,opts) => { if(!el) return Promise.resolve(); return el.animate(kf,{fill:"forwards",...opts}).finished; };
  const runIntro = useCallback(async () => {
    const s=stateRef.current; s.phase="fadein"; s.sphereAlpha=0; s.transitionT=0; s.rotation=0;
    await animVal(v=>{ s.sphereAlpha=v; },0,1,900,easeOutCubic);
    await new Promise(r=>setTimeout(r,100));
    const intro=introRef.current;
    if(intro){
      const wm=intro.querySelector(".intro-wm"), bar=intro.querySelector(".intro-bar"), tag=intro.querySelector(".intro-tag");
      if(wm) await anim(wm,[{opacity:0,transform:"scale(.94) translateY(12px)"},{opacity:1,transform:"scale(1) translateY(0)"}],{duration:700,easing:"cubic-bezier(.16,1,.3,1)"});
      await new Promise(r=>setTimeout(r,180));
      if(bar) await anim(bar,[{transform:"scaleX(0)",transformOrigin:"left"},{transform:"scaleX(1)",transformOrigin:"left"}],{duration:600,easing:"cubic-bezier(.16,1,.3,1)"});
      await new Promise(r=>setTimeout(r,300));
      if(tag) await anim(tag,[{opacity:0,transform:"translateY(8px)"},{opacity:1,transform:"translateY(0)"}],{duration:500,easing:"ease"});
    }
    await new Promise(r=>setTimeout(r,820));
    s.phase="transition";
    const intro2=introRef.current;
    if(intro2){
      const wm=intro2.querySelector(".intro-wm"), tag=intro2.querySelector(".intro-tag");
      if(wm) wm.animate([{opacity:1,transform:"scale(1)"},{opacity:0,transform:"scale(.9) translateY(-18px)"}],{duration:550,easing:"cubic-bezier(.4,0,1,1)",fill:"forwards"});
      if(tag) tag.animate([{opacity:1},{opacity:0}],{duration:380,fill:"forwards"});
    }
    animVal(v=>{ s.transitionT=v; },0,1,900,easeInOutCubic);
    await new Promise(r=>setTimeout(r,180));
    const hg=heroRef.current?.querySelector(".hero-gradient");
    if(hg) hg.animate([{opacity:0},{opacity:1}],{duration:800,easing:"ease",fill:"forwards"});
    const nav=navRef.current;
    if(nav) nav.animate([{opacity:0,transform:"translateY(-16px)"},{opacity:1,transform:"translateY(0)"}],{duration:700,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
    await new Promise(r=>setTimeout(r,150));
    const hc=heroRef.current;
    if(hc){
      const label=hc.querySelector(".hero-label-inner"), line1=hc.querySelector(".hl1"), line2=hc.querySelector(".hl2"), meta=hc.querySelector(".hero-meta"), labelWrap=hc.querySelector(".hero-label-wrap");
      if(labelWrap) labelWrap.animate([{opacity:0},{opacity:1}],{duration:300,fill:"forwards"});
      if(label) label.animate([{transform:"translateY(100%)"},{transform:"translateY(0)"}],{duration:700,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
      await new Promise(r=>setTimeout(r,100));
      if(line1) line1.animate([{transform:"translateY(110%)"},{transform:"translateY(0)"}],{duration:1000,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
      await new Promise(r=>setTimeout(r,70));
      if(line2) line2.animate([{transform:"translateY(110%)"},{transform:"translateY(0)"}],{duration:1000,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
      await new Promise(r=>setTimeout(r,380));
      if(meta) meta.animate([{opacity:0,transform:"translateY(14px)"},{opacity:1,transform:"translateY(0)"}],{duration:700,easing:"ease",fill:"forwards"});
    }
    s.phase="done";
  }, []);
  useEffect(()=>{
    const canvas=canvasRef.current; if(!canvas) return;
    const ctx=canvas.getContext("2d"), s=stateRef.current;
    const resize=()=>{ s.W=canvas.width=canvas.offsetWidth; s.H=canvas.height=canvas.offsetHeight; s.mouse.x=s.W/2; s.mouse.y=s.H/2; };
    resize();
    const onMouse=e=>{ s.mouse.x=e.clientX; s.mouse.y=e.clientY; };
    const onTouch=e=>{ if(e.touches[0]){ s.mouse.x=e.touches[0].clientX; } };
    window.addEventListener("resize",resize); window.addEventListener("mousemove",onMouse); window.addEventListener("touchmove",onTouch,{passive:true});
    let raf; const loop=()=>{ drawFrame(ctx,s); raf=requestAnimationFrame(loop); }; loop();
    document.fonts.ready.then(()=>setTimeout(runIntro,200));
    return ()=>{ cancelAnimationFrame(raf); window.removeEventListener("resize",resize); window.removeEventListener("mousemove",onMouse); window.removeEventListener("touchmove",onTouch); };
  }, [runIntro]);
  return { canvasRef, introRef, heroRef, navRef };
}

function useReveal(deps = []) {
  useEffect(() => {
    const obs = new IntersectionObserver(entries => {
      entries.forEach(e => { if (e.isIntersecting) e.target.classList.add("in"); });
    }, { threshold: 0.1 });
    document.querySelectorAll(".rv,.fi").forEach(el => obs.observe(el));
    return () => obs.disconnect();
  }, deps);
}

/* ─── SIDEBAR ─── */
function Sidebar({ active, go, auth, onLogout }) {
  const navItems = [
    { id: "dashboard", ico: "◈", label: "Dashboard" },
    { id: "devices", ico: "📱", label: "Devices", badge: "3" },
    { id: "auditlogs", ico: "📋", label: "Audit Logs" },
    { id: "rbac", ico: "👥", label: "Users & Roles" },
  ];
  const secItems = [
    { id: "recovery", ico: "🔒", label: "Recovery" },
    { id: "risk", ico: "🛡", label: "Risk Intel" },
    { id: "sessions", ico: "👁", label: "Sessions" },
  ];
  const adminItems = [
    { id: "admin", ico: "⚙", label: "Admin" },
    { id: "policy", ico: "📜", label: "Policy Engine" },
    { id: "orgsettings", ico: "🏢", label: "Org Settings" },
  ];
  const NavBtn = ({ item }) => (
    <button key={item.id} onClick={() => go(item.id)} className="si" style={{
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
  const SectionLabel = ({ label, topBorder }) => (
    <div className="sb-label-txt" style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted2)", padding: "4px 4px 6px", marginBottom: 2, ...(topBorder ? { marginTop: 16, borderTop: "1px solid var(--line)", paddingTop: 14 } : {}) }}>{label}</div>
  );
  return (
    <aside style={{ width: 220, flexShrink: 0, background: "var(--ink-2)", borderRight: "1px solid var(--line)", display: "flex", flexDirection: "column", position: "sticky", top: 0, height: "100vh", overflowY: "auto", overflowX: "hidden" }}>
      <div style={{ padding: "20px 20px 16px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14, cursor: "pointer" }} onClick={() => go("landing")}>
          <div style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent)", flexShrink: 0 }} />
          <span className="sb-mark" style={{ fontFamily: "var(--display)", fontSize: 16, letterSpacing: ".12em" }}>CRYPTON</span>
        </div>
        <button onClick={() => go("home")} style={{ width: "100%", display: "flex", alignItems: "center", gap: 8, padding: "7px 10px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "rgba(255,255,255,.03)", border: "1px solid var(--line)", cursor: "pointer", transition: "color .2s, background .2s" }}
          onMouseEnter={e => { e.currentTarget.style.color = "var(--paper)"; e.currentTarget.style.background = "rgba(255,255,255,.06)"; }}
          onMouseLeave={e => { e.currentTarget.style.color = "var(--muted)"; e.currentTarget.style.background = "rgba(255,255,255,.03)"; }}>
          <span style={{ fontSize: 10 }}>←</span>
          <span className="si-label">Back to Home</span>
        </button>
      </div>
      <nav style={{ padding: "16px 12px", flex: 1 }}>
        <SectionLabel label="Main" />
        {navItems.map(item => <NavBtn key={item.id} item={item} />)}
        <SectionLabel label="Security" topBorder />
        {secItems.map(item => <NavBtn key={item.id} item={item} />)}
        <SectionLabel label="Admin" topBorder />
        {adminItems.map(item => <NavBtn key={item.id} item={item} />)}
      </nav>
      <div style={{ padding: 16, borderTop: "1px solid var(--line)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, padding: 10, background: "rgba(255,255,255,.025)" }}>
          <div style={{ width: 30, height: 30, background: "var(--accent)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--display)", fontSize: 14, color: "var(--ink)", flexShrink: 0 }}>
            {auth?.username ? auth.username[0].toUpperCase() : "?"}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div className="user-name" style={{ fontSize: 12, fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{auth?.username || "Not signed in"}</div>
            <div className="user-role" style={{ fontFamily: "var(--mono)", fontSize: 9, color: auth?.token ? "var(--success)" : "var(--muted)", letterSpacing: ".06em", textTransform: "uppercase" }}>
              {auth?.token ? "Authenticated" : "No session"}
            </div>
          </div>
        </div>
        {onLogout && (
          <button onClick={onLogout} style={{ width: "100%", marginTop: 8, padding: "7px 10px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "1px solid var(--line)", cursor: "pointer", transition: "color .2s, border-color .2s" }}
            onMouseEnter={e => { e.currentTarget.style.color = "var(--danger)"; e.currentTarget.style.borderColor = "rgba(248,113,113,.4)"; }}
            onMouseLeave={e => { e.currentTarget.style.color = "var(--muted)"; e.currentTarget.style.borderColor = "var(--line)"; }}>
            Sign Out
          </button>
        )}
      </div>
    </aside>
  );
}

/* ─── APP SHELL (pages with sidebar) ─── */
const PAGE_LABELS = {
  dashboard: "Dashboard", devices: "Devices", auditlogs: "Audit Logs", rbac: "Users & Roles",
  recovery: "Recovery", risk: "Risk Intelligence", sessions: "Sessions",
  admin: "Admin", policy: "Policy Engine", orgsettings: "Org Settings",
};
function Breadcrumb({ page, go }) {
  return (
    <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted2)", display: "flex", alignItems: "center", gap: 6, marginBottom: 8 }}>
      <button onClick={() => go("landing")} style={{ background: "none", border: "none", cursor: "pointer", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted)", padding: 0, transition: "color .2s" }}
        onMouseEnter={e => e.target.style.color = "var(--accent)"} onMouseLeave={e => e.target.style.color = "var(--muted)"}>CRYPTON</button>
      <span>/</span>
      <span style={{ color: "var(--paper)" }}>{PAGE_LABELS[page] || page}</span>
    </div>
  );
}
function BottomTabBar({ active, go }) {
  const tabs = [
    { id: "dashboard", ico: "◈",  label: "Home"    },
    { id: "devices",   ico: "📱", label: "Devices"  },
    { id: "auditlogs", ico: "📋", label: "Logs"     },
    { id: "risk",      ico: "🛡", label: "Risk"     },
    { id: "admin",     ico: "⚙",  label: "Admin"    },
  ];
  return (
    <div className="bottom-tabs">
      <div className="bottom-tabs-inner">
        {tabs.map(t => (
          <button key={t.id} className={`tab-btn${active === t.id ? " active" : ""}`} onClick={() => go(t.id)}>
            <span>{t.ico}</span>
            <span>{t.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function AppShell({ active, go, auth, onLogout, children }) {
  return (
    <div style={{ display: "flex", flexDirection: "row", minHeight: "100vh", overflow: "hidden" }}>
      <Sidebar active={active} go={go} auth={auth} onLogout={onLogout} />
      <main className="app-main" style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", minWidth: 0 }}>
        <div className="breadcrumb-wrap" style={{ padding: "14px 44px 0" }}>
          <Breadcrumb page={active} go={go} />
        </div>
        {children}
      </main>
      <BottomTabBar active={active} go={go} />
    </div>
  );
}

/* ─── BUTTONS ─── */
const BtnF = ({ children, onClick, style = {} }) => (
  <button onClick={onClick} style={{
    display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 10,
    letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--accent)",
    padding: "12px 24px", border: "1px solid var(--accent)", cursor: "pointer", transition: "all .25s", ...style
  }}
    onMouseEnter={e => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "var(--accent)"; }}
    onMouseLeave={e => { e.currentTarget.style.background = "var(--accent)"; e.currentTarget.style.color = "var(--ink)"; }}
  >{children}</button>
);

const BtnO = ({ children, onClick, style = {} }) => (
  <button onClick={onClick} style={{
    display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 10,
    letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none",
    padding: "12px 24px", border: "1px solid var(--line2)", cursor: "pointer", transition: "all .25s", ...style
  }}
    onMouseEnter={e => { e.currentTarget.style.background = "var(--paper)"; e.currentTarget.style.color = "var(--ink)"; }}
    onMouseLeave={e => { e.currentTarget.style.background = "none"; e.currentTarget.style.color = "var(--paper)"; }}
  >{children}</button>
);

/* ═══════════════════════════════════════════════════════════════
   LANDING PAGE
═══════════════════════════════════════════════════════════════ */
function HeroCanvas() {
  const ref = useRef(null);
  useEffect(() => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const resize = () => { c.width = c.offsetWidth; c.height = c.offsetHeight; };
    resize(); window.addEventListener("resize", resize);
    const nodes = Array.from({ length: 55 }, () => ({
      x: Math.random(), y: Math.random(),
      vx: (Math.random() - .5) * .00025, vy: (Math.random() - .5) * .00025
    }));
    let raf;
    const draw = () => {
      ctx.clearRect(0, 0, c.width, c.height);
      nodes.forEach(n => {
        n.x += n.vx; n.y += n.vy;
        if (n.x < 0 || n.x > 1) n.vx *= -1;
        if (n.y < 0 || n.y > 1) n.vy *= -1;
      });
      for (let i = 0; i < nodes.length; i++) for (let j = i + 1; j < nodes.length; j++) {
        const dx = (nodes[i].x - nodes[j].x) * c.width, dy = (nodes[i].y - nodes[j].y) * c.height;
        const d = Math.sqrt(dx * dx + dy * dy);
        if (d < 110) {
          ctx.beginPath(); ctx.moveTo(nodes[i].x * c.width, nodes[i].y * c.height);
          ctx.lineTo(nodes[j].x * c.width, nodes[j].y * c.height);
          ctx.strokeStyle = `rgba(200,245,90,${.12 * (1 - d / 110)})`; ctx.lineWidth = .6; ctx.stroke();
        }
      }
      nodes.forEach(n => { ctx.beginPath(); ctx.arc(n.x * c.width, n.y * c.height, 1.2, 0, Math.PI * 2); ctx.fillStyle = "rgba(200,245,90,.35)"; ctx.fill(); });
      raf = requestAnimationFrame(draw);
    };
    draw();
    return () => { window.removeEventListener("resize", resize); cancelAnimationFrame(raf); };
  }, []);
  return <canvas ref={ref} style={{ position: "absolute", inset: 0, opacity: .3, width: "100%", height: "100%" }} />;
}

function Landing({ go, toast }) {
  useReveal([]);
  const [scrolled, setScrolled] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const { canvasRef, introRef, heroRef, navRef } = useSphereIntro();

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 40);
    window.addEventListener("scroll", onScroll);
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  const scrollTo = id => {
    setMenuOpen(false);
    const el = document.getElementById(id);
    if (el) el.scrollIntoView({ behavior: "smooth" });
  };

  return (
    <div style={{ background: "var(--ink)" }}>
      <canvas ref={canvasRef} style={{ position: "fixed", inset: 0, width: "100%", height: "100%", zIndex: 0 }} />
      <div ref={introRef} style={{ position: "fixed", inset: 0, zIndex: 500, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>
        <div className="intro-wm" style={{ fontFamily: "var(--display)", fontSize: "clamp(48px,10vw,120px)", letterSpacing: ".14em", textTransform: "uppercase", opacity: 0, position: "relative", textAlign: "center" }}>
          CRYPTON
          <div className="intro-bar" style={{ position: "absolute", bottom: -10, left: 0, right: 0, height: 2, background: "var(--accent)", transform: "scaleX(0)" }} />
        </div>
        <div className="intro-tag" style={{ fontFamily: "var(--mono)", fontSize: "clamp(9px,1.5vw,11px)", letterSpacing: ".22em", textTransform: "uppercase", color: "var(--accent)", marginTop: 20, opacity: 0 }}>
          Zero-trust · Device identity · No passwords
        </div>
      </div>
      <div ref={navRef} className={"landing-nav" + (scrolled ? " scrolled" : "")} style={{ opacity: 0, zIndex: 600 }}>
        <a onClick={() => scrollTo("hero")} style={{ fontFamily: "var(--display)", fontSize: 20, letterSpacing: ".14em", color: "var(--paper)", textDecoration: "none", cursor: "pointer" }}>CRYPTON</a>
        <ul className="nav-links-wrap" style={{ display: "flex", gap: 36, listStyle: "none" }}>
          {[{ label: "About", target: "about" }, { label: "Protocol", target: "protocol" }, { label: "Features", target: "features" }, { label: "Developer", target: "developer" }].map(l => (
            <li key={l.label}>
              <button onClick={() => scrollTo(l.target)} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none", border: "none", opacity: .7, transition: "opacity .2s", cursor: "pointer" }}
                onMouseEnter={e => e.target.style.opacity = 1} onMouseLeave={e => e.target.style.opacity = .7}>{l.label}</button>
            </li>
          ))}
        </ul>
        <div className="nav-desktop-btns" style={{ display: "flex", gap: 10 }}>
          <button onClick={() => go("dashboard")} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none", padding: "10px 18px", border: "1px solid var(--line2)", cursor: "pointer", transition: "all .2s" }}
            onMouseEnter={e => e.currentTarget.style.borderColor = "var(--paper)"}
            onMouseLeave={e => e.currentTarget.style.borderColor = "var(--line2)"}>Dashboard</button>
          <button onClick={() => go("register")} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--paper)", padding: "10px 22px", border: "none", cursor: "pointer", transition: "background .2s" }}
            onMouseEnter={e => e.currentTarget.style.background = "var(--accent)"}
            onMouseLeave={e => e.currentTarget.style.background = "var(--paper)"}>Enroll Device</button>
        </div>
        <button className={"mob-menu-btn" + (menuOpen ? " open" : "")} onClick={() => setMenuOpen(o => !o)} aria-label="Menu">
          <span /><span /><span />
        </button>
      </div>
      <div className={"mob-drawer" + (menuOpen ? " open" : "")}>
        {[{ label: "About", target: "about" }, { label: "Protocol", target: "protocol" }, { label: "Features", target: "features" }, { label: "Developer", target: "developer" }].map(l => (
          <button key={l.label} className="mob-link" onClick={() => scrollTo(l.target)}>{l.label}</button>
        ))}
        <div className="mob-drawer-ctas">
          <button onClick={() => { setMenuOpen(false); go("dashboard"); }} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none", padding: "14px 0", border: "1px solid var(--line2)", cursor: "pointer", textAlign: "center" }}>Dashboard</button>
          <button onClick={() => { setMenuOpen(false); go("register"); }} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--accent)", padding: "14px 0", border: "none", cursor: "pointer", textAlign: "center" }}>Enroll Device</button>
        </div>
      </div>
      <section id="hero" style={{ height: "100vh", position: "relative", overflow: "hidden", display: "flex", flexDirection: "column", justifyContent: "flex-end", padding: "0 52px 56px", zIndex: 10 }} className="hero-pad">
        <div ref={heroRef} style={{ position: "absolute", inset: 0, pointerEvents: "none" }}>
          <div className="hero-gradient" style={{ position: "absolute", inset: 0, background: "linear-gradient(to top,rgba(10,10,10,.9) 0%,rgba(10,10,10,.25) 45%,transparent 72%)", opacity: 0 }} />
          <div style={{ position: "absolute", bottom: 56, left: 52, right: 52 }} className="hero-pad-inner">
            <div className="hero-label-wrap" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", display: "flex", alignItems: "center", gap: 14, marginBottom: 20, overflow: "hidden", opacity: 0 }}>
              <div style={{ width: 36, height: 1, background: "var(--accent)", flexShrink: 0 }} />
              <div className="hero-label-inner" style={{ transform: "translateY(100%)" }}>Zero-trust · Device identity · No passwords</div>
            </div>
            <div style={{ overflow: "hidden" }}>
              <div className="hl1" style={{ fontFamily: "var(--display)", fontSize: "clamp(58px,13.5vw,195px)", lineHeight: .9, letterSpacing: ".025em", textTransform: "uppercase", transform: "translateY(110%)", display: "block" }}>Identity</div>
            </div>
            <div style={{ overflow: "hidden" }}>
              <div className="hl2" style={{ fontFamily: "var(--serif)", fontStyle: "italic", fontSize: "clamp(52px,12.5vw,180px)", lineHeight: .9, transform: "translateY(110%)", display: "block" }}>Redefined.</div>
            </div>
            <div className="hero-meta" style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", marginTop: 36, paddingTop: 24, borderTop: "1px solid var(--line)", opacity: 0, flexWrap: "wrap", gap: 16 }}>
              <p style={{ fontSize: 14, color: "var(--muted)", maxWidth: 300, lineHeight: 1.75, fontWeight: 300 }}>Authentication powered by hardware cryptography. Every request verified. Every device accountable.</p>
              <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "flex", alignItems: "center", gap: 12, cursor: "pointer" }} onClick={() => scrollTo("about")}>
                <div style={{ width: 1, height: 44, background: "rgba(122,117,112,.3)", position: "relative", overflow: "hidden" }}>
                  <div style={{ position: "absolute", top: 0, left: 0, width: "100%", height: "50%", background: "var(--accent)", animation: "sp 2s ease-in-out infinite" }} />
                </div>
                Scroll to explore
              </div>
            </div>
          </div>
        </div>
      </section>
      <div style={{ background: "var(--accent)", overflow: "hidden", padding: "13px 0", whiteSpace: "nowrap", position: "relative", zIndex: 10 }} aria-hidden="true">
        <div className="ticker-track">
          {["Zero Trust","Device Identity","Hardware Keys","No Passwords","Cryptographic Proof","Instant Revocation"].flatMap(t => [
            <span key={t} style={{ fontFamily: "var(--display)", fontSize: 17, letterSpacing: ".1em", color: "var(--ink)", padding: "0 28px", textTransform: "uppercase" }}>{t}</span>,
            <span key={t+"d"} style={{ opacity: .35, fontFamily: "var(--display)", fontSize: 17, color: "var(--ink)" }}>—</span>
          ])}
          {["Zero Trust","Device Identity","Hardware Keys","No Passwords","Cryptographic Proof","Instant Revocation"].flatMap(t => [
            <span key={t+"2"} style={{ fontFamily: "var(--display)", fontSize: 17, letterSpacing: ".1em", color: "var(--ink)", padding: "0 28px", textTransform: "uppercase" }}>{t}</span>,
            <span key={t+"d2"} style={{ opacity: .35, fontFamily: "var(--display)", fontSize: 17, color: "var(--ink)" }}>—</span>
          ])}
        </div>
      </div>
      <div style={{ position: "relative", zIndex: 10, background: "var(--ink)" }}>
        <section id="about" style={{ padding: "140px 52px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
            <span>01 — Manifesto</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="rv manifesto-grid" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 80, alignItems: "end" }}>
            <div style={{ fontFamily: "var(--serif)", fontSize: "clamp(34px,3.8vw,54px)", lineHeight: 1.15, letterSpacing: "-.01em" }}>
              Passwords were a<br /><em style={{ fontStyle: "italic", color: "var(--muted)" }}>compromise.</em><br />We built the alternative.
            </div>
            <div>
              <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.85, maxWidth: 360, fontWeight: 300, marginBottom: 36 }}>
                Crypton is a zero-trust identity platform where authentication is tied to cryptographic device keys — not passwords, not secrets, not human memory. Your private key never leaves your hardware.
              </p>
              <BtnO onClick={() => scrollTo("developer")}>Read the whitepaper →</BtnO>
              <div style={{ marginTop: 16 }}><BtnF onClick={() => go("register")}>Enroll Your Device →</BtnF></div>
            </div>
          </div>
        </section>
        <section id="protocol" style={{ padding: "0 52px 140px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
            <span>02 — Protocol</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <h2 className="rv" style={{ fontFamily: "var(--display)", fontSize: "clamp(44px,7vw,92px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .93, marginBottom: 6 }}>Three-step<br />verification</h2>
          <p className="rv rv-1" style={{ fontSize: 14, color: "var(--muted)", maxWidth: 420, fontWeight: 300, lineHeight: 1.75, marginBottom: 0 }}>A deterministic challenge-response protocol. No shared secrets. No replay attacks. Mathematically sound.</p>
          <div className="rv rv-1 hiw-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 0, background: "var(--line)", marginTop: 72, border: "1px solid var(--line)" }}>
            {[
              { n: "01", t: "Challenge", b: "Server issues a unique, time-bound cryptographic nonce. Non-repeatable. Expires in milliseconds. Cannot be predicted or pre-computed." },
              { n: "02", t: "Sign", b: "Your device’s hardware security module signs the nonce using its private key. The key never leaves the chip. Face ID. Touch ID. Hardware token." },
              { n: "03", t: "Verify", b: "The server verifies the signature against your registered public key. Match means access. Mismatch means instant rejection. Sub-200ms total." },
            ].map(c => <HiWCard key={c.n} {...c} />)}
          </div>
        </section>
        <section id="features" style={{ padding: "0 52px 140px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
            <span>03 — Capabilities</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="features-grid" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 0 }}>
            <div className="features-list" style={{ paddingRight: 72, borderRight: "1px solid var(--line)" }}>
              {[
                { i: "F.01", t: "No Passwords", b: "Eliminate the entire password attack surface. No phishing, no credential stuffing, no breached database exposure." },
                { i: "F.02", t: "Hardware Keys", b: "Private keys generated and stored in device secure enclaves. Extraction is physically impossible by design." },
                { i: "F.03", t: "Zero Trust", b: "Every single request independently verified. No implicit trust. Deny by default, verify by cryptographic proof." },
                { i: "F.04", t: "Instant Revocation", b: "Revoke a compromised device in under 500ms. Propagates globally. No stale sessions. No grace periods." },
                { i: "F.05", t: "Audit Trail", b: "Cryptographically signed, tamper-proof log of every authentication event. Export to CSV or JSON for compliance." },
                { i: "F.06", t: "Time-Lock Recovery", b: "24-hour mandatory waiting period on recovery. Existing trusted devices can cancel unauthorized attempts." },
              ].map((f, idx) => (
                <div key={f.i} className="feat-item rv" style={{ padding: "32px 0", borderBottom: idx < 5 ? "1px solid var(--line)" : "none", display: "flex", alignItems: "flex-start", gap: 20 }}>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".12em", marginTop: 3, flexShrink: 0 }}>{f.i}</span>
                  <div>
                    <div style={{ fontFamily: "var(--display)", fontSize: 22, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 7 }}>{f.t}</div>
                    <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{f.b}</div>
                  </div>
                </div>
              ))}
            </div>
            <div className="features-sticky" style={{ paddingLeft: 72 }}>
              <div className="rv" style={{ position: "sticky", top: 120, height: 400, background: "var(--ink-3)", border: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", overflow: "hidden" }}>
                <div style={{ position: "absolute", inset: 0, backgroundImage: "linear-gradient(var(--line) 1px,transparent 1px),linear-gradient(90deg,var(--line) 1px,transparent 1px)", backgroundSize: "44px 44px", maskImage: "radial-gradient(ellipse 80% 80% at 50% 50%,black,transparent)" }} />
                <div style={{ textAlign: "center", position: "relative", zIndex: 1 }}>
                  <div className="fv-glyph" style={{ fontSize: 100, lineHeight: 1, filter: "drop-shadow(0 0 50px rgba(200,245,90,.35))" }}>🔐</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".15em", textTransform: "uppercase", color: "var(--accent)", marginTop: 14 }}>// secure enclave active</div>
                </div>
              </div>
            </div>
          </div>
        </section>
        <div className="rv stats-grid" style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", background: "var(--ink-2)", borderTop: "1px solid var(--line)", borderBottom: "1px solid var(--line)" }}>
          {[
            { v: "0", d: "Passwords ever stored\nin the Crypton system" },
            { v: "100%", d: "Requests cryptographically\nverified, every time" },
            { v: "<200ms", d: "End-to-end authentication\nlatency on 4G" },
            { v: "∞", d: "Entropy in each\nhardware-generated key" },
          ].map((s, i) => (
            <div key={i} className="stat-c" style={{ padding: "52px 36px", borderRight: i < 3 ? "1px solid var(--line)" : "none" }}>
              <div style={{ fontFamily: "var(--display)", fontSize: "clamp(48px,5.5vw,76px)", lineHeight: 1, letterSpacing: ".02em", marginBottom: 10 }}>{s.v}</div>
              <div style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300, lineHeight: 1.65, whiteSpace: "pre-line" }}>{s.d}</div>
            </div>
          ))}
        </div>
        <section id="developer" style={{ padding: "140px 52px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
            <span>04 — Developer</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="dev-grid" style={{ display: "grid", gridTemplateColumns: "5fr 7fr", alignItems: "stretch" }}>
            <div className="rv dev-left" style={{ padding: "80px 56px 80px 0", borderRight: "1px solid var(--line)", display: "flex", flexDirection: "column", justifyContent: "center" }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 22 }}>// Integrate in 5 minutes</div>
              <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(44px,5.5vw,76px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 28 }}>Ship<br />Zero-<br />Trust.</h2>
              <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.85, marginBottom: 36, fontWeight: 300, maxWidth: 320 }}>Our SDK handles all cryptographic operations. Type-safe API. Works with any backend.</p>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 36 }}>
                {["TypeScript", "Rust", "Python", "Go"].map(l => <span key={l} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "5px 11px", border: "1px solid var(--line)", color: "var(--muted)" }}>{l}</span>)}
              </div>
              <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
                <BtnF onClick={() => go("admin")}>Read the Docs →</BtnF>
                <BtnO onClick={() => window.open("https://github.com/Aryanvirpsu/Crypton-DI", "_blank")}>GitHub ↗</BtnO>
              </div>
            </div>
            <div className="rv rv-1 dev-right" style={{ padding: "80px 0 80px 56px" }}>
              <CodeBlock toast={toast} />
            </div>
          </div>
        </section>
        <footer style={{ borderTop: "1px solid var(--line)", padding: "72px 52px 36px" }} className="section-pad">
          <div className="rv footer-grid" style={{ display: "grid", gridTemplateColumns: "3fr 1fr 1fr 1fr", gap: 48, paddingBottom: 56, borderBottom: "1px solid var(--line)" }}>
            <div className="footer-brand">
              <div style={{ fontFamily: "var(--display)", fontSize: 56, letterSpacing: ".08em", lineHeight: 1, marginBottom: 20 }}>CRYPTON</div>
              <p style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300, lineHeight: 1.7, maxWidth: 250 }}>Zero-trust device identity. Authentication powered by cryptography — not passwords, not hope.</p>
            </div>
            {[
              { h: "Product", links: [{ l: "Protocol", action: () => scrollTo("protocol") }, { l: "Features", action: () => scrollTo("features") }, { l: "Pricing", action: () => toast("Pricing — coming soon", "info") }, { l: "Changelog", action: () => go("auditlogs") }] },
              { h: "Developer", links: [{ l: "Documentation", action: () => go("admin") }, { l: "API Reference", action: () => go("admin") }, { l: "SDK", action: () => scrollTo("developer") }, { l: "GitHub", action: () => window.open("https://github.com/Aryanvirpsu/Crypton-DI", "_blank") }] },
              { h: "Company", links: [{ l: "About", action: () => scrollTo("about") }, { l: "Security", action: () => go("risk") }, { l: "Privacy", action: () => toast("Privacy policy — coming soon", "info") }, { l: "Terms", action: () => toast("Terms of service — coming soon", "info") }] },
            ].map(col => (
              <div key={col.h}>
                <h5 style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 18 }}>{col.h}</h5>
                <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: 11 }}>
                  {col.links.map(({ l, action }) => (
                    <li key={l}><button onClick={action} style={{ fontSize: 13, color: "var(--paper)", opacity: .55, background: "none", border: "none", cursor: "pointer", padding: 0, transition: "opacity .2s", fontFamily: "var(--body)" }}
                      onMouseEnter={e => e.target.style.opacity = 1} onMouseLeave={e => e.target.style.opacity = .55}>{l}</button></li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: 28, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted)", flexWrap: "wrap", gap: 8 }}>
            <span>© 2026 CRYPTON — ALL RIGHTS RESERVED</span><span>V1.0 · MARCH 2026</span>
          </div>
        </footer>
      </div>
    </div>
  );
}

function HiWCard({ n, t, b }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)}
      style={{ background: hov ? "var(--ink-2)" : "var(--ink)", padding: "44px 32px", position: "relative", overflow: "hidden", cursor: "default", transition: "background .35s", borderRight: "1px solid var(--line)" }}>
      <div style={{ fontFamily: "var(--display)", fontSize: 72, color: hov ? "rgba(200,245,90,.18)" : "rgba(244,241,236,.07)", lineHeight: 1, marginBottom: 28, transition: "color .3s" }}>{n}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 14 }}>{t}</div>
      <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{b}</div>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .5s cubic-bezier(.16,1,.3,1)" }} />
    </div>
  );
}

function CodeBlock({ toast }) {
  const code = `import { Crypton } from '@crypton/sdk';

const client = new Crypton({
  apiKey: process.env.CRYPTON_KEY,
  origin: 'https://yourapp.com',
});

// authenticate a device
const { verified, deviceId } =
  await client.authenticate({
    userId: 'user_abc123',
    challenge: generateNonce(),
  });

if (verified) {
  // ✓ cryptographically proven
  grantAccess(deviceId);
}`;
  return (
    <div style={{ background: "#0C0C0C", border: "1px solid var(--line)", overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "13px 18px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
        <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: ".05em" }}>crypton.ts</span>
        <button onClick={() => { navigator.clipboard.writeText(code); toast("Code copied"); }} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", color: "var(--accent)", background: "none", border: "1px solid rgba(200,245,90,.25)", padding: "4px 11px", cursor: "pointer" }}>COPY</button>
      </div>
      <div style={{ padding: "22px 18px", fontFamily: "var(--mono)", fontSize: 12.5, lineHeight: 2, overflowX: "auto" }}>
        <pre style={{ margin: 0 }}><code dangerouslySetInnerHTML={{ __html: code.replace(/import|const|await|if/g, w => `<span style="color:#CC99FF">${w}</span>`).replace(/'[^']*'/g, m => `<span style="color:#C8F55A">${m}</span>`).replace(/Crypton|authenticate|grantAccess|generateNonce/g, m => `<span style="color:#82AAFF">${m}</span>`) }} /></pre>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   HOME / SPLASH
═══════════════════════════════════════════════════════════════ */
function Home({ go }) {
  return (
    <div className="pg-in" style={{ background: "var(--ink)", minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", padding: "52px 24px", textAlign: "center", gap: 0 }}>
      <div style={{ fontFamily: "var(--display)", fontSize: "clamp(64px,14vw,160px)", letterSpacing: ".06em", lineHeight: .9, marginBottom: 32 }}>CRYPTON</div>
      <p style={{ fontSize: 14, color: "var(--muted)", maxWidth: 400, lineHeight: 1.75, fontWeight: 300, marginBottom: 52 }}>Zero-trust identity. Navigate the full platform below.</p>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 1, background: "var(--line)", maxWidth: 640, width: "100%", border: "1px solid var(--line)" }}>
        {[
          { id: "register", n: "01", t: "Register", d: "Create a new identity — triggers Windows Hello / Touch ID passkey creation" },
          { id: "login", n: "02", t: "Sign In", d: "Authenticate an existing account — Windows Hello passkey verification" },
          { id: "dashboard", n: "03", t: "Dashboard", d: "Security status, Trust Orb, and real-time activity feed" },
          { id: "devices", n: "04", t: "Devices", d: "Manage enrolled passkeys, view trust status, revoke access" },
          { id: "recovery", n: "05", t: "Recovery", d: "Time-lock vault, 24hr countdown, and zero-trust recovery flow" },
          { id: "landing", n: "00", t: "Landing Page", d: "Marketing site — hero, manifesto, features, developer docs" },
        ].map((c, i) => (
          <HomeCell key={c.id} {...c} go={go} full={i === 5} />
        ))}
      </div>
    </div>
  );
}

function HomeCell({ id, n, t, d, go, full }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)} onClick={() => go(id)}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", textAlign: "left", gridColumn: full ? "1/-1" : undefined }}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 12 }}>{n}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8, display: "flex", alignItems: "center", gap: 10 }}>
        {t} <span style={{ display: "inline-block", transition: "transform .25s", transform: hov ? "translateX(4px)" : "none", fontFamily: "var(--mono)" }}>→</span>
      </div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.65, fontWeight: 300 }}>{d}</div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   REGISTER
═══════════════════════════════════════════════════════════════ */
function Register({ go, toast, setAuth }) {
  const [step, setStep] = useState(0);
  const [ident, setIdent] = useState("");
  const [identErr, setIdentErr] = useState(false);
  const [pkDone, setPkDone] = useState(false);
  const [loading, setLoading] = useState(false);
  const [errMsg, setErrMsg] = useState("");
  const challengeRef = useRef(null);

  const gotoStep = s => setStep(s);

  const nextStep = async (from) => {
    if (from === 0) {
      if (!ident || ident.length < 2 || !/^[a-zA-Z0-9-]+$/.test(ident)) { setIdentErr(true); return; }
      setIdentErr(false);
      setLoading(true); setErrMsg("");
      try {
        const resp = await API.registerStart(ident, ident);
        challengeRef.current = { challenge_id: resp.challenge_id, publicKey: resp.publicKey };
        gotoStep(1);
      } catch (e) {
        setErrMsg("Server error: " + e.message);
      } finally { setLoading(false); }
    } else if (from === 1 && pkDone) {
      gotoStep(2);
    }
  };

  const activatePK = async () => {
    if (loading || pkDone || !challengeRef.current) return;
    setLoading(true); setErrMsg("");
    try {
      const ch = challengeRef.current;
      const options = {
        publicKey: {
          ...ch.publicKey,
          challenge: b64url.decode(ch.publicKey.challenge),
          user: { ...ch.publicKey.user, id: b64url.decode(ch.publicKey.user.id) }
        }
      };
      const credential = await navigator.credentials.create(options);
      const attestation = {
        id: credential.id,
        rawId: b64url.encode(credential.rawId),
        type: credential.type,
        response: {
          clientDataJSON: b64url.encode(credential.response.clientDataJSON),
          attestationObject: b64url.encode(credential.response.attestationObject),
        }
      };
      await API.registerFinish(ch.challenge_id, attestation);
      if (setAuth) setAuth({ token: null, username: ident });
      setPkDone(true);
      toast("Passkey created — key sealed in hardware", "success");
    } catch (e) {
      if (e.name === "NotAllowedError") setErrMsg("Passkey creation cancelled or timed out.");
      else setErrMsg(e.message || "Passkey creation failed");
    } finally { setLoading(false); }
  };

  const devClass = ["rv-device", "rv-device s2", "rv-device s3"][step];
  const devLabel = ["Waiting for identity...", "Creating passkey...", "Device enrolled ✓"][step];

  return (
    <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => go("home")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10, transition: "color .2s" }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← Back to Home</button>

        {/* PIPS */}
        <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 40 }}>
          {[0, 1, 2].map(i => (
            <div key={i} style={{ height: 3, background: i < step ? "var(--accent)" : i === step ? "var(--accent)" : "var(--line2)", width: i === step ? 28 : 16, transition: "all .3s" }} />
          ))}
        </div>

        {step === 0 && (
          <div className="pg-in">
            <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Create your<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>identity</em></h2>
            <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>Choose a username. This is how you'll be identified across the Crypton network.</p>
            <div style={{ marginBottom: 20 }}>
              <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Username</label>
              <input type="text" value={ident} onChange={e => setIdent(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && nextStep(0)}
                placeholder="e.g. alex" maxLength={32}
                style={{ width: "100%", padding: "14px 16px", background: "var(--ink-3)", border: `1px solid ${identErr ? "var(--danger)" : "var(--line2)"}`, color: "var(--paper)", fontFamily: "var(--body)", fontSize: 14, outline: "none" }} />
              {identErr && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginTop: 6 }}>2–32 chars, letters, numbers, hyphens only</div>}
              {errMsg && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginTop: 6 }}>{errMsg}</div>}
            </div>
            <BtnF onClick={() => nextStep(0)} style={{ opacity: loading ? .5 : 1, pointerEvents: loading ? "none" : "auto" }}>
              {loading ? "Connecting..." : "Continue →"}
            </BtnF>
          </div>
        )}

        {step === 1 && (
          <div className="pg-in">
            <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Create your<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>passkey</em></h2>
            <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>Your device generates a hardware-bound key pair. Windows Hello / Touch ID will prompt for biometric confirmation.</p>
            <div onClick={!pkDone && !loading ? activatePK : undefined} style={{
              width: "100%", padding: 28,
              border: pkDone ? "1px solid var(--success)" : errMsg ? "1px dashed var(--danger)" : "1px dashed rgba(200,245,90,.25)",
              background: pkDone ? "rgba(74,222,128,.08)" : errMsg ? "rgba(248,113,113,.05)" : "var(--accent-dim)",
              cursor: pkDone || loading ? "default" : "pointer",
              textAlign: "center", transition: "all .25s", marginBottom: errMsg ? 12 : 28
            }}>
              <div style={{ fontSize: 36, marginBottom: 10 }}>{pkDone ? "✅" : loading ? "⏳" : "🔑"}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 22, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 6 }}>
                {pkDone ? "Passkey Created ✓" : loading ? "Waiting for Windows Hello..." : "Create Passkey"}
              </div>
              <div style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300 }}>
                {pkDone ? "Hardware key sealed in secure enclave" : loading ? "Complete the biometric prompt on your device" : "Click to trigger Windows Hello / Touch ID"}
              </div>
            </div>
            {errMsg && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginBottom: 20, padding: "10px 14px", background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)" }}>{errMsg}</div>}
            <BtnF onClick={() => nextStep(1)} style={{ opacity: pkDone ? 1 : .4, pointerEvents: pkDone ? "auto" : "none" }}>Continue →</BtnF>
          </div>
        )}

        {step === 2 && (
          <div className="pg-in">
            <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Device<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>enrolled</em> ✓</h2>
            <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>Your device is now part of your Crypton identity. Authentication is instant and passwordless.</p>
            <div style={{ display: "flex", alignItems: "center", gap: 16, padding: 20, border: "1px solid rgba(74,222,128,.2)", background: "rgba(74,222,128,.05)", marginBottom: 28 }}>
              <div style={{ fontSize: 36 }}>💻</div>
              <div>
                <div style={{ fontFamily: "var(--display)", fontSize: 20, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 4 }}>{ident || "my-device"}</div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--success)" }}>Enrolled now · Passkey sealed</div>
              </div>
            </div>
            <BtnF onClick={() => go("dashboard")}>Open Dashboard →</BtnF>
          </div>
        )}
      </div>

      {/* VIZ PANEL */}
      <div style={{ background: "#080808", borderLeft: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden" }}>
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse 70% 70% at 50% 50%,rgba(200,245,90,.06),transparent)" }} />
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 20, position: "relative", zIndex: 1 }}>
          <div className={devClass}>
            💻
            <div className="rv-glow" />
            <div className="rv-key">🔑</div>
            <div className="rv-shield">🛡</div>
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)" }}>{devLabel}</div>
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   LOGIN
═══════════════════════════════════════════════════════════════ */
function Login({ go, toast, setAuth }) {
  const [username, setUsername] = useState("");
  const [loading, setLoading] = useState(false);
  const [errMsg, setErrMsg] = useState("");

  const handleLogin = async () => {
    if (!username || username.length < 2) { setErrMsg("Enter your username"); return; }
    setLoading(true); setErrMsg("");
    try {
      const startResp = await API.loginStart(username);
      const options = {
        publicKey: {
          ...startResp.publicKey,
          challenge: b64url.decode(startResp.publicKey.challenge),
          allowCredentials: (startResp.publicKey.allowCredentials || []).map(c => ({
            ...c, id: b64url.decode(c.id)
          }))
        }
      };
      const credential = await navigator.credentials.get(options);
      const assertion = {
        id: credential.id,
        rawId: b64url.encode(credential.rawId),
        type: credential.type,
        response: {
          clientDataJSON: b64url.encode(credential.response.clientDataJSON),
          authenticatorData: b64url.encode(credential.response.authenticatorData),
          signature: b64url.encode(credential.response.signature),
          userHandle: credential.response.userHandle ? b64url.encode(credential.response.userHandle) : null
        }
      };
      const finishResp = await API.loginFinish(startResp.challenge_id, assertion);
      localStorage.setItem('crypton_token', finishResp.token);
      localStorage.setItem('crypton_username', username);
      setAuth({ token: finishResp.token, username });
      toast("Authentication verified ✓", "success");
      go("dashboard");
    } catch (e) {
      if (e.name === "NotAllowedError") setErrMsg("Authentication cancelled or timed out.");
      else setErrMsg(e.message || "Authentication failed");
    } finally { setLoading(false); }
  };

  return (
    <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => go("home")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10, transition: "color .2s" }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← Back to Home</button>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>// Passkey Authentication</div>
        <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Sign in<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>without passwords</em></h2>
        <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 36 }}>Enter your username. Your device will verify your identity using Windows Hello or Touch ID — no password needed.</p>
        <div style={{ marginBottom: 20 }}>
          <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Username</label>
          <input type="text" value={username} onChange={e => setUsername(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleLogin()}
            placeholder="e.g. alex" maxLength={32}
            style={{ width: "100%", padding: "14px 16px", background: "var(--ink-3)", border: `1px solid ${errMsg ? "var(--danger)" : "var(--line2)"}`, color: "var(--paper)", fontFamily: "var(--body)", fontSize: 14, outline: "none", marginBottom: 8 }} />
          {errMsg && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", padding: "10px 14px", background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)" }}>{errMsg}</div>}
        </div>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <BtnF onClick={handleLogin} style={{ opacity: loading ? .5 : 1, pointerEvents: loading ? "none" : "auto" }}>
            {loading ? "Waiting for biometric..." : "Authenticate →"}
          </BtnF>
          <button onClick={() => go("register")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", transition: "color .2s" }}
            onMouseEnter={e => e.currentTarget.style.color = "var(--accent)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>No account? Register →</button>
        </div>
      </div>
      <div style={{ background: "#080808", borderLeft: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden" }}>
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse 70% 70% at 50% 50%,rgba(200,245,90,.06),transparent)" }} />
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 20, position: "relative", zIndex: 1, textAlign: "center", padding: 40 }}>
          <div style={{ fontSize: 72, marginBottom: 8 }}>{loading ? "⏳" : "🔐"}</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)" }}>
            {loading ? "Awaiting biometric..." : "Zero-knowledge proof"}
          </div>
          <div style={{ fontSize: 12, color: "var(--muted)", maxWidth: 200, lineHeight: 1.7 }}>
            {loading ? "Complete the Windows Hello prompt on your device" : "Your private key never leaves your hardware. No password to steal."}
          </div>
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   DASHBOARD
═══════════════════════════════════════════════════════════════ */
const ORB_DATA = [
  { title: "All Secure", desc: "All enrolled devices active and verified. No suspicious activity detected in the last 24 hours. Last sweep: 2 minutes ago.", cls: "", ico: "🛡", type: "success" },
  { title: "Action Required", desc: "2 devices have not authenticated recently. Review inactive device access policies.", cls: "w", ico: "⚠", type: "warning" },
  { title: "Threat Detected", desc: "Suspicious authentication attempt detected on Work Desktop. Immediate review recommended.", cls: "r", ico: "🚨", type: "danger" },
];

function Dashboard({ go, toast, auth, onLogout }) {
  const [orbIdx, setOrbIdx] = useState(0);
  const [activeDeviceCount, setActiveDeviceCount] = useState(MOCK_DASHBOARD_STATS.activeDevices);
  const orb = ORB_DATA[orbIdx];

  useReveal([orbIdx]);

  // Fetch real active device count if authenticated
  useEffect(() => {
    if (!auth?.token) return;
    API.listDevices(auth.token)
      .then(data => setActiveDeviceCount(data.filter(d => d.status === "active").length))
      .catch(() => {}); // fall back to mock on error
  }, [auth?.token]);

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
    <AppShell active="dashboard" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Dashboard</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Security overview</div></div>
        <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Enroll Device</BtnF>
      </div>
      <div style={{ padding: "36px 44px 60px", flex: 1 }}>
        {/* STAT CARDS */}
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", marginBottom: 28, border: "1px solid var(--line)" }}>
          {[
            { l: "Active Devices", v: String(activeDeviceCount), d: "↑ 1 this week", i: "📱", link: "devices" },
            { l: "Auth Events (24h)", v: String(MOCK_DASHBOARD_STATS.authEvents24h), d: "All verified", i: "⚡", link: "auditlogs" },
            { l: "Security Score", v: `${MOCK_DASHBOARD_STATS.securityScore}%`, d: "No issues found", i: "🛡", vc: "var(--success)", link: "risk" },
          ].map((s, i) => <StatCard key={i} {...s} go={go} />)}
        </div>

        {/* ORB */}
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "auto 1fr", border: "1px solid var(--line)", marginBottom: 28, background: "var(--ink-2)" }}>
          <div onClick={() => setOrb((orbIdx + 1) % 3)} style={{ width: 180, display: "flex", alignItems: "center", justifyContent: "center", padding: 40, borderRight: "1px solid var(--line)", cursor: "pointer", position: "relative" }}>
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
              <button onClick={() => go("sessions")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", padding: "7px 14px", border: "1px solid rgba(248,113,113,.3)", color: "var(--danger)", background: "var(--s-danger)", cursor: "pointer", marginLeft: "auto" }}>Kill Session</button>
            </div>
          </div>
        </div>

        {/* ACTIVITY */}
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 16, display: "flex", alignItems: "center", gap: 14 }} className="pg-in">
          Recent Activity
          <div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          <button onClick={() => go("auditlogs")} style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", letterSpacing: ".08em" }}>View All →</button>
        </div>
        <div className="pg-in" style={{ display: "flex", flexDirection: "column", border: "1px solid var(--line)" }}>
          {MOCK_ACTIVITY.map((a) => <ActivityItem key={a.id} ico={a.ico} t={a.type} title={a.title} meta={a.meta} time={a.time} link={a.link} go={go} />)}
        </div>
      </div>
    </AppShell>
  );
}

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
    s: { borderColor: "rgba(74,222,128,.3)", background: "var(--s-success)" },
    w: { borderColor: "rgba(251,191,36,.3)", background: "var(--s-warning)" },
    i: { borderColor: "rgba(200,245,90,.2)", background: "var(--accent-dim)" },
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

/* ═══════════════════════════════════════════════════════════════
   DEVICES
═══════════════════════════════════════════════════════════════ */
function Devices({ go, toast, auth, onLogout }) {
  const [showRevoke, setShowRevoke] = useState(false);
  const [revTarget, setRevTarget] = useState(null); // { id, name }
  const [countdown, setCountdown] = useState(3);
  const [canRevoke, setCanRevoke] = useState(false);
  const [devices, setDevices] = useState([]);
  const [loadingDevices, setLoadingDevices] = useState(false);
  const timerRef = useRef(null);

  useEffect(() => {
    if (!auth?.token) return;
    setLoadingDevices(true);
    API.listDevices(auth.token)
      .then(data => setDevices(data))
      .catch(e => toast("Failed to load devices: " + e.message, "danger"))
      .finally(() => setLoadingDevices(false));
  }, [auth?.token]);

  const openRevoke = (id, name) => {
    setRevTarget({ id, name }); setShowRevoke(true); setCountdown(3); setCanRevoke(false);
    timerRef.current = setInterval(() => {
      setCountdown(c => {
        if (c <= 1) { clearInterval(timerRef.current); setCanRevoke(true); return 0; }
        return c - 1;
      });
    }, 1000);
  };
  const closeRevoke = () => { clearInterval(timerRef.current); setShowRevoke(false); };
  const doRevoke = async () => {
    if (!revTarget || !auth?.token) return;
    closeRevoke();
    try {
      await API.revokeDevice(auth.token, revTarget.id);
      setDevices(prev => prev.map(d => d.id === revTarget.id ? { ...d, status: "revoked" } : d));
      toast(`${revTarget.name} revoked — access blocked`, "danger");
    } catch (e) {
      toast("Revoke failed: " + e.message, "danger");
    }
  };

  const deviceCards = auth?.token ? devices : [];

  return (
    <AppShell active="devices" go={go} auth={auth} onLogout={onLogout}>
      {showRevoke && (
        <div onClick={e => { if (e.target === e.currentTarget) closeRevoke(); }} style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.88)", backdropFilter: "blur(14px)", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <div className="modal-anim" style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 48, maxWidth: 440, width: "90%", position: "relative" }}>
            <button onClick={closeRevoke} style={{ position: "absolute", top: 18, right: 18, background: "none", border: "none", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 16, cursor: "pointer" }}>×</button>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>// Destructive Action</div>
            <h3 style={{ fontFamily: "var(--display)", fontSize: 44, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 14 }}>Revoke<br />Device?</h3>
            <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, marginBottom: 24, fontWeight: 300 }}>
              <strong style={{ color: "var(--paper)" }}>{revTarget?.name}</strong> will lose all access immediately. Cannot be undone without re-enrollment.
            </p>
            <div style={{ background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)", padding: 14, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginBottom: 24 }}>⚠ DEVICE BLOCKED WITHIN 500MS OF CONFIRMATION</div>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 24 }}>
              <div style={{ width: 38, height: 38, position: "relative", flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", border: "2px solid var(--danger)", borderRadius: "50%" }}>
                <span style={{ fontFamily: "var(--display)", fontSize: 14, color: "var(--danger)" }}>{countdown}</span>
              </div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", color: "var(--muted)", lineHeight: 1.6 }}>Confirm button activates in {countdown}s —<br />mandatory delay for destructive operations</div>
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <BtnO onClick={closeRevoke} style={{ padding: "8px 16px", fontSize: 9 }}>Cancel</BtnO>
              <button onClick={doRevoke} disabled={!canRevoke} style={{ display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: canRevoke ? "pointer" : "not-allowed", opacity: canRevoke ? 1 : .5, transition: "all .25s" }}>Revoke Device</button>
            </div>
          </div>
        </div>
      )}

      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Devices</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Enrolled hardware · Trust registry</div></div>
        <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Enroll New</BtnF>
      </div>
      <div style={{ padding: "36px 44px 60px" }}>
        {!auth?.token && (
          <div style={{ padding: 28, border: "1px solid var(--line)", background: "var(--ink-2)", textAlign: "center", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>
            Sign in to view your enrolled devices.{" "}
            <button onClick={() => go("login")} style={{ color: "var(--accent)", background: "none", border: "none", cursor: "pointer", fontFamily: "var(--mono)", fontSize: 11 }}>Sign in →</button>
          </div>
        )}
        {auth?.token && loadingDevices && (
          <div style={{ padding: 28, border: "1px solid var(--line)", background: "var(--ink-2)", textAlign: "center", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>Loading devices...</div>
        )}
        {auth?.token && !loadingDevices && deviceCards.length === 0 && (
          <div style={{ padding: 28, border: "1px solid var(--line)", background: "var(--ink-2)", textAlign: "center", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>
            No devices enrolled yet.{" "}
            <button onClick={() => go("register")} style={{ color: "var(--accent)", background: "none", border: "none", cursor: "pointer", fontFamily: "var(--mono)", fontSize: 11 }}>Enroll one →</button>
          </div>
        )}
        {auth?.token && !loadingDevices && deviceCards.length > 0 && (
          <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(290px,1fr))", gap: 1, background: "var(--line)", border: "1px solid var(--line)" }}>
            {deviceCards.map(d => {
              const name = d.nickname || `Device-${d.id.slice(0, 8)}`;
              return <DeviceCard key={d.id}
                ico="💻" name={name} type="WebAuthn · Passkey"
                status={d.status} enrolled="—" last="—"
                fp={d.id.slice(0, 8) + "..."}
                canRevoke={d.status !== "revoked"}
                onRevoke={() => openRevoke(d.id, name)} toast={toast} />;
            })}
          </div>
        )}
      </div>
    </AppShell>
  );
}

function DeviceCard({ ico, name, type, status, enrolled, last, fp, onRevoke, toast, canRevoke = true }) {
  const [hov, setHov] = useState(false);
  const ringC = status === "active" ? "var(--success)" : status === "revoked" ? "var(--danger)" : "var(--muted2)";
  const statusC = status === "active" ? { background: "var(--s-success)", color: "var(--success)" } : status === "revoked" ? { background: "var(--s-danger)", color: "var(--danger)" } : { background: "rgba(90,85,80,.15)", color: "var(--muted)" };
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: 28, transition: "background .25s", position: "relative", overflow: "hidden", opacity: status === "inactive" ? .75 : 1 }}>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ display: "flex", alignItems: "flex-start", gap: 16, marginBottom: 20 }}>
        <div className="dmodel" style={{ width: 52, height: 76, background: "linear-gradient(160deg,var(--ink-3),#080808)", border: "1px solid var(--line2)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 26, flexShrink: 0, position: "relative", opacity: status === "inactive" ? .5 : 1 }}>
          {ico}
          <div style={{ position: "absolute", bottom: -3, left: "50%", transform: "translateX(-50%)", width: 40, height: 4, background: ringC, boxShadow: status === "active" ? `0 0 10px ${ringC}` : "none" }} />
        </div>
        <div>
          <div style={{ fontSize: 15, fontWeight: 500, marginBottom: 4 }}>{name}</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--muted)" }}>{type}</div>
          <div style={{ display: "inline-flex", alignItems: "center", gap: 5, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "3px 8px", marginTop: 8, ...statusC }}>● {status.charAt(0).toUpperCase() + status.slice(1)}</div>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 20, paddingTop: 16, borderTop: "1px solid var(--line)" }}>
        {[["Enrolled", enrolled], ["Last active", last], ["Fingerprint", fp]].map(([l, v]) => (
          <div key={l} style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--muted2)" }}>{l}</span>
            <span style={{ fontSize: 12, fontFamily: l === "Fingerprint" ? "var(--mono)" : "var(--body)" }}>{v}</span>
          </div>
        ))}
      </div>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        {canRevoke && (
          <button onClick={onRevoke} style={{ display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: "pointer", transition: "all .25s" }}>Revoke</button>
        )}
        {!canRevoke && (
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--danger)", padding: "8px 16px", background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)" }}>Revoked</div>
        )}
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   RECOVERY
═══════════════════════════════════════════════════════════════ */
function Recovery({ go, toast, auth, onLogout }) {
  const [secs, setSecs] = useState(86400 - 76);
  const [done, setDone] = useState(false);
  const intRef = useRef(null);

  useEffect(() => {
    intRef.current = setInterval(() => setSecs(s => s > 0 ? s - 1 : 0), 1000);
    return () => clearInterval(intRef.current);
  }, []);

  const h = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60), s = secs % 60;
  const timeStr = `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  const pct = secs / 86400;
  const circ = 2 * Math.PI * 65;
  const offset = circ * (1 - pct);

  const complete = () => {
    clearInterval(intRef.current);
    setDone(true);
    toast("Recovery completed — new device enrolled", "success");
  };

  return (
    <AppShell active="recovery" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Recovery</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Time-lock zero-trust protocol</div>
      </div>
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", padding: "60px 24px" }}>
        <div className="pg-in" style={{ display: "flex", flexDirection: "column", alignItems: "center", textAlign: "center" }}>
          <div style={{
            width: 160, height: 160, background: "linear-gradient(160deg,var(--ink-3),#060606)",
            border: `2px solid ${done ? "rgba(74,222,128,.5)" : "rgba(251,191,36,.35)"}`,
            display: "flex", alignItems: "center", justifyContent: "center", fontSize: 64, marginBottom: 36,
            boxShadow: done ? "0 0 80px rgba(74,222,128,.2)" : "0 0 60px rgba(251,191,36,.12)",
            animation: "vFloat 5s ease-in-out infinite", transition: "all .8s"
          }}>{done ? "🔓" : "🔐"}</div>

          <div style={{ position: "relative", marginBottom: 32 }}>
            <svg width="168" height="168" viewBox="0 0 168 168" style={{ transform: "rotate(-90deg)" }}>
              <circle fill="none" stroke="var(--ink-3)" strokeWidth={4} cx={84} cy={84} r={65} />
              <circle fill="none" stroke={done ? "var(--success)" : "var(--warning)"} strokeWidth={4} strokeLinecap="round"
                strokeDasharray={circ} strokeDashoffset={done ? 0 : offset} cx={84} cy={84} r={65} style={{ transition: "stroke-dashoffset 1s linear, stroke .5s" }} />
            </svg>
            <div style={{ position: "absolute", top: "50%", left: "50%", transform: "translate(-50%,-50%)", textAlign: "center" }}>
              <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em" }}>{done ? "00:00:00" : timeStr}</div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginTop: 2 }}>remaining</div>
            </div>
          </div>

          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 12 }}>
            {done ? "✅ Recovery Complete" : "⏳ Time-Lock Active"}
          </div>
          <p style={{ fontSize: 13, color: "var(--muted)", maxWidth: 480, lineHeight: 1.75, fontWeight: 300, marginBottom: 40 }}>
            {done
              ? "Time lock expired. New device has been enrolled. All existing devices notified of the completed recovery."
              : "Your recovery request is in progress. A mandatory 24-hour wait protects against unauthorized device enrollment. All trusted devices have been notified."}
          </p>
          <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
            {!done && <BtnF onClick={complete}>Simulate Completion</BtnF>}
            <BtnO onClick={() => toast("Recovery cancelled by trusted device", "danger")}>Cancel</BtnO>
          </div>
        </div>
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   ADMIN
═══════════════════════════════════════════════════════════════ */
function NetworkCanvas() {
  const ref = useRef(null);
  useEffect(() => {
    const canvas = ref.current; if (!canvas) return;
    const ctx = canvas.getContext("2d");
    canvas.width = canvas.offsetWidth; canvas.height = 220;
    const W = canvas.width, H = canvas.height;
    const nodes = [
      { x: W * .18, y: H * .5, label: "MacBook Pro", c: "#4ADE80" },
      { x: W * .42, y: H * .2, label: "iPhone 15", c: "#4ADE80" },
      { x: W * .75, y: H * .45, label: "iPad Air", c: "#4ADE80" },
      { x: W * .5, y: H * .72, label: "Server", c: "#C8F55A" },
      { x: W * .3, y: H * .38, label: "Work Desktop", c: "#5A5550" },
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

function Admin({ go, toast, auth, onLogout }) {
  return (
    <AppShell active="admin" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Admin</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>System management · Analytics</div>
      </div>
      <div style={{ padding: "36px 44px 60px" }}>
        <div className="pg-in" style={{ border: "1px solid var(--line)", background: "var(--ink-2)", marginBottom: 28, overflow: "hidden" }}>
          <div style={{ padding: "18px 22px", borderBottom: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)" }}>Network Topology — Live Trust Graph</span>
            <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--success)", letterSpacing: ".06em" }}>● 3 nodes active</span>
          </div>
          <NetworkCanvas />
        </div>
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 28 }}>
          {[
            { ico: "📜", t: "Policy Editor", b: "Visual rule builder for access policies. Configure deny-by-default and step-up verification rules.", link: "policy" },
            { ico: "👥", t: "User Directory", b: "Searchable user list with device counts, last active timestamps, and trust scores.", link: "rbac" },
            { ico: "🗂", t: "Audit Log", b: "Full cryptographic event stream. Export to CSV or JSON for compliance reporting.", link: "auditlogs" },
            { ico: "📊", t: "System Health", b: "Uptime monitoring, response times, WebAuthn success rates, error dashboards.", link: "risk" },
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

function AdminCard({ ico, t, b, link, go, toast }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)} onClick={() => link ? go(link) : toast(t, "info")}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", position: "relative", overflow: "hidden" }}>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ fontSize: 28, marginBottom: 18 }}>{ico}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>{t}</div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{b}</div>
      {link && <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", marginTop: 14, letterSpacing: ".08em" }}>Open →</div>}
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   AUDIT LOGS
═══════════════════════════════════════════════════════════════ */
function AuditLogs({ go, toast, auth, onLogout }) {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("ALL");
  const filtered = MOCK_AUDIT_LOGS.filter(r =>
    (filter === "ALL" || r.action === filter) &&
    (search === "" || r.actor.includes(search) || r.action.includes(search.toUpperCase()) || r.loc.toLowerCase().includes(search.toLowerCase()))
  );
  const typeColor = { success: "var(--success)", danger: "var(--danger)", warning: "var(--warning)", info: "var(--accent)" };
  const exportCSV = () => {
    const rows = [["Actor","Action","Device","IP","Location","Time"], ...MOCK_AUDIT_LOGS.map(r => [r.actor,r.action,r.device,r.ip,r.loc,r.time])];
    const csv = rows.map(r => r.join(",")).join("\n");
    const a = document.createElement("a"); a.href = "data:text/csv," + encodeURIComponent(csv); a.download = "crypton-audit.csv"; a.click();
    toast("Audit log exported as CSV", "success");
  };
  return (
    <AppShell active="auditlogs" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Audit Logs</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Full cryptographic event stream</div></div>
        <BtnF onClick={exportCSV} style={{ padding: "8px 16px", fontSize: 9 }}>↓ Export CSV</BtnF>
      </div>
      <div style={{ padding: "28px 44px 60px" }}>
        <div style={{ display: "flex", gap: 12, marginBottom: 20, flexWrap: "wrap" }}>
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search actor, action, location..."
            style={{ flex: 1, minWidth: 200, padding: "10px 14px", background: "var(--ink-3)", border: "1px solid var(--line2)", color: "var(--paper)", fontFamily: "var(--body)", fontSize: 13, outline: "none" }} />
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {["ALL","LOGIN","BLOCKED","REVOKE","POLICY"].map(f => {
              const match = f === "ALL" ? "ALL" : f === "BLOCKED" ? "LOGIN_BLOCKED" : f === "REVOKE" ? "DEVICE_REVOKE" : f === "POLICY" ? "POLICY_UPDATE" : f;
              return <button key={f} onClick={() => setFilter(match)} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "8px 14px", border: `1px solid ${filter === match ? "var(--accent)" : "var(--line2)"}`, background: filter === match ? "var(--accent-dim)" : "none", color: filter === match ? "var(--accent)" : "var(--muted)", cursor: "pointer", transition: "all .2s" }}>{f}</button>;
            })}
          </div>
        </div>
        <div style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
          <div style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1.2fr 1.2fr 1.5fr 0.8fr", padding: "10px 16px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
            {["Actor","Action","Device","IP","Location","Time"].map(h => <span key={h} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted2)" }}>{h}</span>)}
          </div>
          {filtered.length === 0 && <div style={{ padding: "32px 16px", textAlign: "center", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>No results found</div>}
          {filtered.map((r, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1.2fr 1.2fr 1.5fr 0.8fr", padding: "13px 16px", borderBottom: i < filtered.length - 1 ? "1px solid var(--line)" : "none", background: i % 2 === 0 ? "var(--ink-2)" : "var(--ink)", alignItems: "center", transition: "background .15s" }}
              onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = i % 2 === 0 ? "var(--ink-2)" : "var(--ink)"}>
              <span style={{ fontSize: 12 }}>{r.actor}</span>
              <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: typeColor[r.type], letterSpacing: ".05em" }}>{r.action}</span>
              <span style={{ fontSize: 12, color: "var(--muted)" }}>{r.device}</span>
              <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{r.ip}</span>
              <span style={{ fontSize: 12, color: "var(--muted)" }}>{r.loc}</span>
              <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{r.time}</span>
            </div>
          ))}
        </div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)", marginTop: 12, letterSpacing: ".06em" }}>{filtered.length} of {MOCK_AUDIT_LOGS.length} events shown</div>
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   RISK & THREAT INTELLIGENCE
═══════════════════════════════════════════════════════════════ */
function RiskIntel({ go, toast, auth, onLogout }) {
  const [selected, setSelected] = useState(0);
  const [scanning, setScanning] = useState(false);
  const [scores, setScores] = useState(MOCK_RISK_USERS.map(u => u.score));
  const user = MOCK_RISK_USERS[selected];
  const score = scores[selected];
  const levelColor = s => s >= 70 ? "var(--danger)" : s >= 40 ? "var(--warning)" : "var(--success)";
  const levelLabel = s => s >= 70 ? "HIGH" : s >= 40 ? "MEDIUM" : "LOW";
  const levelBg = s => s >= 70 ? "var(--s-danger)" : s >= 40 ? "var(--s-warning)" : "var(--s-success)";
  const rescan = () => {
    setScanning(true);
    toast("Running threat intelligence scan...", "info");
    setTimeout(() => {
      setScores(s => s.map((v, i) => i === selected ? Math.max(5, Math.min(95, v + (Math.random() > 0.5 ? 1 : -1) * Math.floor(Math.random() * 12))) : v));
      setScanning(false);
      toast("Scan complete — scores updated", "success");
    }, 2000);
  };
  return (
    <AppShell active="risk" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Risk Intelligence</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Behavioral anomaly · Geo-velocity · Threat scoring</div></div>
        <BtnF onClick={rescan} style={{ padding: "8px 16px", fontSize: 9, opacity: scanning ? .6 : 1 }}>{scanning ? "Scanning..." : "↻ Re-scan"}</BtnF>
      </div>
      <div style={{ padding: "28px 44px 60px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 20, marginBottom: 24 }}>
          <div style={{ border: "1px solid var(--line)", background: "var(--ink-2)", overflow: "hidden" }}>
            <div style={{ padding: "14px 18px", borderBottom: "1px solid var(--line)", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)" }}>Select User</div>
            {MOCK_RISK_USERS.map((u, i) => (
              <div key={i} onClick={() => setSelected(i)} style={{ padding: "14px 18px", cursor: "pointer", background: selected === i ? "rgba(200,245,90,.05)" : "none", borderLeft: selected === i ? "2px solid var(--accent)" : "2px solid transparent", borderBottom: i < MOCK_RISK_USERS.length - 1 ? "1px solid var(--line)" : "none", display: "flex", justifyContent: "space-between", alignItems: "center", transition: "background .15s" }}>
                <div>
                  <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 3 }}>{u.user}</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", letterSpacing: ".05em" }}>{u.device} · {u.loc}</div>
                </div>
                <div style={{ fontFamily: "var(--display)", fontSize: 22, color: levelColor(scores[i]) }}>{scores[i]}</div>
              </div>
            ))}
          </div>
          <div style={{ border: `1px solid ${levelColor(score)}30`, background: "var(--ink-2)", padding: 24, position: "relative", overflow: "hidden" }}>
            <div style={{ position: "absolute", top: 0, left: 0, right: 0, height: 3, background: `linear-gradient(90deg, ${levelColor(score)}, transparent)` }} />
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>Risk Profile</div>
            <div style={{ fontSize: 13, marginBottom: 16 }}>{user.user}</div>
            <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 20 }}>
              <div style={{ fontFamily: "var(--display)", fontSize: 72, lineHeight: 1, color: levelColor(score), transition: "color .5s" }}>{score}</div>
              <div>
                <div style={{ display: "inline-flex", padding: "4px 10px", background: levelBg(score), color: levelColor(score), fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", marginBottom: 6 }}>{levelLabel(score)} RISK</div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)" }}>{user.ip} · {user.time}</div>
              </div>
            </div>
            <div style={{ height: 4, background: "var(--line2)", marginBottom: 20, overflow: "hidden" }}>
              <div style={{ height: "100%", width: `${score}%`, background: `linear-gradient(90deg, var(--success), ${score > 60 ? "var(--warning)" : "var(--success)"}, ${score > 75 ? "var(--danger)" : "transparent"})`, transition: "width 1s cubic-bezier(.16,1,.3,1)" }} />
            </div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>Risk Factors</div>
            {user.reasons.map((r, i) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 7 }}>
                <div style={{ width: 5, height: 5, borderRadius: "50%", background: levelColor(score), flexShrink: 0 }} />
                <span style={{ fontSize: 12, color: "var(--muted)" }}>{r}</span>
              </div>
            ))}
            <div style={{ display: "flex", gap: 8, marginTop: 20 }}>
              <BtnF onClick={() => toast(`Session killed for ${user.user}`, "danger")} style={{ padding: "8px 14px", fontSize: 9 }}>Kill Session</BtnF>
              <BtnO onClick={() => toast(`${user.user} flagged for review`, "warning")} style={{ padding: "8px 14px", fontSize: 9 }}>Flag User</BtnO>
            </div>
          </div>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 24 }}>
          {[
            { label: "Geo-Velocity", val: "8,400 km/h", sub: "Impossible travel detected", c: "var(--danger)" },
            { label: "Device Reputation", val: "Unknown", sub: "First-time device fingerprint", c: "var(--warning)" },
            { label: "Behavioral Score", val: "42 / 100", sub: "Deviation from baseline", c: "var(--warning)" },
          ].map((c, i) => (
            <div key={i} style={{ background: "var(--ink-2)", padding: "22px 20px" }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>{c.label}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 28, color: c.c, letterSpacing: ".04em", marginBottom: 6 }}>{c.val}</div>
              <div style={{ fontSize: 11, color: "var(--muted2)" }}>{c.sub}</div>
            </div>
          ))}
        </div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 14, display: "flex", alignItems: "center", gap: 14 }}>
          Suspicious Activity Feed <div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <div style={{ border: "1px solid var(--line)" }}>
          {MOCK_RISK_FEED.map((f, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 14, padding: "13px 18px", borderBottom: i < MOCK_RISK_FEED.length - 1 ? "1px solid var(--line)" : "none", background: "var(--ink-2)", transition: "background .15s" }}
              onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
              <span style={{ fontSize: 16 }}>{f.ico}</span>
              <span style={{ flex: 1, fontSize: 13 }}>{f.msg}</span>
              <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{f.time}</span>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   SESSIONS
═══════════════════════════════════════════════════════════════ */
function Sessions({ go, toast, auth, onLogout }) {
  const [sessions, setSessions] = useState(MOCK_SESSIONS);
  const kill = id => { setSessions(s => s.filter(x => x.id !== id)); toast("Session terminated immediately", "danger"); };
  const killAll = () => { setSessions([]); toast("All sessions terminated — users signed out", "danger"); };
  return (
    <AppShell active="sessions" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Sessions</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Active sessions · Real-time monitor</div></div>
        <button onClick={killAll} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: "pointer" }}>⚡ Kill All Sessions</button>
      </div>
      <div style={{ padding: "28px 44px 60px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 24 }}>
          {[
            { l: "Active Sessions", v: sessions.filter(s => s.active).length.toString(), c: "var(--success)" },
            { l: "Idle Sessions", v: sessions.filter(s => !s.active).length.toString(), c: "var(--warning)" },
            { l: "Total Users Online", v: [...new Set(sessions.map(s => s.user))].length.toString(), c: "var(--accent)" },
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
        ) : (
          <div style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
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
        )}
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   RBAC — USERS & ROLES
═══════════════════════════════════════════════════════════════ */
const ROLES = ["Super Admin", "Admin", "Security Analyst", "Viewer"];
const ROLE_COLORS = { "Super Admin": "var(--danger)", "Admin": "var(--accent)", "Security Analyst": "var(--warning)", "Viewer": "var(--muted)" };
const ROLE_PERMS = {
  "Super Admin": ["Revoke devices", "Change roles", "Update policies", "View audit logs", "Kill sessions", "Emergency lockdown"],
  "Admin": ["Revoke devices", "Change roles", "Update policies", "View audit logs", "Kill sessions"],
  "Security Analyst": ["View audit logs", "View risk scores", "Flag users"],
  "Viewer": ["View dashboard", "View devices (read-only)"],
};
function RBAC({ go, toast, auth, onLogout }) {
  const [users, setUsers] = useState(MOCK_USERS);
  const [selectedUser, setSelectedUser] = useState(null);
  const changeRole = (id, newRole) => {
    setUsers(u => u.map(x => x.id === id ? { ...x, role: newRole } : x));
    toast(`Role updated to ${newRole}`, "success");
    setSelectedUser(null);
  };
  return (
    <AppShell active="rbac" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Users & Roles</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Role-based access control · Least privilege</div></div>
        <BtnF onClick={() => toast("Invite flow — full implementation pending", "info")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Invite User</BtnF>
      </div>
      <div style={{ padding: "28px 44px 60px" }}>
        {selectedUser && (
          <div onClick={() => setSelectedUser(null)} style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.85)", backdropFilter: "blur(12px)", display: "flex", alignItems: "center", justifyContent: "center" }}>
            <div onClick={e => e.stopPropagation()} className="modal-anim" style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 40, maxWidth: 420, width: "90%", position: "relative" }}>
              <button onClick={() => setSelectedUser(null)} style={{ position: "absolute", top: 16, right: 16, background: "none", border: "none", color: "var(--muted)", fontSize: 16, cursor: "pointer", fontFamily: "var(--mono)" }}>×</button>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 14 }}>// Assign Role</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 32, letterSpacing: ".04em", textTransform: "uppercase", lineHeight: .95, marginBottom: 20 }}>{selectedUser.name}</div>
              <div style={{ fontSize: 12, color: "var(--muted)", marginBottom: 24 }}>{selectedUser.email}</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {ROLES.map(r => (
                  <button key={r} onClick={() => changeRole(selectedUser.id, r)} style={{ padding: "12px 16px", display: "flex", alignItems: "center", justifyContent: "space-between", background: selectedUser.role === r ? "rgba(200,245,90,.07)" : "var(--ink-3)", border: `1px solid ${selectedUser.role === r ? "var(--accent)" : "var(--line)"}`, cursor: "pointer", transition: "all .2s" }}>
                    <span style={{ fontSize: 13, color: "var(--paper)" }}>{r}</span>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: ROLE_COLORS[r] }}>{ROLE_PERMS[r].length} permissions {selectedUser.role === r ? "· current" : ""}</span>
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
        <div style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
          <div style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1fr 0.8fr 0.8fr", padding: "10px 16px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
            {["User","Role","Devices","Last Active","Action"].map(h => <span key={h} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted2)" }}>{h}</span>)}
          </div>
          {users.map((u, i) => (
            <div key={u.id} style={{ display: "grid", gridTemplateColumns: "2fr 1.5fr 1fr 0.8fr 0.8fr", padding: "14px 16px", borderBottom: i < users.length - 1 ? "1px solid var(--line)" : "none", background: "var(--ink-2)", alignItems: "center", transition: "background .15s" }}
              onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <div style={{ width: 30, height: 30, background: "var(--ink-3)", border: "1px solid var(--line2)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--display)", fontSize: 13, color: "var(--accent)", flexShrink: 0 }}>{u.avatar}</div>
                <div><div style={{ fontSize: 13, fontWeight: 500 }}>{u.name}</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)" }}>{u.email}</div></div>
              </div>
              <div style={{ display: "inline-flex", padding: "3px 10px", background: `${ROLE_COLORS[u.role]}15`, color: ROLE_COLORS[u.role], fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", width: "fit-content" }}>{u.role}</div>
              <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>{u.devices} enrolled</span>
              <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)" }}>{u.lastActive}</span>
              <button onClick={() => setSelectedUser(u)} style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", background: "var(--accent-dim)", border: "1px solid rgba(200,245,90,.2)", padding: "5px 10px", cursor: "pointer" }}>Edit Role</button>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   POLICY ENGINE
═══════════════════════════════════════════════════════════════ */
function PolicyEngine({ go, toast, auth, onLogout }) {
  const [policies, setPolicies] = useState(MOCK_POLICIES);
  const [threshold, setThreshold] = useState(70);
  const [trustDays, setTrustDays] = useState(30);
  const toggle = id => {
    setPolicies(p => p.map(x => x.id === id ? { ...x, active: !x.active } : x));
    const pol = policies.find(x => x.id === id);
    toast(`Policy "${pol.label}" ${pol.active ? "disabled" : "enabled"}`, pol.active ? "warning" : "success");
  };
  const catColor = { geo: "var(--accent)", risk: "var(--danger)", network: "var(--warning)", device: "#7EC8E3", auth: "var(--success)" };
  const cats = [...new Set(policies.map(p => p.cat))];
  return (
    <AppShell active="policy" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Policy Engine</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Zero-trust rules · Adaptive enforcement</div></div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--success)", letterSpacing: ".06em", display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)", boxShadow: "0 0 8px var(--success)" }} />
          {policies.filter(p => p.active).length} / {policies.length} policies active
        </div>
      </div>
      <div style={{ padding: "28px 44px 60px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 28 }}>
          {[
            { label: "Risk Score Threshold", val: threshold, set: setThreshold, unit: "", desc: "Step-up auth triggered above this score", min: 10, max: 95 },
            { label: "Device Trust Duration", val: trustDays, set: setTrustDays, unit: " days", desc: "Days before re-verification required", min: 1, max: 90 },
          ].map(s => (
            <div key={s.label} style={{ background: "var(--ink-2)", border: "1px solid var(--line)", padding: "20px 22px" }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{s.label}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 40, color: "var(--accent)", marginBottom: 12 }}>{s.val}{s.unit}</div>
              <input type="range" min={s.min} max={s.max} value={s.val} onChange={e => { s.set(Number(e.target.value)); toast(`${s.label} set to ${e.target.value}${s.unit}`, "info"); }}
                style={{ width: "100%", accentColor: "var(--accent)", cursor: "pointer" }} />
              <div style={{ fontSize: 11, color: "var(--muted2)", marginTop: 8 }}>{s.desc}</div>
            </div>
          ))}
        </div>
        {cats.map(cat => (
          <div key={cat} style={{ marginBottom: 20 }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: catColor[cat], marginBottom: 10, display: "flex", alignItems: "center", gap: 10 }}>
              <div style={{ width: 8, height: 8, borderRadius: "50%", background: catColor[cat] }} />{cat.toUpperCase()} POLICIES
            </div>
            <div style={{ border: "1px solid var(--line)", overflow: "hidden" }}>
              {policies.filter(p => p.cat === cat).map((p, i, arr) => (
                <div key={p.id} style={{ display: "flex", alignItems: "center", gap: 16, padding: "16px 18px", borderBottom: i < arr.length - 1 ? "1px solid var(--line)" : "none", background: "var(--ink-2)", transition: "background .15s" }}
                  onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
                  <div onClick={() => toggle(p.id)} style={{ width: 36, height: 20, borderRadius: 10, background: p.active ? "var(--accent)" : "var(--ink-3)", border: `1px solid ${p.active ? "var(--accent)" : "var(--line2)"}`, cursor: "pointer", position: "relative", flexShrink: 0, transition: "background .25s" }}>
                    <div style={{ position: "absolute", top: 2, left: p.active ? 17 : 2, width: 14, height: 14, borderRadius: "50%", background: p.active ? "var(--ink)" : "var(--muted)", transition: "left .25s" }} />
                  </div>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 3 }}>{p.label}</div>
                    <div style={{ fontSize: 11, color: "var(--muted)" }}>{p.desc}</div>
                  </div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: p.active ? "var(--success)" : "var(--muted2)", letterSpacing: ".06em" }}>{p.active ? "ACTIVE" : "INACTIVE"}</div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   ORG SETTINGS
═══════════════════════════════════════════════════════════════ */
function OrgSettings({ go, toast, auth, onLogout }) {
  const [orgName, setOrgName] = useState(MOCK_ORG.orgName);
  const [domain, setDomain] = useState(MOCK_ORG.domain);
  const [mfa, setMfa] = useState(MOCK_ORG.mfaEnforced);
  const [sessionTimeout, setSessionTimeout] = useState(MOCK_ORG.sessionTimeoutHours);
  const [countries, setCountries] = useState(MOCK_ORG.allowedCountries);
  const [domainVerified, setDomainVerified] = useState(MOCK_ORG.domainVerified);
  const allCountries = ["US", "CA", "GB", "DE", "AU", "FR", "JP", "SG", "IN", "BR", "NL", "SE"];
  const toggleCountry = c => setCountries(cs => cs.includes(c) ? cs.filter(x => x !== c) : [...cs, c]);
  const save = () => toast("Organization settings saved", "success");
  const verifyDomain = () => { setDomainVerified(true); toast(`Domain ${domain} verified ✓`, "success"); };
  return (
    <AppShell active="orgsettings" go={go} auth={auth} onLogout={onLogout}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Org Settings</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Organization configuration · Multi-tenant</div></div>
        <BtnF onClick={save} style={{ padding: "8px 16px", fontSize: 9 }}>Save Changes</BtnF>
      </div>
      <div style={{ padding: "28px 44px 60px", display: "flex", flexDirection: "column", gap: 20, maxWidth: 720 }}>
        <div style={{ background: "var(--ink-2)", border: "1px solid var(--line)", padding: "24px 26px" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 20, paddingBottom: 14, borderBottom: "1px solid var(--line)" }}>Organization Identity</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
            <div>
              <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Org Name</label>
              <input value={orgName} onChange={e => setOrgName(e.target.value)} style={{ width: "100%", padding: "11px 14px", background: "var(--ink-3)", border: "1px solid var(--line2)", color: "var(--paper)", fontFamily: "var(--body)", fontSize: 13, outline: "none" }} />
            </div>
            <div>
              <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Primary Domain</label>
              <div style={{ display: "flex", gap: 8 }}>
                <input value={domain} onChange={e => { setDomain(e.target.value); setDomainVerified(false); }} style={{ flex: 1, padding: "11px 14px", background: "var(--ink-3)", border: "1px solid var(--line2)", color: "var(--paper)", fontFamily: "var(--body)", fontSize: 13, outline: "none" }} />
                <button onClick={verifyDomain} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "0 14px", background: domainVerified ? "var(--s-success)" : "var(--accent-dim)", color: domainVerified ? "var(--success)" : "var(--accent)", border: `1px solid ${domainVerified ? "rgba(74,222,128,.3)" : "rgba(200,245,90,.3)"}`, cursor: "pointer", whiteSpace: "nowrap" }}>{domainVerified ? "✓ Verified" : "Verify"}</button>
              </div>
            </div>
          </div>
        </div>
        <div style={{ background: "var(--ink-2)", border: "1px solid var(--line)", padding: "24px 26px" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 20, paddingBottom: 14, borderBottom: "1px solid var(--line)" }}>Security Policies</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div>
                <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 3 }}>Enforce MFA on all logins</div>
                <div style={{ fontSize: 11, color: "var(--muted)" }}>Require additional factor for every authentication attempt</div>
              </div>
              <div onClick={() => { setMfa(!mfa); toast(`MFA ${mfa ? "disabled" : "enabled"}`, mfa ? "warning" : "success"); }} style={{ width: 40, height: 22, borderRadius: 11, background: mfa ? "var(--accent)" : "var(--ink-3)", border: `1px solid ${mfa ? "var(--accent)" : "var(--line2)"}`, cursor: "pointer", position: "relative", transition: "background .25s", flexShrink: 0 }}>
                <div style={{ position: "absolute", top: 2, left: mfa ? 19 : 2, width: 16, height: 16, borderRadius: "50%", background: mfa ? "var(--ink)" : "var(--muted)", transition: "left .25s" }} />
              </div>
            </div>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                <div style={{ fontSize: 13, fontWeight: 500 }}>Session Timeout</div>
                <span style={{ fontFamily: "var(--display)", fontSize: 20, color: "var(--accent)" }}>{sessionTimeout}h</span>
              </div>
              <input type="range" min={1} max={24} value={sessionTimeout} onChange={e => setSessionTimeout(Number(e.target.value))} style={{ width: "100%", accentColor: "var(--accent)", cursor: "pointer" }} />
              <div style={{ display: "flex", justifyContent: "space-between", marginTop: 4 }}>
                <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)" }}>1h</span>
                <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)" }}>24h</span>
              </div>
            </div>
          </div>
        </div>
        <div style={{ background: "var(--ink-2)", border: "1px solid var(--line)", padding: "24px 26px" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 8, paddingBottom: 14, borderBottom: "1px solid var(--line)" }}>Allowed Countries</div>
          <div style={{ fontSize: 12, color: "var(--muted)", marginBottom: 16 }}>Auth attempts from countries not listed here will be automatically blocked.</div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            {allCountries.map(c => (
              <button key={c} onClick={() => { toggleCountry(c); toast(`${c} ${countries.includes(c) ? "removed from" : "added to"} allowlist`, "info"); }} style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: ".06em", padding: "7px 14px", border: `1px solid ${countries.includes(c) ? "var(--accent)" : "var(--line2)"}`, background: countries.includes(c) ? "var(--accent-dim)" : "none", color: countries.includes(c) ? "var(--accent)" : "var(--muted)", cursor: "pointer", transition: "all .2s" }}>{c}</button>
            ))}
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)", marginTop: 14 }}>{countries.length} countries allowed · {allCountries.length - countries.length} blocked</div>
        </div>
      </div>
    </AppShell>
  );
}

/* ═══════════════════════════════════════════════════════════════
   ROOT
═══════════════════════════════════════════════════════════════ */
const ROUTES = {
  "/": "landing", "/landing": "landing", "/register": "register", "/login": "login",
  "/dashboard": "dashboard", "/devices": "devices", "/audit-logs": "auditlogs",
  "/risk": "risk", "/sessions": "sessions", "/users": "rbac", "/recovery": "recovery",
  "/policy": "policy", "/org": "orgsettings", "/admin": "admin",
};
const PAGE_TO_PATH = Object.fromEntries(Object.entries(ROUTES).map(([k, v]) => [v, k]));
function getPageFromPath() { return ROUTES[window.location.pathname] || "landing"; }

export default function App() {
  const [page, setPage] = useState(getPageFromPath);
  const [toasts, addToast] = useToasts();
  const [auth, setAuthState] = useState(() => {
    try {
      const token = localStorage.getItem('crypton_token');
      const username = localStorage.getItem('crypton_username');
      return token ? { token, username } : null;
    } catch { return null; }
  });

  const go = useCallback(id => {
    const path = PAGE_TO_PATH[id] || "/";
    window.history.pushState({ page: id }, "", path);
    setPage(id);
    window.scrollTo({ top: 0 });
  }, []);

  useEffect(() => {
    const onPop = () => setPage(getPageFromPath());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const toast = useCallback((msg, type = "info") => addToast(msg, type), [addToast]);

  const setAuth = useCallback((a) => {
    if (a?.token) localStorage.setItem('crypton_token', a.token);
    if (a?.username) localStorage.setItem('crypton_username', a.username);
    setAuthState(a);
  }, []);

  const logout = useCallback(() => {
    localStorage.removeItem('crypton_token');
    localStorage.removeItem('crypton_username');
    setAuthState(null);
    toast("Signed out", "info");
    go("home");
  }, [go, toast]);

  useEffect(() => {
    setTimeout(() => toast("CRYPTON — Zero passwords. Zero trust.", "info"), 800);
  }, []);

  const shell = { go, toast, auth, onLogout: logout };

  return (
    <>
      <FontLink />
      <div className="grain" />
      <ToastStack toasts={toasts} />

      {page === "home"        && <Home go={go} auth={auth} />}
      {page === "landing"     && <Landing go={go} toast={toast} />}
      {page === "register"    && <Register go={go} toast={toast} setAuth={setAuth} />}
      {page === "login"       && <Login go={go} toast={toast} setAuth={setAuth} />}
      {page === "dashboard"   && <Dashboard {...shell} />}
      {page === "devices"     && <Devices {...shell} />}
      {page === "recovery"    && <Recovery {...shell} />}
      {page === "admin"       && <Admin {...shell} />}
      {page === "auditlogs"   && <AuditLogs {...shell} />}
      {page === "risk"        && <RiskIntel {...shell} />}
      {page === "sessions"    && <Sessions {...shell} />}
      {page === "rbac"        && <RBAC {...shell} />}
      {page === "policy"      && <PolicyEngine {...shell} />}
      {page === "orgsettings" && <OrgSettings {...shell} />}
    </>
  );
}
