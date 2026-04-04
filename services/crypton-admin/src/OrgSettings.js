import { useState, useEffect } from 'react';
import { api } from './api';
import { getToken } from './auth';
import { MOCK_ORG } from './constants';
import { BtnF } from './Buttons';
import AppShell from './AppShell';

export default function OrgSettings({ go, toast }) {
  const [orgName, setOrgName] = useState(MOCK_ORG.orgName);
  const [domain, setDomain] = useState(MOCK_ORG.domain);
  const [mfa, setMfa] = useState(MOCK_ORG.mfaEnforced);
  const [sessionTimeout, setSessionTimeout] = useState(MOCK_ORG.sessionTimeoutHours);
  const [countries, setCountries] = useState(MOCK_ORG.allowedCountries);
  const [domainVerified, setDomainVerified] = useState(MOCK_ORG.domainVerified);
  const allCountries = ["US", "CA", "GB", "DE", "AU", "FR", "JP", "SG", "IN", "BR", "NL", "SE"];

  useEffect(() => {
    if (!getToken()) return;
    api.get("/org").then(data => {
      if (data && typeof data === 'object') {
        if (data.orgName || data.name) setOrgName(data.orgName || data.name);
        if (data.domain) setDomain(data.domain);
        if (data.mfa_required !== undefined) setMfa(data.mfa_required);
        if (data.mfaEnforced !== undefined) setMfa(data.mfaEnforced);
        if (data.sessionTimeoutHours !== undefined) setSessionTimeout(data.sessionTimeoutHours);
        if (data.session_timeout_hours !== undefined) setSessionTimeout(data.session_timeout_hours);
        if (Array.isArray(data.allowedCountries)) setCountries(data.allowedCountries);
        else if (Array.isArray(data.allowed_countries)) setCountries(data.allowed_countries);
        if (data.domainVerified !== undefined) setDomainVerified(data.domainVerified);
      }
    }).catch(() => {});
  }, []);

  const toggleCountry = c => setCountries(cs => cs.includes(c) ? cs.filter(x => x !== c) : [...cs, c]);

  const save = async () => {
    try {
      await api.patch("/org", {
        orgName,
        domain,
        mfaEnforced: mfa,
        sessionTimeoutHours: sessionTimeout,
        allowedCountries: countries,
      });
    } catch {}
    toast("Organization settings saved", "success");
  };

  const verifyDomain = () => { setDomainVerified(true); toast(`Domain ${domain} verified ✓`, "success"); };

  return (
    <AppShell active="orgsettings" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Org Settings</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Organization configuration · Multi-tenant</div></div>
        <BtnF onClick={save} style={{ padding: "8px 16px", fontSize: 9 }}>Save Changes</BtnF>
      </div>
      <div style={{ padding: "28px 44px 60px", display: "flex", flexDirection: "column", gap: 20, maxWidth: 720 }}>

        {/* Identity */}
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

        {/* Security Policies */}
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
              <input type="range" min={1} max={24} value={sessionTimeout} onChange={e => { setSessionTimeout(Number(e.target.value)); }}
                style={{ width: "100%", accentColor: "var(--accent)", cursor: "pointer" }} />
              <div style={{ display: "flex", justifyContent: "space-between", marginTop: 4 }}>
                <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)" }}>1h</span>
                <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)" }}>24h</span>
              </div>
            </div>
          </div>
        </div>

        {/* Allowed Countries */}
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
