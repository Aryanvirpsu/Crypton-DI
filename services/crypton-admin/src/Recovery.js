import { useState, useEffect, useRef } from 'react';
import { BtnF, BtnO } from './Buttons';
import AppShell from './AppShell';

export default function Recovery({ go, toast }) {
  const [secs, setSecs] = useState(86400 - 76);
  const [done, setDone] = useState(false);
  const intRef = useRef(null);

  useEffect(() => {
    intRef.current = setInterval(() => setSecs(s => s > 0 ? s - 1 : 0), 1000);
    return () => clearInterval(intRef.current);
  }, []);

  const h = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60), s = secs % 60;
  const timeStr = `${String(h).padStart(2,"0")}:${String(m).padStart(2,"0")}:${String(s).padStart(2,"0")}`;
  const pct = secs / 86400;
  const circ = 2 * Math.PI * 65;
  const offset = circ * (1 - pct);

  const complete = () => {
    clearInterval(intRef.current);
    setDone(true);
    toast("Recovery completed — new device enrolled", "success");
  };

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
            border: `2px solid ${done ? "rgba(74,222,128,.5)" : "rgba(251,191,36,.35)"}`,
            display: "flex", alignItems: "center", justifyContent: "center", fontSize: 64, marginBottom: 36,
            boxShadow: done ? "0 0 80px rgba(74,222,128,.2)" : "0 0 60px rgba(251,191,36,.12)",
            animation: "vFloat 5s ease-in-out infinite", transition: "all .8s"
          }}>{done ? "🔓" : "🔐"}</div>
          <div style={{ position: "relative", marginBottom: 32 }}>
            <svg width="168" height="168" viewBox="0 0 168 168" style={{ transform: "rotate(-90deg)" }}>
              <circle fill="none" stroke="var(--ink-3)" strokeWidth={4} cx={84} cy={84} r={65} />
              <circle fill="none" stroke={done ? "var(--success)" : "var(--warning)"} strokeWidth={4} strokeLinecap="round"
                strokeDasharray={circ} strokeDashoffset={done ? 0 : offset} cx={84} cy={84} r={65} style={{ transition: "stroke-dashoffset 1s linear, stroke .5s" }} />
            </svg>
            <div style={{ position: "absolute", top: "50%", left: "50%", transform: "translate(-50%,-50%)", textAlign: "center" }}>
              <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".04em" }}>{done ? "00:00:00" : timeStr}</div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginTop: 2 }}>remaining</div>
            </div>
          </div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 12 }}>
            {done ? "✅ Recovery Complete" : "⏳ Time-Lock Active"}
          </div>
          <p style={{ fontSize: 13, color: "var(--muted)", maxWidth: 480, lineHeight: 1.75, fontWeight: 300, marginBottom: 40 }}>
            {done
              ? "Time lock expired. New device has been enrolled. All existing devices notified of the completed recovery."
              : "Your recovery request is in progress. A mandatory 24-hour wait protects against unauthorized device enrollment. All trusted devices have been notified."}
          </p>
          <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
            {!done && <BtnF onClick={complete}>Simulate Completion</BtnF>}
            <BtnO onClick={() => toast("Recovery cancelled by trusted device", "danger")}>Cancel</BtnO>
          </div>
        </div>
      </div>
    </AppShell>
  );
}
