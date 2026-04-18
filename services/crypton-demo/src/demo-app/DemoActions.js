/**
 * DemoActions — showcase of Crypton's WebAuthn-protected action flow.
 *
 * Lives in demo-app/ — not part of the admin security surface.
 * Uses crypton.actions.* (SDK) exclusively. No api.js, no webauthn.js.
 *
 * Flow per action:
 *   1. User clicks action card
 *   2. Confirm modal → user clicks "Sign"
 *   3. crypton.actions.sign(id)
 *        → POST /actions/challenge  (backend issues WebAuthn challenge)
 *        → navigator.credentials.get (device prompt)
 *        → POST /actions/execute    (backend verifies signature + runs action)
 *   4. Real audit event written: demo:<action_id>
 */
import { useState } from "react";
import { crypton, getSessionToken } from '../sdk';
import { BtnF, BtnO } from '../Buttons';
import AppShell from '../AppShell';

// Minimal authenticated fetch for non-JSON endpoints
async function _authFetch(path, options = {}) {
  const token = getSessionToken();
  const headers = new Headers(options.headers || {});
  if (token) headers.set('Authorization', `Bearer ${token}`);

  const res = await fetch(`${process.env.REACT_APP_API_BASE || ""}${path}`, { ...options, headers });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res;
}

// ── Demo action catalogue ─────────────────────────────────────────────────────
// Each id must match a string in ALLOWED_ACTIONS in actions.rs
const DEMO_ACTIONS = [
  {
    id: "export_data",
    label: "Export Sensitive Data",
    desc: "Download audit log export — requires cryptographic device sign-off.",
    ico: "📦",
    demo: true,
  },
  {
    id: "rotate_api_key",
    label: "Rotate API Key",
    desc: "Generate new API credentials — hardware signature required.",
    ico: "🔑",
    demo: true,
  },
  {
    id: "add_admin",
    label: "Grant Admin Access",
    desc: "Elevate a user to admin role — Super Admin device required.",
    ico: "👤",
    demo: true,
  },
];

// ── Component ─────────────────────────────────────────────────────────────────

