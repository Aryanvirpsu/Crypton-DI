import { useState, useEffect } from 'react';
import { crypton, parseJwt, CryptonError, getSessionToken } from './sdk';
import { _authRef } from './auth';
import { BtnF } from './Buttons';
import { MAIN_URL } from './config';
import AnimatedHeading from './AnimatedHeading';

// Minimal authenticated fetch for non-JSON endpoints
async function _authFetch(path, options = {}) {
  const token = getSessionToken();
  const headers = new Headers(options.headers || {});
  if (token) headers.set('Authorization', `Bearer ${token}`);

  const res = await fetch(`${MAIN_URL}${path}`, { ...options, headers });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res;
}

export default function Login({ go, toast }) {
  const [email, setEmail] = useState("");
  const [emailErr, setEmailErr] = useState(false);
  const [busy, setBusy] = useState(false);
  const [view, setView] = useState("login"); // login, recover_start, recover_status
  const [statusType, setStatusType] = useState(""); // pending, approved
  const [reqId, setReqId] = useState(null);
  const [expiresAt, setExpiresAt] = useState(null);
  const [secsLeft, setSecsLeft] = useState(null);

  useEffect(() => {
    if (!expiresAt) return;
    const tick = () => setSecsLeft(Math.max(0, Math.floor((new Date(expiresAt) - Date.now()) / 1000)));
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [expiresAt]);

  const fetchStatus = async (userEmail) => {
    try {
      const data = await crypton.recovery.getRecoveryStatus(userEmail);
      if (data && data.request) {
        setReqId(data.request.id);
        setStatusType(data.request.status);
        setExpiresAt(data.request.expires_at);
        setView("recover_status");
      }
    } catch {}
  };

  const submit = async () => {
    if (!email || email.length < 2) { setEmailErr(true); return; }
    setEmailErr(false);
    setBusy(true);
    try {
      const result = await crypton.auth.login(email);
      const user = parseJwt(result.token);
      if (_authRef.setUser && user) _authRef.setUser(user);
      // OAuth mode: detected via ?oauth=<nonce> injected by /authorize redirect
      const oauthNonce = new URLSearchParams(window.location.search).get('oauth');
      if (oauthNonce) {
        const resp = await _authFetch('/auth/oauth/complete', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ nonce: oauthNonce, token: result.token }),
        });
        if (!resp.ok) throw new Error('OAuth handshake failed');
        const { redirect_url } = await resp.json();
        window.location.href = redirect_url;
        return;
      }
      toast("Welcome back — authenticated ✓", "success");
      go("dashboard");
    } catch (err) {
      if (err && err.name === "NotAllowedError") {
        toast("Authentication was cancelled", "warning");
      } else if (err instanceof CryptonError && err.code === "recovery_pending") {
        setStatusType("pending");
        setView("recover_status");
        fetchStatus(email);
      } else if (err instanceof CryptonError && err.code === "recovery_approved") {
        setStatusType("approved");
        fetchStatus(email);
      } else {
        toast(`Login failed: ${err.message}`, "danger");
      }
    } finally {
      setBusy(false);
    }
  };

  const startRecovery = async () => {
    if (!email || email.length < 2) { setEmailErr(true); return; }
    setBusy(true);
    try {
      const res = await crypton.recovery.requestRecovery(email);
      setReqId(res.id);
      setStatusType(res.status);
      setView("recover_status");
      toast("Recovery requested", "success");
    } catch (err) {
      toast("Failed to start recovery", "danger");
    } finally {
      setBusy(false);
    }
  };

  const completeRecovery = async () => {
    try {
      setBusy(true);
      await crypton.recovery.claimRecovery(reqId, email);
      toast("Recovery approved! Please register your new authenticator", "success");

      const result = await crypton.auth.register(email);
      const recovered = result.token ? parseJwt(result.token) : null;
      if (_authRef.setUser && recovered) _authRef.setUser(recovered);
      toast("Account successfully recovered. Welcome back.", "success");
      go("dashboard");
    } catch (err) {
      toast("Failed to complete recovery", "danger");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="pg-in register-grid" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div className="register-form" style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => { if (view !== "login") setView("login"); else window.location.href = MAIN_URL; }} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10 }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← {view === "login" ? "Back to Home" : "Cancel"}</button>

        {view === "login" && (
          <div style={{ fontFamily: "var(--display)", fontSize: "clamp(18px, 3vw, 26px)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 12 }}>
            <AnimatedHeading text="Login with Crypton" />
          </div>
        )}
        <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(36px, 6vw, 52px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>
          {view === "login" && <>Authenticate<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>your identity</em></>}
          {view === "recover_start" && <>Recover<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>your account</em></>}
          {view === "recover_status" && <>Access<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>Restricted</em></>}
        </h2>
        
        <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>
          {view === "login" && "Enter your username or email and confirm with your passkey — no password needed."}
          {view === "recover_start" && "Lost your device? Start a recovery request to lock out the account and initiate a 24-jour time-lock."}
          {view === "recover_status" && statusType === "pending" && "Your account is temporarily locked. A 24-hour mandatory time-lock is active while existing authorized devices are given an opportunity to review or cancel this recovery request."}
          {view === "recover_status" && statusType === "approved" && "Your recovery request has been explicitly approved by a trusted device. You may now complete recovery to enroll your new device."}
        </p>

        {view !== "recover_status" && (
          <div style={{ marginBottom: 20 }}>
            <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Username or Handle</label>
            <input type="text" value={email} onChange={e => setEmail(e.target.value)}
              onKeyDown={e => e.key === "Enter" && (view === "login" ? submit() : startRecovery())}
              placeholder="your handle or email" autoFocus
              style={{ width: "100%", padding: "14px 16px", background: "var(--ink-3)", border: `1px solid ${emailErr ? "var(--danger)" : "var(--line2)"}`, color: "var(--paper)", fontFamily: "var(--body)", fontSize: 14, outline: "none" }} />
            {emailErr && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginTop: 6 }}>Username required (2+ characters)</div>}
          </div>
        )}

        {view === "login" && (
          <BtnF onClick={submit} style={{ opacity: busy ? 0.5 : 1, pointerEvents: busy ? "none" : "auto" }}>
            {busy ? "Waiting for passkey..." : "Authenticate with Passkey →"}
          </BtnF>
        )}

        {view === "recover_start" && (
          <BtnF onClick={startRecovery} style={{ opacity: busy ? 0.5 : 1, pointerEvents: busy ? "none" : "auto", background: "var(--warning)", color: "#000" }}>
            {busy ? "Starting Recovery..." : "Initiate Time-Lock Protocol"}
          </BtnF>
        )}

        {view === "recover_status" && statusType === "pending" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {secsLeft != null && (
              <div style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: ".08em", color: "var(--warning)", background: "rgba(251,191,36,.06)", border: "1px solid rgba(251,191,36,.2)", padding: "10px 14px" }}>
                TIME REMAINING — {Math.floor(secsLeft / 3600)}h {Math.floor((secsLeft % 3600) / 60)}m {secsLeft % 60}s
              </div>
            )}
            <button onClick={() => setView("login")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "1px solid var(--line2)", padding: "10px 14px", cursor: "pointer", transition: "color .2s" }}
              onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>
              ← Back to Login
            </button>
          </div>
        )}

        {view === "recover_status" && statusType === "approved" && (
          <BtnF onClick={completeRecovery} style={{ opacity: busy ? 0.5 : 1, pointerEvents: busy ? "none" : "auto", background: "var(--accent)", color: "#000" }}>
            {busy ? "Completing..." : "Complete Recovery & Enroll Device"}
          </BtnF>
        )}

        {view === "login" && (
          <div style={{ marginTop: 24, fontSize: 12, color: "var(--muted)", display: "flex", justifyContent: "space-between" }}>
            <span>
              No account?{" "}
              <button onClick={() => go("register")} style={{ fontFamily: "var(--body)", fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0 }}>
                Register a passkey →
              </button>
            </span>
            <button onClick={() => setView("recover_start")} style={{ fontFamily: "var(--body)", fontSize: 12, color: "var(--warning)", background: "none", border: "none", cursor: "pointer", padding: 0 }}>
              Lost access? →
            </button>
          </div>
        )}
      </div>

      <div className="register-vis" style={{ background: "#080808", borderLeft: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden" }}>
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse 70% 70% at 50% 50%,rgba(200,245,90,.06),transparent)" }} />
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 20, position: "relative", zIndex: 1 }}>
          <div className={`rv-device ${view === "recover_status" ? "" : "s2"}`} style={{ borderColor: view === "recover_status" ? "var(--warning)" : undefined }}>
             {view === "recover_status" ? "🔒" : "🔑"}
            <div className="rv-glow" />
            <div className="rv-shield">🛡</div>
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)" }}>
            {view === "login" ? (busy ? "Waiting for authenticator..." : "Touch ID / Face ID / Hardware key") : "Zero-Trust Device Binding"}
          </div>
        </div>
      </div>
    </div>
  );
}
