import { useState } from 'react';
import { doLogin } from './webauthn';
import { BtnF } from './Buttons';

export default function Login({ go, toast }) {
  const [email, setEmail] = useState("");
  const [emailErr, setEmailErr] = useState(false);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!email || email.length < 2) { setEmailErr(true); return; }
    setEmailErr(false);
    setBusy(true);
    try {
      await doLogin(email);
      toast("Welcome back — authenticated ✓", "success");
      go("dashboard");
    } catch (err) {
      if (err && err.name === "NotAllowedError") {
        toast("Authentication was cancelled", "warning");
      } else {
        toast(`Login failed: ${err.message}`, "danger");
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="pg-in register-grid" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div className="register-form" style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => go("landing")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10 }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← Back to Home</button>

        <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Authenticate<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>your identity</em></h2>
        <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>Enter your username or email and confirm with your passkey — no password needed.</p>

        <div style={{ marginBottom: 20 }}>
          <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Username or Handle</label>
          <input type="text" value={email} onChange={e => setEmail(e.target.value)}
            onKeyDown={e => e.key === "Enter" && submit()}
            placeholder="your handle or email" autoFocus
            style={{ width: "100%", padding: "14px 16px", background: "var(--ink-3)", border: `1px solid ${emailErr ? "var(--danger)" : "var(--line2)"}`, color: "var(--paper)", fontFamily: "var(--body)", fontSize: 14, outline: "none" }} />
          {emailErr && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginTop: 6 }}>Username required (2+ characters)</div>}
        </div>

        <BtnF onClick={submit} style={{ opacity: busy ? 0.5 : 1, pointerEvents: busy ? "none" : "auto" }}>
          {busy ? "Waiting for passkey..." : "Authenticate with Passkey →"}
        </BtnF>

        <div style={{ marginTop: 24, fontSize: 12, color: "var(--muted)" }}>
          No account?{" "}
          <button onClick={() => go("register")} style={{ fontFamily: "var(--body)", fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0 }}>
            Register a passkey →
          </button>
        </div>
      </div>

      <div className="register-vis" style={{ background: "#080808", borderLeft: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden" }}>
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse 70% 70% at 50% 50%,rgba(200,245,90,.06),transparent)" }} />
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 20, position: "relative", zIndex: 1 }}>
          <div className="rv-device s2">
            🔑
            <div className="rv-glow" />
            <div className="rv-shield">🛡</div>
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)" }}>{busy ? "Waiting for authenticator..." : "Touch ID / Face ID / Hardware key"}</div>
        </div>
      </div>
    </div>
  );
}
