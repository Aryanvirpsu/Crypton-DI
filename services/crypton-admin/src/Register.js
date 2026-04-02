import { useState } from 'react';
import { doRegister } from './webauthn';
import { BtnF } from './Buttons';

export default function Register({ go, toast }) {
  const [step, setStep] = useState(0);
  const [ident, setIdent] = useState("");
  const [identErr, setIdentErr] = useState(false);
  const [pkDone, setPkDone] = useState(false);
  const [pkBusy, setPkBusy] = useState(false);

  const gotoStep = s => setStep(s);

  const nextStep = from => {
    if (from === 0) {
      if (!ident || ident.length < 2 || !/^[a-zA-Z0-9-@.]+$/.test(ident)) { setIdentErr(true); return; }
      setIdentErr(false); gotoStep(1);
    } else if (from === 1 && pkDone) gotoStep(2);
  };

  const activatePK = async () => {
    if (pkBusy || pkDone) return;
    setPkBusy(true);
    try {
      const email = ident.includes("@") ? ident : `${ident}@crypton.local`;
      await doRegister(email);
      setPkDone(true);
      toast("Passkey created — key sealed in hardware", "success");
    } catch (err) {
      if (err && err.name === "NotAllowedError") {
        toast("Passkey creation was cancelled", "warning");
      } else {
        toast(`Registration failed: ${err.message}`, "danger");
      }
    } finally {
      setPkBusy(false);
    }
  };

  const devClass = ["rv-device", "rv-device s2", "rv-device s3"][step];
  const devLabel = ["Waiting for identity...", "Creating passkey...", "Device enrolled ✓"][step];

  return (
    <div className="pg-in register-grid" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div className="register-form" style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => go("landing")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10, transition: "color .2s" }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← Back to Home</button>

        <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 40 }}>
          {[0, 1, 2].map(i => (
            <div key={i} style={{ height: 3, background: i < step ? "var(--accent)" : i === step ? "var(--accent)" : "var(--line2)", width: i === step ? 28 : 16, transition: "all .3s" }} />
          ))}
        </div>

        {step === 0 && (
          <div className="pg-in">
            <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Create your<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>identity</em></h2>
            <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>Choose an alias for this device. This is how you'll be identified across the Crypton network.</p>
            <div style={{ marginBottom: 20 }}>
              <label style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "block", marginBottom: 8 }}>Identity Label</label>
              <input type="text" value={ident} onChange={e => setIdent(e.target.value)} placeholder="e.g. alex-macbook-pro" maxLength={32}
                style={{ width: "100%", padding: "14px 16px", background: "var(--ink-3)", border: `1px solid ${identErr ? "var(--danger)" : "var(--line2)"}`, color: "var(--paper)", fontFamily: "var(--body)", fontSize: 14, outline: "none" }} />
              {identErr && <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginTop: 6 }}>2–32 chars, alphanumeric and hyphens only</div>}
            </div>
            <BtnF onClick={() => nextStep(0)}>Continue →</BtnF>
          </div>
        )}

        {step === 1 && (
          <div className="pg-in">
            <h2 style={{ fontFamily: "var(--display)", fontSize: 52, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 16 }}>Create your<br /><em style={{ fontFamily: "var(--serif)", fontStyle: "italic" }}>passkey</em></h2>
            <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 32 }}>Your device generates a hardware-bound key pair. The private key is sealed in your secure enclave — forever.</p>
            <div onClick={!pkDone && !pkBusy ? activatePK : undefined} style={{
              width: "100%", padding: 28, border: pkDone ? "1px solid var(--success)" : "1px dashed rgba(200,245,90,.25)",
              background: pkDone ? "rgba(74,222,128,.08)" : "var(--accent-dim)", cursor: (pkDone || pkBusy) ? "default" : "pointer",
              textAlign: "center", transition: "all .25s", marginBottom: 28, opacity: pkBusy ? 0.6 : 1
            }}>
              <div style={{ fontSize: 36, marginBottom: 10 }}>{pkDone ? "✅" : pkBusy ? "⏳" : "🔑"}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 22, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 6 }}>{pkDone ? "Passkey Created ✓" : pkBusy ? "Creating..." : "Create Passkey"}</div>
              <div style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300 }}>{pkDone ? "Hardware key sealed in secure enclave" : pkBusy ? "Follow your device's prompt..." : "Tap to trigger Face ID / Touch ID / hardware key"}</div>
            </div>
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

      <div className="register-vis" style={{ background: "#080808", borderLeft: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden" }}>
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
