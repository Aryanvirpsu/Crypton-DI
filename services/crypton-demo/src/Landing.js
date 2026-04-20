import { BtnF, BtnO } from './Buttons';
import { MAIN_URL } from './config';
import AnimatedHeading from './AnimatedHeading';

export default function Landing({ go }) {
  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", background: "var(--ink)", padding: 20 }}>
      <div style={{ textAlign: "center", maxWidth: 640 }}>
        <div style={{ fontFamily: "var(--display)", fontSize: "clamp(52px, 10vw, 88px)", letterSpacing: ".1em", textTransform: "uppercase", marginBottom: 20, lineHeight: 1 }}>
          <AnimatedHeading
            text="CRYPTON WORKSPACE"
            charStyle={(char, i) => i >= 8 ? { color: 'var(--accent)' } : {}}
          />
        </div>

        <p style={{ fontSize: 16, color: "var(--muted)", lineHeight: 1.8, marginBottom: 40, fontWeight: 300 }}>
          Welcome to your zero-trust SaaS environment. Securely access your applications, manage trusted devices, and review audit logs — all without passwords.
        </p>

        <div style={{ marginBottom: 20 }}>
          <div style={{ fontFamily: "var(--display)", fontSize: "clamp(22px, 4vw, 30px)", letterSpacing: ".08em", textTransform: "uppercase", color: "var(--paper)", marginBottom: 20 }}>
            <AnimatedHeading text="Login with Crypton" />
          </div>
          <div style={{ display: "flex", gap: 16, justifyContent: "center", flexWrap: "wrap" }}>
            <BtnF onClick={() => go("login")} style={{ padding: "16px 32px", fontSize: 14 }}>
              Authenticate →
            </BtnF>
            <BtnO onClick={() => window.location.href = MAIN_URL} style={{ padding: "16px 32px", fontSize: 14 }}>
              Learn More
            </BtnO>
          </div>
        </div>
      </div>

      <div style={{ position: "fixed", bottom: 20, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted2)", textTransform: "uppercase" }}>
        Crypton Demo Application Surface
      </div>
    </div>
  );
}
