import { useState, useEffect, useRef } from 'react';
import { crypton, getSessionToken } from './sdk';
import { BtnF, BtnO } from './Buttons';
import AppShell from './AppShell';

export default function Recovery({ go, toast }) {
  const [request, setRequest]   = useState(null); // RecoveryRequest object
  const [loading, setLoading]   = useState(true);
  const [secs, setSecs]         = useState(0);
  const intRef                  = useRef(null);

  // Compute remaining seconds from expires_at
  const computeSecs = (expiresAt) => {
    if (!expiresAt) return 0;
    const diff = Math.floor((new Date(expiresAt).getTime() - Date.now()) / 1000);
    return Math.max(diff, 0);
  };

  // Load existing pending/approved request on mount
  useEffect(() => {
    if (!getSessionToken()) { setLoading(false); return; }
    crypton.recovery.current().then(data => {
      if (data && data.request) {
        setRequest(data.request);
        setSecs(computeSecs(data.request.expires_at));
      }
    }).catch(() => {}).finally(() => setLoading(false));
  }, []);

  // Countdown timer while request is pending/approved
  useEffect(() => {
    clearInterval(intRef.current);
    if (request && (request.status === "pending" || request.status === "approved") && secs > 0) {
      intRef.current = setInterval(() => setSecs(s => s > 0 ? s - 1 : 0), 1000);
    }
    return () => clearInterval(intRef.current);
  }, [request, secs > 0]); // eslint-disable-line react-hooks/exhaustive-deps

  const startRecovery = async () => {
    try {
      const req = await crypton.recovery.start();
      setRequest(req);
      setSecs(computeSecs(req.expires_at));
      toast("Recovery request created — 24-hour time-lock active", "success");
    } catch (err) {
      toast("Failed to start recovery", "danger");
    }
  };

  const approveRecovery = async () => {
    if (!request) return;
    try {
      const req = await crypton.recovery.approve(request.id);
      setRequest(req);
      toast("Recovery approved by this device", "success");
    } catch {
      toast("Approval failed", "danger");
    }
  };

  const rejectRecovery = async () => {
    if (!request) return;
    try {
      await crypton.recovery.reject(request.id);
      setRequest(null);
      toast("Recovery cancelled by trusted device", "success");
    } catch {
      toast("Failed to cancel recovery", "danger");
    }
  };

  const completeRecovery = async () => {
    if (!request) return;
    try {
      const req = await crypton.recovery.complete(request.id);
      setRequest(req);
      clearInterval(intRef.current);
      toast("Recovery completed — new device enrolled", "success");
    } catch {
      toast("Completion failed", "danger");
    }
  };

  const h   = Math.floor(secs / 3600);
  const m   = Math.floor((secs % 3600) / 60);
  const s   = secs % 60;
  const timeStr = `${String(h).padStart(2,"0")}:${String(m).padStart(2,"0")}:${String(s).padStart(2,"0")}`;

  const totalSecs = 86400;
  const pct       = secs / totalSecs;
  const circ      = 2 * Math.PI * 65;
  const offset    = circ * (1 - pct);

  const done      = request?.status === "completed";
  const approved  = request?.status === "approved";
  const pending   = request?.status === "pending";
  const noRequest = !request && !loading;

  return (
    <AppShell active="recovery" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Recovery</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Time-lock zero-trust protocol</div>
      </div>
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", padding: "60px 24px" }}>
        <div className="pg-in" style={{ display: "flex", flexDirection: "column", alignItems: "center", textAlign: "center" }}>
          <div style={{
            width: 160, height: 160, background: "linear-gradient(160deg,var(--ink-3),#060606)",
            border: `2px solid ${done ? "rgba(74,222,128,.5)" : approved ? "rgba(200,245,90,.4)" : "rgba(251,191,36,.35)"}`,
            display: "flex", alignItems: "center", justifyContent: "center", fontSize: 64, marginBottom: 36,
            boxShadow: done ? "0 0 80px rgba(74,222,128,.2)" : "0 0 60px rgba(251,191,36,.12)",
            animation: "vFloat 5s ease-in-out infinite", transition: "all .8s"
          }}>{done ? "🔓" : noRequest ? "🔒" : "🔐"}</div>

          {(pending || approved) && (
            <div style={{ position: "relative", marginBottom: 32 }}>
              <svg width="168" height="168" viewBox="0 0 168 168" style={{ transform: "rotate(-90deg)" }}>
                <circle fill="none" stroke="var(--ink-3)" strokeWidth={4} cx={84} cy={84} r={65} />
                <circle fill="none" stroke={approved ? "var(--accent)" : "var(--warning)"} strokeWidth={4} strokeLinecap="round"
                  strokeDasharray={circ} strokeDashoffset={offset} cx={84} cy={84} r={65} style={{ transition: "stroke-dashoffset 1s linear, stroke .5s" }} />
              </svg>
              <div style={{ position: "absolute", top: "50%", left: "50%", transform: "translate(-50%,-50%)", textAlign: "center" }}>
                <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em" }}>{timeStr}</div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginTop: 2 }}>remaining</div>
              </div>
            </div>
          )}

          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 12 }}>
            {loading    && "Loading..."}
            {noRequest  && "No Active Recovery"}
            {pending    && "⏳ Time-Lock Active"}
            {approved   && "✅ Approved — Complete Recovery"}
            {done       && "✅ Recovery Complete"}
          </div>
          <p style={{ fontSize: 13, color: "var(--muted)", maxWidth: 480, lineHeight: 1.75, fontWeight: 300, marginBottom: 40 }}>
            {loading   && "Checking for existing recovery request..."}
            {noRequest && "No recovery in progress. Start a recovery request to enroll a new device after a 24-hour time-lock."}
            {pending   && "Your recovery request is in progress. A mandatory 24-hour wait protects against unauthorized device enrollment. All trusted devices have been notified."}
            {approved  && "Recovery has been approved by a trusted device. You may now complete the process to enroll a new device."}
            {done      && "Time lock expired. New device has been enrolled. All existing devices notified of the completed recovery."}
          </p>

          {request && (
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted2)", letterSpacing: ".06em", marginBottom: 24 }}>
              Request ID: {request.id?.slice(0, 16)}… · Method: {request.method} · Status: {request.status}
            </div>
          )}

          <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
            {noRequest  && <BtnF onClick={startRecovery}>Start Recovery</BtnF>}
            {pending    && <BtnF onClick={approveRecovery}>Approve (This Device)</BtnF>}
            {approved   && <BtnF onClick={completeRecovery}>Complete Recovery →</BtnF>}
            {(pending || approved) && (
              <BtnO onClick={rejectRecovery}>Cancel</BtnO>
            )}
          </div>
        </div>
      </div>
    </AppShell>
  );
}
