import { useState, useEffect, useRef } from "react";
import { api } from './api';
import { getToken } from './auth';
import { BtnF, BtnO } from './Buttons';
import AppShell from './AppShell';

function formatRelativeTime(isoString) {
  if (!isoString) return "—";
  const d = new Date(isoString);
  if (isNaN(d)) return isoString;
  const diff = Date.now() - d.getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

/** Parse user-agent string to extract OS and browser name */
function parseUserAgent(ua) {
  if (!ua || ua === 'unknown') return { os: 'Device', browser: '' };
  const ua_lower = ua.toLowerCase();
  
  // Detect OS
  let os = 'Device';
  if (ua_lower.includes('mac')) os = ua_lower.includes('iphone') ? 'iPhone' : ua_lower.includes('ipad') ? 'iPad' : 'macOS';
  else if (ua_lower.includes('win')) os = 'Windows';
  else if (ua_lower.includes('linux')) os = ua_lower.includes('android') ? 'Android' : 'Linux';
  else if (ua_lower.includes('iphone')) os = 'iPhone';
  else if (ua_lower.includes('ipad')) os = 'iPad';
  
  // Detect browser
  let browser = '';
  if (ua_lower.includes('safari') && !ua_lower.includes('chrome')) browser = 'Safari';
  else if (ua_lower.includes('chrome')) browser = 'Chrome';
  else if (ua_lower.includes('firefox')) browser = 'Firefox';
  else if (ua_lower.includes('edge')) browser = 'Edge';
  else if (ua_lower.includes('opera')) browser = 'Opera';
  else browser = 'WebAuthn';
  
  return { os, browser };
}

export default function Devices({ go, toast }) {
  const [showRevoke, setShowRevoke] = useState(false);
  const [revTarget, setRevTarget] = useState(null);
  const [revTargetId, setRevTargetId] = useState(null);
  const [countdown, setCountdown] = useState(3);
  const [canRevoke, setCanRevoke] = useState(false);
  const [devices, setDevices] = useState([]);
  const [loading, setLoading] = useState(true);
  const timerRef = useRef(null);

  const fetchDevices = () => {
    if (!getToken()) { setLoading(false); return; }
    setLoading(true);
    api.get("/devices").then(data => {
      if (Array.isArray(data)) {
        setDevices(data.map(d => {
          const { os, browser } = parseUserAgent(d.user_agent);
          return {
            id: d.id,
            ico: "💻",
            name: d.nickname || "Device",
            type: browser ? `${os} • ${browser}` : os,
            status: d.status,
            enrolled: formatRelativeTime(d.created_at),
            last: formatRelativeTime(d.last_used_at),
            fp: d.id.slice(0, 12) + "...",
          };
        }));
      }
    }).catch(() => {}).finally(() => setLoading(false));
  };

  useEffect(() => { fetchDevices(); }, []);

  const openRevoke = (name, id) => {
    setRevTarget(name); setRevTargetId(id); setShowRevoke(true); setCountdown(3); setCanRevoke(false);
    timerRef.current = setInterval(() => {
      setCountdown(c => {
        if (c <= 1) { clearInterval(timerRef.current); setCanRevoke(true); return 0; }
        return c - 1;
      });
    }, 1000);
  };
  const closeRevoke = () => { clearInterval(timerRef.current); setShowRevoke(false); };
  const doRevoke = async () => {
    closeRevoke();
    try {
      if (revTargetId) await api.post(`/devices/${revTargetId}/revoke`, {});
      toast("Device revoked — access blocked", "danger");
    } catch {
      toast("Revoke failed", "danger");
    }
    fetchDevices(); // refetch
  };

  const doMarkLost = async (id, name) => {
    try {
      await api.post(`/devices/${id}/mark-lost`, {});
      toast(`${name} marked as lost`, "warning");
    } catch {
      toast("Mark lost failed", "danger");
    }
    fetchDevices();
  };

  return (
    <AppShell active="devices" go={go}>
      {showRevoke && (
        <div onClick={e => { if (e.target === e.currentTarget) closeRevoke(); }} style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.88)", backdropFilter: "blur(14px)", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <div className="modal-anim" style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 48, maxWidth: 440, width: "90%", position: "relative" }}>
            <button onClick={closeRevoke} style={{ position: "absolute", top: 18, right: 18, background: "none", border: "none", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 16, cursor: "pointer" }}>×</button>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>// Destructive Action</div>
            <h3 style={{ fontFamily: "var(--display)", fontSize: 44, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 14 }}>Revoke<br />Device?</h3>
            <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, marginBottom: 24, fontWeight: 300 }}>This device will lose all access immediately. Cannot be undone without re-enrollment.</p>
            <div style={{ background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)", padding: 14, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginBottom: 24 }}>⚠ DEVICE BLOCKED WITHIN 500MS OF CONFIRMATION</div>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 24 }}>
              <div style={{ width: 38, height: 38, flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", border: "2px solid var(--danger)", borderRadius: "50%" }}>
                <span style={{ fontFamily: "var(--display)", fontSize: 14, color: "var(--danger)" }}>{countdown}</span>
              </div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", color: "var(--muted)", lineHeight: 1.6 }}>Confirm button activates in {countdown}s</div>
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <BtnO onClick={closeRevoke} style={{ padding: "8px 16px", fontSize: 9 }}>Cancel</BtnO>
              <button onClick={doRevoke} disabled={!canRevoke} style={{ display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: canRevoke ? "pointer" : "not-allowed", opacity: canRevoke ? 1 : .5, transition: "all .25s" }}>Revoke Device</button>
            </div>
          </div>
        </div>
      )}

      <div style={{ padding: "36px 44px 0", borderBottom: "1px solid var(--line)" }}>
        <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", marginBottom: 24 }}>
          <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Devices</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Enrolled hardware · Trust registry</div></div>
          <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Enroll New</BtnF>
        </div>
      </div>

      <div className="page-body" style={{ padding: "28px 44px 60px" }}>
        {loading ? (
          <div style={{ textAlign: "center", padding: "60px 0", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>Loading devices...</div>
        ) : devices.length === 0 ? (
          <div style={{ border: "1px solid var(--line)", padding: "60px", textAlign: "center", background: "var(--ink-2)" }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>📱</div>
            <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>No Devices</div>
            <div style={{ fontSize: 13, color: "var(--muted)", marginBottom: 24 }}>Enroll your first device to get started.</div>
            <BtnF onClick={() => go("register")}>+ Enroll Device</BtnF>
          </div>
        ) : (
          <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(290px,1fr))", gap: 1, background: "var(--line)", border: "1px solid var(--line)" }}>
            {devices.map(d => <DeviceCard key={d.id || d.name} {...d} onRevoke={() => openRevoke(d.name, d.id)} onMarkLost={() => doMarkLost(d.id, d.name)} toast={toast} />)}
          </div>
        )}
      </div>
    </AppShell>
  );
}

function DeviceCard({ ico, name, type: dtype, status, enrolled, last, fp, onRevoke, onMarkLost, toast }) {
  const [hov, setHov] = useState(false);
  const ringC = status === "active" ? "var(--success)" : status === "revoked" ? "var(--danger)" : status === "lost" ? "var(--warning)" : "var(--muted2)";
  const statusC = status === "active" ? { background: "var(--s-success)", color: "var(--success)" } : status === "revoked" ? { background: "var(--s-danger)", color: "var(--danger)" } : status === "lost" ? { background: "var(--s-warning)", color: "var(--warning)" } : { background: "rgba(90,85,80,.15)", color: "var(--muted)" };
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
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--muted)" }}>{dtype}</div>
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
        {status === "active" && (
          <button onClick={onMarkLost} style={{ display: "inline-flex", alignItems: "center", gap: 6, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--warning)", background: "var(--s-warning)", padding: "8px 16px", border: "1px solid rgba(251,191,36,.25)", cursor: "pointer", transition: "all .25s" }}>Mark Lost</button>
        )}
        <button onClick={onRevoke} style={{ display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: "pointer", transition: "all .25s" }}>Revoke</button>
      </div>
    </div>
  );
}
