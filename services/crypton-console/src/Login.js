import { useState } from 'react';
import { crypton, parseJwt, CryptonError } from './sdk';
import { _authRef } from './auth';
import { BtnF } from './Buttons';
import { MAIN_URL } from './config';

/**
 * Login + lost-device recovery entry point.
 *
 * The recovery flow here is intentionally the "no session" half of a
 * two-device process — see Recovery.js for the "approve from a trusted
 * device" half. There is no email/SMS delivery in this system: the
 * recovery_token IS the out-of-band secret, and the UI's job is to make
 * that fact obvious (show it, let the user copy it, let them paste it
 * back in) rather than pretend it was silently emailed anywhere.
 */
export default function Login({ go, toast }) {
  const [email, setEmail] = useState("");
  const [emailErr, setEmailErr] = useState(false);
  const [busy, setBusy] = useState(false);
  const [view, setView] = useState("login"); // login | recover_start | recover_status
  const [recoveryToken, setRecoveryToken] = useState("");
  const [pastedToken, setPastedToken] = useState("");
  const [status, setStatus] = useState(null); // PublicRecoveryStatus | null

  const submit = async () => {
    if (!email || email.length < 2) { setEmailErr(true); return; }
    setEmailErr(false);
    setBusy(true);
    try {
      const result = await crypton.auth.login(email);
      const user = parseJwt(result.token);
      if (_authRef.setUser && user) _authRef.setUser(user);
      toast("Welcome back — authenticated ✓", "success");
      go("dashboard");
    } catch (err) {
      if (CryptonError.is(err) && err.code === "webauthn_cancelled") {
        toast("Authentication was cancelled", "warning");
      } else if (CryptonError.is(err) && err.code === "recovery_approved") {
        // login_start refuses to proceed while an approved recovery is
        // pending completion — send the user to finish it instead.
        toast("A device recovery is approved and awaiting completion", "warning");
        checkStatus();
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
      setStatus(res.request);
      if (res.recovery_token) setRecoveryToken(res.recovery_token);
      setView("recover_status");
      toast("Recovery started — see the code below", "success");
    } catch (err) {
      toast(`Failed to start recovery: ${err.message}`, "danger");
    } finally {
      setBusy(false);
    }
  };

  const checkStatus = async () => {
    setBusy(true);
    try {
      const res = await crypton.recovery.getRecoveryStatus(email);
      setStatus(res.request);
      setView("recover_status");
    } catch (err) {
      toast(`Couldn't check status: ${err.message}`, "danger");
    } finally {
      setBusy(false);
    }
  };

  const completeRecovery = async () => {
    const token = (pastedToken || recoveryToken).trim();
    if (!token) { toast("Paste the recovery code first", "warning"); return; }
    setBusy(true);
    try {
      const claim = await crypton.recovery.claimRecovery(token, email);
      toast("Recovery claimed — creating your new passkey", "success");

      const result = await crypton.auth.register(email, claim.enrollment_token);
      const recovered = result.token ? parseJwt(result.token) : null;
      if (_authRef.setUser && recovered) _authRef.setUser(recovered);
      toast("Account recovered. Welcome back.", "success");
      go("dashboard");
    } catch (err) {
      if (CryptonError.is(err) && err.code === "webauthn_cancelled") {
        toast("Passkey creation was cancelled", "warning");
      } else {
        toast(`Failed to complete recovery: ${err.message}`, "danger");
      }
    } finally {
      setBusy(false);
    }
  };

  const copyToken = () => {
    navigator.clipboard.writeText(recoveryToken);
    toast("Recovery code copied", "success");
  };

  return (
    <div className="pg-in register-grid" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div className="register-form" style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => { if (view !== "login") setView("login"); else window.location.href = MAIN_URL; }} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10 }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← {view === "login" ? "Back to Home" : "Cancel"}</button>

        <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(36px, 6vw, 52px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>
          {view === "login" && <>Authenticate<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>your identity</em></>}
          {view === "recover_start" && <>Recover<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>your account</em></>}
          {view === "recover_status" && <>Lost-device<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>recovery</em></>}
        </h2>

        <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>
          {view === "login" && "Enter your username or email and confirm with your passkey — no password needed."}
          {view === "recover_status" && "There is no email/SMS in this system — the code below IS the recovery secret. Approve this request from a device where you're still logged in, then paste the code back here to finish."}
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
            {busy ? "Starting Recovery..." : "Start Recovery →"}
          </BtnF>
        )}

        {view === "recover_status" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {status && (
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)", letterSpacing: ".06em" }}>
                Status: <span style={{ color: status.status === "approved" ? "var(--accent)" : "var(--warning)" }}>{status.status}</span> · Expires {status.expires_at}
              </div>
            )}

            {recoveryToken && (
              <div style={{ background: "var(--accent-dim)", border: "1px solid rgba(200,245,90,.3)", padding: 16 }}>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 8 }}>Your recovery code — save this</div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 12, wordBreak: "break-all", marginBottom: 10 }}>{recoveryToken}</div>
                <button onClick={copyToken} style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", background: "none", border: "1px solid rgba(200,245,90,.3)", padding: "6px 12px", cursor: "pointer" }}>COPY</button>
              </div>
            )}

            <button onClick={checkStatus} disabled={busy} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "1px solid var(--line2)", padding: "10px 14px", cursor: "pointer" }}>
              ↻ Re-check status
            </button>

            <div>
              <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Recovery code</label>
              <input type="text" value={pastedToken} onChange={e => setPastedToken(e.target.value)}
                placeholder="paste your recovery code here"
                style={{ width: "100%", padding: "14px 16px", background: "var(--ink-3)", border: "1px solid var(--line2)", color: "var(--paper)", fontFamily: "var(--mono)", fontSize: 12, outline: "none", marginBottom: 12 }} />
              <BtnF onClick={completeRecovery} style={{ opacity: busy ? 0.5 : 1, pointerEvents: busy ? "none" : "auto" }}>
                {busy ? "Completing..." : "Complete Recovery & Enroll Device →"}
              </BtnF>
            </div>
          </div>
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
