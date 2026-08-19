/**
 * Protected Actions — step-up (re-authentication) verification demo.
 *
 * This is a real, minimal primitive, not a demo of fake business actions:
 * it proves you still hold your passkey right now, for a given intent
 * label, with the result audit-logged server-side. It used to gate three
 * hardcoded "results" (grant admin, rotate an API key, delete a resource)
 * that were never persisted — those were removed rather than kept as
 * decoration. See services/crypton-id/src/routes/actions.rs.
 */
import { useState } from "react";
import { crypton, CryptonError } from './sdk';
import { BtnF, BtnO } from './Buttons';
import AppShell from './AppShell';

const EXAMPLE_INTENTS = [
  { id: "confirm_sensitive_action", label: "Confirm a Sensitive Action", desc: "Generic step-up check before something risky proceeds.", ico: "🔐" },
  { id: "approve_role_change",      label: "Approve a Role Change",      desc: "Prove presence before an admin promotes/demotes a user.", ico: "👤" },
  { id: "confirm_recovery_approval",label: "Confirm a Recovery Approval",desc: "Prove presence before approving another device's recovery.", ico: "🔑" },
];

export default function Actions({ go, toast }) {
  const [modal, setModal] = useState(null); // { intent, phase, result, error }

  const startAction = (intent) => setModal({ intent, phase: "confirm", result: null, error: null });

  const executeAction = async () => {
    const { intent } = modal;
    setModal(m => ({ ...m, phase: "signing" }));
    try {
      const outcome = await crypton.actions.sign(intent.id);
      setModal(m => ({ ...m, phase: "success", result: outcome }));
      toast(`${intent.label} — verified`, "success");
    } catch (err) {
      const cancelled = CryptonError.is(err) && err.code === "webauthn_cancelled";
      setModal(m => ({ ...m, phase: "error", error: cancelled ? "Authentication cancelled by user" : (err.message || "Verification failed") }));
      toast(cancelled ? "Cancelled" : `Failed: ${err.message}`, cancelled ? "warning" : "danger");
    }
  };

  return (
    <AppShell active="actions" go={go}>
      {modal && (
        <div onClick={e => { if (e.target === e.currentTarget && modal.phase !== "signing") setModal(null); }}
          style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.88)", backdropFilter: "blur(14px)", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <div className="modal-anim" style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 48, maxWidth: 480, width: "90%", position: "relative" }}>
            <button onClick={() => modal.phase !== "signing" && setModal(null)} style={{ position: "absolute", top: 18, right: 18, background: "none", border: "none", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 16, cursor: "pointer" }}>×</button>

            {modal.phase === "confirm" && (
              <>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>{'// Step-up verification'}</div>
                <h3 style={{ fontFamily: "var(--display)", fontSize: 36, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 14 }}>{modal.intent.label}</h3>
                <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, marginBottom: 24, fontWeight: 300 }}>
                  This proves you currently hold a registered passkey. It does not execute a business action — a real caller decides what to do with a verified result.
                </p>
                <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                  <BtnO onClick={() => setModal(null)} style={{ padding: "8px 16px", fontSize: 9 }}>Cancel</BtnO>
                  <BtnF onClick={executeAction} style={{ padding: "8px 16px", fontSize: 9 }}>Verify with Passkey →</BtnF>
                </div>
              </>
            )}

            {modal.phase === "signing" && (
              <div style={{ textAlign: "center", padding: "20px 0" }}>
                <div style={{ fontSize: 48, marginBottom: 20 }}>🔐</div>
                <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 12 }}>Waiting for Device</div>
                <div style={{ fontSize: 13, color: "var(--muted)", fontWeight: 300 }}>Follow your device's prompt…</div>
              </div>
            )}

            {modal.phase === "success" && (
              <>
                <div style={{ textAlign: "center", marginBottom: 24 }}>
                  <div style={{ fontSize: 48, marginBottom: 16 }}>✅</div>
                  <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 8 }}>Verified</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>intent: {modal.result?.intent}</div>
                </div>
                <div style={{ display: "flex", justifyContent: "flex-end" }}>
                  <BtnO onClick={() => setModal(null)} style={{ padding: "8px 16px", fontSize: 9 }}>Close</BtnO>
                </div>
              </>
            )}

            {modal.phase === "error" && (
              <>
                <div style={{ textAlign: "center", marginBottom: 24 }}>
                  <div style={{ fontSize: 48, marginBottom: 16 }}>❌</div>
                  <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 8, color: "var(--danger)" }}>Failed</div>
                  <div style={{ fontSize: 13, color: "var(--danger)" }}>{modal.error}</div>
                </div>
                <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                  <BtnO onClick={() => setModal(null)} style={{ padding: "8px 16px", fontSize: 9 }}>Close</BtnO>
                  <BtnF onClick={() => setModal(m => ({ ...m, phase: "confirm" }))} style={{ padding: "8px 16px", fontSize: 9 }}>Retry</BtnF>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Protected Actions</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>
          Real WebAuthn re-authentication · Real audit trail
        </div>
      </div>

      <div className="page-body" style={{ padding: "36px 44px 60px" }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", marginBottom: 20, letterSpacing: ".06em" }}>
          {'// Each card triggers an actual cryptographic challenge against your enrolled devices. A revoked device fails.'}
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(290px,1fr))", gap: 1, background: "var(--line)", border: "1px solid var(--line)" }}>
          {EXAMPLE_INTENTS.map(a => <IntentCard key={a.id} intent={a} onClick={() => startAction(a)} />)}
        </div>
      </div>
    </AppShell>
  );
}

function IntentCard({ intent, onClick }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)} onClick={onClick}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", position: "relative", overflow: "hidden" }}>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 18 }}>
        <span style={{ fontSize: 28 }}>{intent.ico}</span>
        <span style={{ fontFamily: "var(--mono)", fontSize: 8, letterSpacing: ".08em", padding: "3px 8px", background: "var(--accent-dim)", color: "var(--accent)", border: "1px solid rgba(200,245,90,.2)" }}>🔐 SIGNATURE REQUIRED</span>
      </div>
      <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>{intent.label}</div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{intent.desc}</div>
    </div>
  );
}
