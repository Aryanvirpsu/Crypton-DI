import { BtnF, BtnO } from './Buttons';
import AppShell from './AppShell';

export default function RiskIntel({ go, toast }) {
  return (
    <AppShell active="risk" go={go}>
      <div className="page-header" style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div>
          <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Risk Intelligence</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Behavioral anomaly · Geo-velocity · Threat scoring</div>
        </div>
      </div>
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", padding: "80px 44px" }}>
        <div style={{ textAlign: "center", maxWidth: 480 }}>
          <div style={{ position: "relative", width: 96, height: 96, margin: "0 auto 40px" }}>
            <div style={{ position: "absolute", inset: 0, borderRadius: "50%", border: "1px solid rgba(200,245,90,0.15)", animation: "rExpand 3.5s ease-out infinite" }} />
            <div style={{ position: "absolute", inset: 0, borderRadius: "50%", border: "1px solid rgba(200,245,90,0.1)", animation: "rExpand 3.5s ease-out infinite", animationDelay: "1.2s" }} />
            <div style={{ position: "absolute", inset: 0, borderRadius: "50%", background: "radial-gradient(circle, rgba(200,245,90,0.12), transparent)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <span style={{ fontSize: 36 }}>🛡</span>
            </div>
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".18em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>
          <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(36px,5vw,60px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .93, marginBottom: 20 }}>
            Risk<br />Intelligence
          </h2>
          <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.8, fontWeight: 300, marginBottom: 40 }}>
            Behavioral anomaly detection, geo-velocity alerts, and real-time threat scoring are in active development.
            This module will surface suspicious patterns before they become incidents.
          </p>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, justifyContent: "center", marginBottom: 40 }}>
            {["Geo-velocity detection","TOR / VPN blocking","Behavioral baselines","Risk score per user","Step-up auth triggers","Threat feed integration"].map(f => (
              <span key={f} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "6px 12px", border: "1px solid var(--line)", color: "var(--muted)" }}>{f}</span>
            ))}
          </div>
          <div style={{ display: "flex", gap: 10, justifyContent: "center", flexWrap: "wrap" }}>
            <BtnF onClick={() => toast("We'll notify you when Risk Intel ships", "success")} style={{ fontSize: 9 }}>Notify Me →</BtnF>
            <BtnO onClick={() => go("dashboard")} style={{ fontSize: 9 }}>← Dashboard</BtnO>
          </div>
        </div>
      </div>
    </AppShell>
  );
}