export default function DemoActions({ go, toast }) {
  const [modal, setModal] = useState(null); // { action, phase, result, error }

  const startAction = (action) =>
    setModal({ action, phase: "confirm", result: null, error: null });

  const executeAction = async () => {
    const { action } = modal;
    setModal(m => ({ ...m, phase: "signing" }));

    try {
      // Full WebAuthn-protected flow via SDK — real backend validation,
      // real device trust check, real audit log written as demo:<action_id>
      const outcome = await crypton.actions.sign(action.id);
      setModal(m => ({ ...m, phase: "success", result: outcome.result }));
      toast(`${action.label} — signed and executed`, "success");
    } catch (err) {
      const cancelled = err?.name === "NotAllowedError";
      setModal(m => ({
        ...m,
        phase: "error",
        error: cancelled ? "Authentication cancelled by user" : (err.message || "Action failed"),
      }));
      toast(cancelled ? "Action cancelled" : `Failed: ${err.message}`, cancelled ? "warning" : "danger");
    }
  };

  const handleDownload = () => {
    // Download audit logs as CSV using authenticated fetch
    _authFetch("/export/audit-logs")
      .then(r => r.blob())
      .then(blob => {
        const a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = "crypton-audit.csv";
        a.click();
        URL.revokeObjectURL(a.href);
        toast("Audit export downloaded", "success");
      })
      .catch(() => toast("Download failed", "danger"));
  };

  return (
    <AppShell active="demo" go={go}>

      {/* Modal */}
      {modal && (
        <div
          onClick={e => { if (e.target === e.currentTarget && modal.phase !== "signing") setModal(null); }}
          style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.88)", backdropFilter: "blur(14px)", display: "flex", alignItems: "center", justifyContent: "center" }}
        >
          <div
            className="modal-anim"
            style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 48, maxWidth: 480, width: "90%", position: "relative" }}
          >
            <button
              onClick={() => modal.phase !== "signing" && setModal(null)}
              style={{ position: "absolute", top: 18, right: 18, background: "none", border: "none", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 16, cursor: "pointer" }}
            >×</button>

            {modal.phase === "confirm" && (
              <>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>{'// Demo — Protected Action'}</div>
                <h3 style={{ fontFamily: "var(--display)", fontSize: 36, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 14 }}>{modal.action.label}</h3>
                <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, marginBottom: 24, fontWeight: 300 }}>
                  This action requires cryptographic verification with your enrolled trusted device.
                </p>
                <div style={{ background: "var(--accent-dim)", border: "1px solid rgba(200,245,90,.2)", padding: 16, marginBottom: 24, display: "flex", alignItems: "center", gap: 14 }}>
                  <span style={{ fontSize: 24 }}>🔐</span>
                  <div>
                    <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 4 }}>Signature Required</div>
                    <div style={{ fontSize: 12, color: "var(--muted)" }}>You will be prompted to verify with Face ID, Touch ID, or your security key.</div>
                  </div>
                </div>
                <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                  <BtnO onClick={() => setModal(null)} style={{ padding: "8px 16px", fontSize: 9 }}>Cancel</BtnO>
                  <BtnF onClick={executeAction} style={{ padding: "8px 16px", fontSize: 9 }}>Sign with Trusted Device →</BtnF>
                </div>
              </>
            )}

            {modal.phase === "signing" && (
              <div style={{ textAlign: "center", padding: "20px 0" }}>
                <div style={{ fontSize: 48, marginBottom: 20 }}>🔐</div>
                <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 12 }}>Waiting for Device</div>
                <div style={{ fontSize: 13, color: "var(--muted)", fontWeight: 300 }}>Follow your device's prompt to verify this action…</div>
              </div>
            )}

            {modal.phase === "success" && (
              <>
                <div style={{ textAlign: "center", marginBottom: 24 }}>
                  <div style={{ fontSize: 48, marginBottom: 16 }}>✅</div>
                  <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 8 }}>Action Complete</div>
                  <div style={{ fontSize: 13, color: "var(--success)", fontWeight: 500 }}>{modal.result?.message || "Success"}</div>
                </div>

                {modal.action.id === "rotate_api_key" && modal.result?.api_key && (
                  <div style={{ background: "var(--ink-3)", border: "1px solid var(--line)", padding: 16, marginBottom: 20 }}>
                    <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", marginBottom: 8, letterSpacing: ".08em", textTransform: "uppercase" }}>New API Key</div>
                    <div style={{ fontFamily: "var(--mono)", fontSize: 13, color: "var(--accent)", wordBreak: "break-all", marginBottom: 10 }}>{modal.result.api_key}</div>
                    <button
                      onClick={() => { navigator.clipboard.writeText(modal.result.api_key); toast("Copied", "success"); }}
                      style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", background: "var(--accent-dim)", border: "1px solid rgba(200,245,90,.2)", padding: "5px 12px", cursor: "pointer", letterSpacing: ".06em" }}
                    >COPY</button>
                  </div>
                )}

                {modal.action.id === "add_admin" && modal.result?.user && (
                  <div style={{ background: "var(--s-success)", border: "1px solid rgba(74,222,128,.2)", padding: 16, marginBottom: 20, display: "flex", alignItems: "center", gap: 12 }}>
                    <span style={{ fontSize: 20 }}>👤</span>
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 500 }}>{modal.result.user}</div>
                      <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--success)", letterSpacing: ".06em" }}>Role: {modal.result.role || "Admin"}</div>
                    </div>
                  </div>
                )}

                {modal.action.id === "export_data" && (
                  <div style={{ textAlign: "center", marginBottom: 20 }}>
                    <BtnF onClick={handleDownload} style={{ fontSize: 9 }}>↓ Download Audit Export</BtnF>
                  </div>
                )}

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

      {/* Header — clearly marked as DEMO */}
      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 14, marginBottom: 10 }}>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Protected Actions</div>
          <span style={{ fontFamily: "var(--mono)", fontSize: 8, letterSpacing: ".14em", textTransform: "uppercase", background: "rgba(251,191,36,.12)", color: "var(--warning)", border: "1px solid rgba(251,191,36,.3)", padding: "3px 10px", alignSelf: "center" }}>DEMO</span>
        </div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)" }}>
          Real WebAuthn validation · Real device trust · Real audit trail
        </div>
      </div>

      {/* Body */}
      <div className="page-body" style={{ padding: "36px 44px 60px" }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", marginBottom: 20, letterSpacing: ".06em" }}>
          {'// Each action triggers an actual cryptographic challenge. Signing with a lost or revoked device fails.'}
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(290px,1fr))", gap: 1, background: "var(--line)", border: "1px solid var(--line)" }}>
          {DEMO_ACTIONS.map(a => (
            <ActionCard key={a.id} action={a} onClick={() => startAction(a)} />
          ))}
        </div>
      </div>

    </AppShell>
  );
}

// ── ActionCard ────────────────────────────────────────────────────────────────

function ActionCard({ action, onClick }) {
  const [hov, setHov] = useState(false);
  return (
    <div
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      onClick={onClick}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", position: "relative", overflow: "hidden" }}
    >
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 18 }}>
        <span style={{ fontSize: 28 }}>{action.ico}</span>
        <span style={{ fontFamily: "var(--mono)", fontSize: 8, letterSpacing: ".08em", padding: "3px 8px", background: "var(--accent-dim)", color: "var(--accent)", border: "1px solid rgba(200,245,90,.2)" }}>🔐 SIGNATURE REQUIRED</span>
      </div>
      <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>{action.label}</div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{action.desc}</div>
    </div>
  );
}
