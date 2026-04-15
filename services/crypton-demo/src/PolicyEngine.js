import { useState, useEffect } from 'react';
import { adminApi } from './api';
import { getAdminToken } from './auth';
import AppShell from './AppShell';

export default function PolicyEngine({ go, toast }) {
  const [policies, setPolicies] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [threshold, setThreshold] = useState(70);
  const [trustDays, setTrustDays] = useState(30);

  useEffect(() => {
    if (!getAdminToken()) { setLoading(false); return; }
    adminApi.get("/policies")
      .then(data => { setPolicies(Array.isArray(data) ? data : []); })
      .catch(() => setError(true))
      .finally(() => setLoading(false));
  }, []);

  const toggle = async id => {
    const pol = policies.find(x => x.id === id);
    const newActive = !pol.active;
    try {
      await adminApi.patch(`/policies/${id}`, { active: newActive });
      setPolicies(p => p.map(x => x.id === id ? { ...x, active: newActive } : x));
      toast(`Policy "${pol.label || pol.name}" ${newActive ? "enabled" : "disabled"}`, newActive ? "success" : "warning");
    } catch {
      toast("Failed to update policy — try again", "danger");
    }
  };

  const catColor = { geo: "var(--accent)", risk: "var(--danger)", network: "var(--warning)", device: "#7EC8E3", auth: "var(--success)", other: "var(--muted)" };
  const cats = [...new Set(policies.map(p => p.cat || "other"))];

  return (
    <AppShell active="policy" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Policy Engine</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Zero-trust rules · Adaptive enforcement</div>
        </div>
        {policies !== null && (
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--success)", letterSpacing: ".06em", display: "flex", alignItems: "center", gap: 8 }}>
            <div style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)", boxShadow: "0 0 8px var(--success)" }} />
            {policies.filter(p => p.active).length} / {policies.length} policies active
          </div>
        )}
      </div>
      <div className="page-body" style={{ padding: "28px 44px 60px" }}>
        {loading && (
          <div style={{ textAlign: "center", padding: "60px 0", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11 }}>Loading policies...</div>
        )}
        {error && (
          <div style={{ border: "1px solid rgba(248,113,113,.2)", padding: "60px", textAlign: "center", background: "var(--ink-2)" }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>⚠</div>
            <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 8 }}>Failed to Load</div>
            <div style={{ fontSize: 13, color: "var(--muted)" }}>Could not reach the policies API. Check your connection and refresh.</div>
          </div>
        )}
        {!loading && !error && policies !== null && <>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 28 }}>
          {[
            { label: "Risk Score Threshold", val: threshold, set: setThreshold, unit: "",      desc: "Step-up auth triggered above this score", min: 10, max: 95 },
            { label: "Device Trust Duration", val: trustDays, set: setTrustDays, unit: " days", desc: "Days before re-verification required",      min: 1,  max: 90 },
          ].map(s => (
            <div key={s.label} style={{ background: "var(--ink-2)", border: "1px solid var(--line)", padding: "20px 22px" }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{s.label}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 40, color: "var(--accent)", marginBottom: 12 }}>{s.val}{s.unit}</div>
              <input type="range" min={s.min} max={s.max} value={s.val}
                onChange={e => { s.set(Number(e.target.value)); toast(`${s.label} set to ${e.target.value}${s.unit}`, "info"); }}
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
              {policies.filter(p => (p.cat || "other") === cat).map((p, i, arr) => (
                <div key={p.id} style={{ display: "flex", alignItems: "center", gap: 16, padding: "16px 18px", borderBottom: i < arr.length - 1 ? "1px solid var(--line)" : "none", background: "var(--ink-2)", transition: "background .15s" }}
                  onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
                  <div onClick={() => toggle(p.id)} style={{ width: 36, height: 20, borderRadius: 10, background: p.active ? "var(--accent)" : "var(--ink-3)", border: `1px solid ${p.active ? "var(--accent)" : "var(--line2)"}`, cursor: "pointer", position: "relative", flexShrink: 0, transition: "background .25s" }}>
                    <div style={{ position: "absolute", top: 2, left: p.active ? 17 : 2, width: 14, height: 14, borderRadius: "50%", background: p.active ? "var(--ink)" : "var(--muted)", transition: "left .25s" }} />
                  </div>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 3 }}>{p.label || p.name}</div>
                    <div style={{ fontSize: 11, color: "var(--muted)" }}>{p.desc || p.description}</div>
                  </div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: p.active ? "var(--success)" : "var(--muted2)", letterSpacing: ".06em" }}>{p.active ? "ACTIVE" : "INACTIVE"}</div>
                </div>
              ))}
            </div>
          </div>
        ))}
        </>}
      </div>
    </AppShell>
  );
}
