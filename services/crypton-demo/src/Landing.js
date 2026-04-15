import { BtnF, BtnO } from './Buttons';
import { MAIN_URL } from './config';

export default function Landing({ go }) {
  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", background: "var(--ink)", padding: 20 }}>
      {/* ── SAAS LANDING APP ── */}
      <div style={{ textAlign: "center", maxWidth: 600 }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 80, letterSpacing: ".1em", textTransform: "uppercase", marginBottom: 20 }}>
          CRYPTON <span style={{ color: "var(--accent)" }}>WORKSPACE</span>
        </div>
        <p style={{ fontSize: 16, color: "var(--muted)", lineHeight: 1.8, marginBottom: 40 }}>
          Welcome to your zero-trust SaaS environment. Securely access your applications, manage trusted devices, and review audit logs—all without passwords.
        </p>
        <div style={{ display: "flex", gap: 16, justifyContent: "center", flexWrap: "wrap" }}>
          <BtnF onClick={() => go("login")} style={{ padding: "16px 32px", fontSize: 14 }}>
            Login with Crypton →
          </BtnF>
          <BtnO onClick={() => window.location.href = MAIN_URL} style={{ padding: "16px 32px", fontSize: 14 }}>
            Learn More
          </BtnO>
        </div>
      </div>
      
      <div style={{ position: "fixed", bottom: 20, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted2)", textTransform: "uppercase" }}>
        Crypton Demo Application Surface
      </div>
    </div>
  );
}
