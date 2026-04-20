import { BtnF } from './Buttons';
import AnimatedHeading from './AnimatedHeading';

export default function Landing({ go }) {
  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", background: "var(--ink)", padding: 20 }}>
      <div style={{ textAlign: "center", maxWidth: 640 }}>
        <div style={{ fontFamily: "var(--display)", fontSize: "clamp(64px, 14vw, 120px)", letterSpacing: ".1em", textTransform: "uppercase", marginBottom: 16, lineHeight: 1 }}>
          <AnimatedHeading text="CRYPTON" />
        </div>

        <div style={{ fontFamily: "var(--display)", fontSize: "clamp(20px, 4vw, 32px)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 40 }}>
          <AnimatedHeading text="Login with Crypton" />
        </div>

        <BtnF onClick={() => go("login")} style={{ padding: "16px 40px", fontSize: 14 }}>
          Authenticate →
        </BtnF>
      </div>

      <div style={{ position: "fixed", bottom: 20, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted2)", textTransform: "uppercase" }}>
        Crypton Demo Application Surface
      </div>
    </div>
  );
}
