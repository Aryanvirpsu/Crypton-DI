import { useState, useEffect, useRef, useCallback } from "react";

/* ─── FONTS ─── */
const FontLink = () => (
  <style>{`
    @import url('https://fonts.googleapis.com/css2?family=Bebas+Neue&family=DM+Serif+Display:ital@0;1&family=DM+Mono:wght@400;500&family=Geist:wght@300;400;500&display=swap');
    :root {
      --ink:#0A0A0A;--ink-2:#111111;--ink-3:#161616;--ink-4:#1C1C1C;
      --paper:#F4F1EC;--off:#E8E4DC;--muted:#7A7570;--muted2:#5A5550;
      --line:rgba(244,241,236,0.07);--line2:rgba(244,241,236,0.13);
      --accent:#C8F55A;--accent-dim:rgba(200,245,90,0.1);
      --success:#4ADE80;--warning:#FBBF24;--danger:#F87171;
      --s-success:rgba(74,222,128,0.1);--s-warning:rgba(251,191,36,0.1);--s-danger:rgba(248,113,113,0.1);
      --serif:'DM Serif Display',serif;
      --display:'Bebas Neue',sans-serif;
      --mono:'DM Mono',monospace;
      --body:'Geist',sans-serif;
    }
    *,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
    html{scroll-behavior:smooth;cursor:none}
    body{font-family:var(--body);background:var(--ink);color:var(--paper);overflow-x:hidden;line-height:1.5}
    ::selection{background:var(--accent);color:var(--ink)}
    ::-webkit-scrollbar{width:2px}::-webkit-scrollbar-track{background:var(--ink)}::-webkit-scrollbar-thumb{background:var(--accent)}

    .grain{position:fixed;inset:0;z-index:9000;pointer-events:none;background-image:url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");opacity:.035}

    /* cursor */
    .cd{position:fixed;width:7px;height:7px;border-radius:50%;background:var(--accent);pointer-events:none;z-index:9999;transform:translate(-50%,-50%)}
    .cr{position:fixed;width:32px;height:32px;border-radius:50%;border:1px solid rgba(200,245,90,0.4);pointer-events:none;z-index:9998;transform:translate(-50%,-50%);transition:width .4s cubic-bezier(.25,1,.5,1),height .4s,border-color .3s}
    .cr.hov{width:52px;height:52px;border-color:var(--accent)}

    /* page transition */
    .pg-in{animation:pgIn .4s cubic-bezier(.16,1,.3,1)}
    @keyframes pgIn{from{opacity:0;transform:translateY(14px)}to{opacity:1;transform:translateY(0)}}

    /* landing hero */
    @keyframes up{to{transform:translateY(0)}}
    .hero-line{transform:translateY(110%);animation:up 1s cubic-bezier(.16,1,.3,1) forwards}
    .hero-line-2{animation-delay:.07s}
    @keyframes sp{0%{transform:translateY(-100%)}100%{transform:translateY(200%)}}
    @keyframes tick{from{transform:translateX(0)}to{transform:translateX(-50%)}}
    .ticker-track{display:inline-flex;animation:tick 28s linear infinite}
    @keyframes fg{0%,100%{transform:translateY(0) rotate(-3deg)}50%{transform:translateY(-14px) rotate(3deg)}}
    .fv-glyph{animation:fg 7s ease-in-out infinite}

    /* nav */
    nav{position:fixed;top:0;left:0;right:0;z-index:1000;display:flex;align-items:center;justify-content:space-between;padding:28px 52px;mix-blend-mode:difference}

    /* orb */
    @keyframes orbPulse{0%,100%{box-shadow:0 0 20px rgba(74,222,128,.2),0 0 60px rgba(74,222,128,.08);transform:scale(1)}50%{box-shadow:0 0 40px rgba(74,222,128,.35),0 0 80px rgba(74,222,128,.12);transform:scale(1.04)}}
    @keyframes orbWarn{0%,100%{box-shadow:0 0 20px rgba(251,191,36,.25);transform:scale(1)}50%{box-shadow:0 0 50px rgba(251,191,36,.5);transform:scale(1.06)}}
    @keyframes orbDanger{0%,100%{box-shadow:0 0 30px rgba(248,113,113,.4);transform:scale(1)}50%{box-shadow:0 0 60px rgba(248,113,113,.7);transform:scale(1.08)}}
    @keyframes rExpand{0%{opacity:.6}100%{opacity:0;transform:scale(1.1)}}
    .orb-pulse{position:absolute;border-radius:50%;border:1px solid rgba(74,222,128,.15)}
    .orb-pulse.r1{width:120px;height:120px;animation:rExpand 3.5s ease-out infinite;animation-delay:1.2s}
    .orb-pulse.r2{width:140px;height:140px;animation:rExpand 3.5s ease-out infinite;animation-delay:2.4s}

    /* reveal */
    .rv{opacity:0;transform:translateY(36px);transition:opacity .75s ease,transform .75s cubic-bezier(.16,1,.3,1)}
    .rv.in{opacity:1;transform:translateY(0)}
    .rv-1{transition-delay:.1s}.rv-2{transition-delay:.2s}

    /* fi */
    .fi{opacity:0;transform:translateY(20px);transition:opacity .5s ease,transform .5s cubic-bezier(.16,1,.3,1)}
    .fi.in{opacity:1;transform:translateY(0)}

    /* vault */
    @keyframes vFloat{0%,100%{transform:translateY(0) rotateX(5deg)}50%{transform:translateY(-10px) rotateX(5deg)}}

    /* modal anim */
    @keyframes mIn{from{opacity:0;transform:scale(.96) translateY(18px)}to{opacity:1;transform:scale(1) translateY(0)}}
    .modal-anim{animation:mIn .4s cubic-bezier(.16,1,.3,1)}

    /* toast */
    @keyframes tIn{from{transform:translateX(100%);opacity:0}to{transform:translateX(0);opacity:1}}
    @keyframes tOut{to{transform:translateX(100%);opacity:0}}
    .toast-in{animation:tIn .4s cubic-bezier(.16,1,.3,1)}
    .toast-out{animation:tOut .3s ease forwards}

    /* register vis */
    .rv-device{width:140px;height:240px;background:linear-gradient(160deg,var(--ink-3),#060606);border:1px solid rgba(200,245,90,.2);display:flex;align-items:center;justify-content:center;font-size:52px;position:relative;transform:perspective(500px) rotateY(-12deg) rotateX(8deg);transition:transform .7s cubic-bezier(.16,1,.3,1),box-shadow .5s;box-shadow:0 40px 80px rgba(0,0,0,.7)}
    .rv-device.s2{transform:perspective(500px) rotateY(0deg) rotateX(5deg)}
    .rv-device.s3{transform:perspective(500px) rotateY(12deg) rotateX(3deg)}
    .rv-key{position:absolute;top:16%;right:-12px;font-size:24px;opacity:0;transform:scale(.5) translateY(10px);transition:all .5s cubic-bezier(.16,1,.3,1)}
    .rv-device.s2 .rv-key,.rv-device.s3 .rv-key{opacity:1;transform:scale(1) translateY(0)}
    .rv-shield{position:absolute;bottom:10%;font-size:44px;opacity:0;transform:scale(.5);transition:all .6s cubic-bezier(.16,1,.3,1)}
    .rv-device.s3 .rv-shield{opacity:1;transform:scale(1)}
    .rv-glow{position:absolute;inset:-20px;opacity:0;transition:opacity .5s}
    .rv-device.s2 .rv-glow{background:radial-gradient(ellipse,rgba(200,245,90,.12),transparent);opacity:1}
    .rv-device.s3 .rv-glow{background:radial-gradient(ellipse,rgba(74,222,128,.15),transparent);opacity:1}

    /* dcard 3d */
    .dcard:hover .dmodel{transform:rotateY(8deg) rotateX(-4deg) scale(1.05)}
    .dmodel{transition:transform .3s}

    /* feat item */
    .feat-item{transition:padding-left .3s}
    .feat-item:hover{padding-left:10px}

    /* stat hover */
    .stat-c{position:relative;overflow:hidden}
    .stat-c::before{content:'';position:absolute;top:0;left:0;right:0;height:2px;background:var(--accent);transform:scaleX(0);transform-origin:left;transition:transform .5s cubic-bezier(.16,1,.3,1)}
    .stat-c:hover::before{transform:scaleX(1)}

    @media(max-width:1023px){
      nav{padding:20px 24px}
      .nav-links-wrap{display:none}
      .hero-pad,.section-pad{padding-left:24px!important;padding-right:24px!important}
      .sidebar{width:56px}
      .sb-mark,.si-label,.sb-label-txt,.user-name,.user-role{display:none}
      .si{justify-content:center}
      .si-badge{display:none}
    }
    @media(max-width:700px){
      .hero-line{font-size:clamp(60px,16vw,100px)!important}
    }
    @media(prefers-reduced-motion:reduce){*,*::before,*::after{animation:none!important;transition:none!important}}
  `}</style>
);

/* ─── CURSOR ─── */
function Cursor() {
  const dotRef = useRef(null);
  const ringRef = useRef(null);
  const pos = useRef({ mx: 0, my: 0, rx: 0, ry: 0 });
  const [hov, setHov] = useState(false);

  useEffect(() => {
    const move = e => { pos.current.mx = e.clientX; pos.current.my = e.clientY; };
    document.addEventListener("mousemove", move);
    let raf;
    const loop = () => {
      const p = pos.current;
      p.rx += (p.mx - p.rx) * 0.13;
      p.ry += (p.my - p.ry) * 0.13;
      if (dotRef.current) { dotRef.current.style.left = p.mx + "px"; dotRef.current.style.top = p.my + "px"; }
      if (ringRef.current) { ringRef.current.style.left = p.rx + "px"; ringRef.current.style.top = p.ry + "px"; }
      raf = requestAnimationFrame(loop);
    };
    loop();
    const over = e => setHov(e.target.closest("a,button") !== null);
    document.addEventListener("mouseover", over);
    return () => { document.removeEventListener("mousemove", move); document.removeEventListener("mouseover", over); cancelAnimationFrame(raf); };
  }, []);

  return (
    <>
      <div ref={dotRef} className="cd" />
      <div ref={ringRef} className={`cr${hov ? " hov" : ""}`} />
    </>
  );
}

/* ─── TOAST ─── */
let toastId = 0;
function useToasts() {
  const [toasts, setToasts] = useState([]);
  const add = useCallback((msg, type = "info") => {
    const id = ++toastId;
    setToasts(t => [...t, { id, msg, type }]);
    setTimeout(() => setToasts(t => t.map(x => x.id === id ? { ...x, out: true } : x)), 3200);
    setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 3500);
  }, []);
  return [toasts, add];
}

function ToastStack({ toasts }) {
  return (
    <div style={{ position: "fixed", bottom: 24, right: 24, zIndex: 8000, display: "flex", flexDirection: "column", gap: 6 }}>
      {toasts.map(t => (
        <div key={t.id} className={t.out ? "toast-out" : "toast-in"} style={{
          background: "var(--ink-2)", border: "1px solid var(--line)",
          borderLeft: `2px solid ${t.type === "danger" ? "var(--danger)" : t.type === "success" ? "var(--success)" : "var(--accent)"}`,
          padding: "11px 16px", fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em",
          color: "var(--paper)", minWidth: 220
        }}>// {t.msg}</div>
      ))}
    </div>
  );
}

/* ─── SCROLL REVEAL ─── */
function useReveal(deps = []) {
  useEffect(() => {
    const obs = new IntersectionObserver(entries => {
      entries.forEach(e => { if (e.isIntersecting) e.target.classList.add("in"); });
    }, { threshold: 0.1 });
    document.querySelectorAll(".rv,.fi").forEach(el => obs.observe(el));
    return () => obs.disconnect();
  }, deps);
}

/* ─── SIDEBAR ─── */
function Sidebar({ active, go }) {
  const navItems = [
    { id: "dashboard", ico: "◈", label: "Dashboard" },
    { id: "devices", ico: "📱", label: "Devices", badge: "3" },
    { id: "activity", ico: "📋", label: "Activity" },
  ];
  const secItems = [
    { id: "recovery", ico: "🔒", label: "Recovery" },
    { id: "admin", ico: "⚙", label: "Admin" },
  ];
  return (
    <aside style={{ width: 220, flexShrink: 0, background: "var(--ink-2)", borderRight: "1px solid var(--line)", display: "flex", flexDirection: "column", position: "sticky", top: 0, height: "100vh", overflowY: "auto" }}>
      <div onClick={() => go("home")} style={{ padding: "24px 20px", display: "flex", alignItems: "center", gap: 10, borderBottom: "1px solid var(--line)", cursor: "pointer" }}>
        <div style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent)", flexShrink: 0 }} />
        <span className="sb-mark" style={{ fontFamily: "var(--display)", fontSize: 16, letterSpacing: ".12em" }}>CRYPTON</span>
      </div>
      <nav style={{ padding: "20px 16px", flex: 1 }}>
        <div className="sb-label-txt" style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted2)", padding: "8px 4px 6px", marginBottom: 2 }}>Main</div>
        {navItems.map(item => (
          <button key={item.id} onClick={() => go(item.id)} className="si" style={{
            display: "flex", alignItems: "center", gap: 10, padding: "10px",
            cursor: "pointer", fontSize: 13, color: active === item.id ? "var(--paper)" : "var(--muted)",
            border: "none", background: active === item.id ? "rgba(200,245,90,.07)" : "none",
            width: "100%", textAlign: "left", fontFamily: "var(--body)", position: "relative",
            borderLeft: active === item.id ? "2px solid var(--accent)" : "2px solid transparent",
            transition: "background .15s, color .15s"
          }}>
            <span style={{ fontSize: 14, width: 18, flexShrink: 0 }}>{item.ico}</span>
            <span className="si-label">{item.label}</span>
            {item.badge && <span className="si-badge" style={{ marginLeft: "auto", fontFamily: "var(--mono)", fontSize: 8, background: "var(--accent)", color: "var(--ink)", padding: "2px 6px" }}>{item.badge}</span>}
          </button>
        ))}
        <div className="sb-label-txt" style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted2)", padding: "8px 4px 6px", marginBottom: 2, marginTop: 14 }}>Security</div>
        {secItems.map(item => (
          <button key={item.id} onClick={() => go(item.id)} className="si" style={{
            display: "flex", alignItems: "center", gap: 10, padding: "10px",
            cursor: "pointer", fontSize: 13, color: active === item.id ? "var(--paper)" : "var(--muted)",
            border: "none", background: active === item.id ? "rgba(200,245,90,.07)" : "none",
            width: "100%", textAlign: "left", fontFamily: "var(--body)", position: "relative",
            borderLeft: active === item.id ? "2px solid var(--accent)" : "2px solid transparent",
            transition: "background .15s, color .15s"
          }}>
            <span style={{ fontSize: 14, width: 18, flexShrink: 0 }}>{item.ico}</span>
            <span className="si-label">{item.label}</span>
          </button>
        ))}
      </nav>
      <div style={{ padding: 16, borderTop: "1px solid var(--line)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, padding: 10, background: "rgba(255,255,255,.025)", cursor: "pointer" }}>
          <div style={{ width: 30, height: 30, background: "var(--accent)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--display)", fontSize: 14, color: "var(--ink)", flexShrink: 0 }}>A</div>
          <div>
            <div className="user-name" style={{ fontSize: 12, fontWeight: 500 }}>Alex Chen</div>
            <div className="user-role" style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", letterSpacing: ".06em", textTransform: "uppercase" }}>Owner</div>
          </div>
        </div>
      </div>
    </aside>
  );
}

/* ─── APP SHELL (pages with sidebar) ─── */
function AppShell({ active, go, children }) {
  return (
    <div style={{ display: "flex", flexDirection: "row", minHeight: "100vh" }}>
      <Sidebar active={active} go={go} />
      <main style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column" }}>{children}</main>
    </div>
  );
}

/* ─── BUTTONS ─── */
const BtnF = ({ children, onClick, style = {} }) => (
  <button onClick={onClick} style={{
    display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 10,
    letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--accent)",
    padding: "12px 24px", border: "1px solid var(--accent)", cursor: "pointer", transition: "all .25s", ...style
  }}
    onMouseEnter={e => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "var(--accent)"; }}
    onMouseLeave={e => { e.currentTarget.style.background = "var(--accent)"; e.currentTarget.style.color = "var(--ink)"; }}
  >{children}</button>
);

const BtnO = ({ children, onClick, style = {} }) => (
  <button onClick={onClick} style={{
    display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 10,
    letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none",
    padding: "12px 24px", border: "1px solid var(--line2)", cursor: "pointer", transition: "all .25s", ...style
  }}
    onMouseEnter={e => { e.currentTarget.style.background = "var(--paper)"; e.currentTarget.style.color = "var(--ink)"; }}
    onMouseLeave={e => { e.currentTarget.style.background = "none"; e.currentTarget.style.color = "var(--paper)"; }}
  >{children}</button>
);

/* ═══════════════════════════════════════════════════════════════
   LANDING PAGE
═══════════════════════════════════════════════════════════════ */
function HeroCanvas() {
  const ref = useRef(null);
  useEffect(() => {
    const c = ref.current; if (!c) return;
    const ctx = c.getContext("2d");
    const resize = () => { c.width = c.offsetWidth; c.height = c.offsetHeight; };
    resize(); window.addEventListener("resize", resize);
    const nodes = Array.from({ length: 55 }, () => ({
      x: Math.random(), y: Math.random(),
      vx: (Math.random() - .5) * .00025, vy: (Math.random() - .5) * .00025
    }));
    let raf;
    const draw = () => {
      ctx.clearRect(0, 0, c.width, c.height);
      nodes.forEach(n => {
        n.x += n.vx; n.y += n.vy;
        if (n.x < 0 || n.x > 1) n.vx *= -1;
        if (n.y < 0 || n.y > 1) n.vy *= -1;
      });
      for (let i = 0; i < nodes.length; i++) for (let j = i + 1; j < nodes.length; j++) {
        const dx = (nodes[i].x - nodes[j].x) * c.width, dy = (nodes[i].y - nodes[j].y) * c.height;
        const d = Math.sqrt(dx * dx + dy * dy);
        if (d < 110) {
          ctx.beginPath(); ctx.moveTo(nodes[i].x * c.width, nodes[i].y * c.height);
          ctx.lineTo(nodes[j].x * c.width, nodes[j].y * c.height);
          ctx.strokeStyle = `rgba(200,245,90,${.12 * (1 - d / 110)})`; ctx.lineWidth = .6; ctx.stroke();
        }
      }
      nodes.forEach(n => { ctx.beginPath(); ctx.arc(n.x * c.width, n.y * c.height, 1.2, 0, Math.PI * 2); ctx.fillStyle = "rgba(200,245,90,.35)"; ctx.fill(); });
      raf = requestAnimationFrame(draw);
    };
    draw();
    return () => { window.removeEventListener("resize", resize); cancelAnimationFrame(raf); };
  }, []);
  return <canvas ref={ref} style={{ position: "absolute", inset: 0, opacity: .3, width: "100%", height: "100%" }} />;
}

function Landing({ go, toast }) {
  useReveal([]);
  return (
    <div style={{ background: "var(--ink)" }} className="pg-in">
      {/* NAV */}
      <nav>
        <a href="#" style={{ fontFamily: "var(--display)", fontSize: 20, letterSpacing: ".14em", color: "var(--paper)", textDecoration: "none" }}>CRYPTON</a>
        <ul className="nav-links-wrap" style={{ display: "flex", gap: 36, listStyle: "none" }}>
          {["About", "Protocol", "Features", "Developer"].map(l => (
            <li key={l}><a href={`#${l.toLowerCase()}`} onClick={e => e.preventDefault()} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", textDecoration: "none", opacity: .5, transition: "opacity .2s" }}
              onMouseEnter={e => e.target.style.opacity = 1} onMouseLeave={e => e.target.style.opacity = .5}>{l}</a></li>
          ))}
        </ul>
        <button onClick={() => go("register")} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--paper)", padding: "10px 22px", border: "none", cursor: "pointer", transition: "background .2s" }}
          onMouseEnter={e => e.currentTarget.style.background = "var(--accent)"}
          onMouseLeave={e => e.currentTarget.style.background = "var(--paper)"}>Enroll Device</button>
      </nav>

      {/* HERO */}
      <section style={{ height: "100vh", position: "relative", overflow: "hidden", display: "flex", flexDirection: "column", justifyContent: "flex-end", padding: "0 52px 56px" }} className="hero-pad">
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse 100% 70% at 50% 15%,#1C1C1C 0%,var(--ink) 55%)" }} />
        <HeroCanvas />
        <div style={{ position: "relative", zIndex: 1, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", display: "flex", alignItems: "center", gap: 14, marginBottom: 20 }}>
          <div style={{ width: 36, height: 1, background: "var(--accent)" }} />Zero-trust · Device identity · No passwords
        </div>
        <div style={{ position: "relative", zIndex: 1, overflow: "hidden" }}>
          <div className="hero-line" style={{ fontFamily: "var(--display)", fontSize: "clamp(80px,13.5vw,195px)", lineHeight: .9, letterSpacing: ".025em", textTransform: "uppercase" }}>Identity</div>
          <div className="hero-line hero-line-2" style={{ fontFamily: "var(--serif)", fontStyle: "italic", fontSize: "clamp(72px,12.5vw,180px)", lineHeight: .9 }}>Redefined.</div>
        </div>
        <div style={{ position: "relative", zIndex: 1, display: "flex", alignItems: "flex-end", justifyContent: "space-between", marginTop: 36, paddingTop: 24, borderTop: "1px solid var(--line)" }}>
          <p style={{ fontSize: 14, color: "var(--muted)", maxWidth: 300, lineHeight: 1.75, fontWeight: 300 }}>Authentication powered by hardware cryptography. Every request verified. Every device accountable.</p>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "flex", alignItems: "center", gap: 12 }}>
            <div style={{ width: 1, height: 44, background: "rgba(122,117,112,.3)", position: "relative", overflow: "hidden" }}>
              <div style={{ position: "absolute", top: 0, left: 0, width: "100%", height: "50%", background: "var(--accent)", animation: "sp 2s ease-in-out infinite" }} />
            </div>
            Scroll to explore
          </div>
        </div>
      </section>

      {/* TICKER */}
      <div style={{ background: "var(--accent)", overflow: "hidden", padding: "13px 0", whiteSpace: "nowrap" }} aria-hidden="true">
        <div className="ticker-track">
          {["Zero Trust","Device Identity","Hardware Keys","No Passwords","Cryptographic Proof","Instant Revocation"].flatMap(t => [
            <span key={t} style={{ fontFamily: "var(--display)", fontSize: 17, letterSpacing: ".1em", color: "var(--ink)", padding: "0 28px", textTransform: "uppercase" }}>{t}</span>,
            <span key={t+"d"} style={{ opacity: .35, fontFamily: "var(--display)", fontSize: 17, color: "var(--ink)" }}>—</span>
          ])}
          {["Zero Trust","Device Identity","Hardware Keys","No Passwords","Cryptographic Proof","Instant Revocation"].flatMap(t => [
            <span key={t+"2"} style={{ fontFamily: "var(--display)", fontSize: 17, letterSpacing: ".1em", color: "var(--ink)", padding: "0 28px", textTransform: "uppercase" }}>{t}</span>,
            <span key={t+"d2"} style={{ opacity: .35, fontFamily: "var(--display)", fontSize: 17, color: "var(--ink)" }}>—</span>
          ])}
        </div>
      </div>

      {/* MANIFESTO */}
      <section id="about" style={{ padding: "140px 52px" }} className="section-pad">
        <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
          <span>01 — Manifesto</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <div className="rv" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 80, alignItems: "end" }}>
          <div style={{ fontFamily: "var(--serif)", fontSize: "clamp(34px,3.8vw,54px)", lineHeight: 1.15, letterSpacing: "-.01em" }}>
            Passwords were a<br /><em style={{ fontStyle: "italic", color: "var(--muted)" }}>compromise.</em><br />We built the alternative.
          </div>
          <div>
            <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.85, maxWidth: 360, fontWeight: 300, marginBottom: 36 }}>
              Crypton is a zero-trust identity platform where authentication is tied to cryptographic device keys — not passwords, not secrets, not human memory. Your private key never leaves your hardware.
            </p>
            <BtnO onClick={() => toast("Whitepaper coming soon")}>Read the whitepaper →</BtnO>
          </div>
        </div>
      </section>

      {/* HOW IT WORKS */}
      <section id="protocol" style={{ padding: "0 52px 140px" }} className="section-pad">
        <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
          <span>02 — Protocol</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <h2 className="rv" style={{ fontFamily: "var(--display)", fontSize: "clamp(44px,7vw,92px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .93, marginBottom: 6 }}>Three-step<br />verification</h2>
        <p className="rv rv-1" style={{ fontSize: 14, color: "var(--muted)", maxWidth: 420, fontWeight: 300, lineHeight: 1.75, marginBottom: 0 }}>A deterministic challenge-response protocol. No shared secrets. No replay attacks. Mathematically sound.</p>
        <div className="rv rv-1" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 0, background: "var(--line)", marginTop: 72, border: "1px solid var(--line)" }}>
          {[
            { n: "01", t: "Challenge", b: "Server issues a unique, time-bound cryptographic nonce. Non-repeatable. Expires in milliseconds. Cannot be predicted or pre-computed." },
            { n: "02", t: "Sign", b: "Your device's hardware security module signs the nonce using its private key. The key never leaves the chip. Face ID. Touch ID. Hardware token." },
            { n: "03", t: "Verify", b: "The server verifies the signature against your registered public key. Match means access. Mismatch means instant rejection. Sub-200ms total." },
          ].map(c => (
            <HiWCard key={c.n} {...c} />
          ))}
        </div>
      </section>

      {/* FEATURES */}
      <section id="features" style={{ padding: "0 52px 140px" }} className="section-pad">
        <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
          <span>03 — Capabilities</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 0 }}>
          <div style={{ paddingRight: 72, borderRight: "1px solid var(--line)" }}>
            {[
              { i: "F.01", t: "No Passwords", b: "Eliminate the entire password attack surface. No phishing, no credential stuffing, no breached database exposure." },
              { i: "F.02", t: "Hardware Keys", b: "Private keys generated and stored in device secure enclaves. Extraction is physically impossible by design." },
              { i: "F.03", t: "Zero Trust", b: "Every single request independently verified. No implicit trust. Deny by default, verify by cryptographic proof." },
              { i: "F.04", t: "Instant Revocation", b: "Revoke a compromised device in under 500ms. Propagates globally. No stale sessions. No grace periods." },
              { i: "F.05", t: "Audit Trail", b: "Cryptographically signed, tamper-proof log of every authentication event. Export to CSV or JSON for compliance." },
              { i: "F.06", t: "Time-Lock Recovery", b: "24-hour mandatory waiting period on recovery. Existing trusted devices can cancel unauthorized attempts." },
            ].map((f, idx) => (
              <div key={f.i} className={`feat-item rv${idx > 2 ? "" : ""}`} style={{ padding: "32px 0", borderBottom: idx < 5 ? "1px solid var(--line)" : "none", display: "flex", alignItems: "flex-start", gap: 20 }}>
                <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".12em", marginTop: 3, flexShrink: 0 }}>{f.i}</span>
                <div>
                  <div style={{ fontFamily: "var(--display)", fontSize: 22, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 7 }}>{f.t}</div>
                  <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{f.b}</div>
                </div>
              </div>
            ))}
          </div>
          <div style={{ paddingLeft: 72 }}>
            <div className="rv" style={{ position: "sticky", top: 120, height: 400, background: "var(--ink-3)", border: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", overflow: "hidden" }}>
              <div style={{ position: "absolute", inset: 0, backgroundImage: "linear-gradient(var(--line) 1px,transparent 1px),linear-gradient(90deg,var(--line) 1px,transparent 1px)", backgroundSize: "44px 44px", maskImage: "radial-gradient(ellipse 80% 80% at 50% 50%,black,transparent)" }} />
              <div style={{ textAlign: "center", position: "relative", zIndex: 1 }}>
                <div className="fv-glyph" style={{ fontSize: 100, lineHeight: 1, filter: "drop-shadow(0 0 50px rgba(200,245,90,.35))" }}>🔐</div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".15em", textTransform: "uppercase", color: "var(--accent)", marginTop: 14 }}>// secure enclave active</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* STATS */}
      <div className="rv" style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", background: "var(--ink-2)", borderTop: "1px solid var(--line)", borderBottom: "1px solid var(--line)" }}>
        {[
          { v: "0", d: "Passwords ever stored\nin the Crypton system" },
          { v: "100%", d: "Requests cryptographically\nverified, every time" },
          { v: "<200ms", d: "End-to-end authentication\nlatency on 4G" },
          { v: "∞", d: "Entropy in each\nhardware-generated key" },
        ].map((s, i) => (
          <div key={i} className="stat-c" style={{ padding: "52px 36px", borderRight: i < 3 ? "1px solid var(--line)" : "none" }}>
            <div style={{ fontFamily: "var(--display)", fontSize: "clamp(48px,5.5vw,76px)", lineHeight: 1, letterSpacing: ".02em", marginBottom: 10 }}>{s.v}</div>
            <div style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300, lineHeight: 1.65, whiteSpace: "pre-line" }}>{s.d}</div>
          </div>
        ))}
      </div>

      {/* DEVELOPER */}
      <section id="developer" style={{ padding: "140px 52px" }} className="section-pad">
        <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", color: "var(--muted)", marginBottom: 72, display: "flex", alignItems: "center", gap: 16 }}>
          <span>04 — Developer</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "5fr 7fr", alignItems: "stretch" }}>
          <div className="rv" style={{ padding: "80px 56px 80px 0", borderRight: "1px solid var(--line)", display: "flex", flexDirection: "column", justifyContent: "center" }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 22 }}>// Integrate in 5 minutes</div>
            <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(44px,5.5vw,76px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 28 }}>Ship<br />Zero-<br />Trust.</h2>
            <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.85, marginBottom: 36, fontWeight: 300, maxWidth: 320 }}>Our SDK handles all cryptographic operations. Type-safe API. Works with any backend.</p>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 36 }}>
              {["TypeScript", "Rust", "Python", "Go"].map(l => <span key={l} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "5px 11px", border: "1px solid var(--line)", color: "var(--muted)" }}>{l}</span>)}
            </div>
            <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
              <BtnF onClick={() => toast("Documentation opening...")}>Read the Docs →</BtnF>
              <BtnO onClick={() => toast("GitHub link")}>GitHub</BtnO>
            </div>
          </div>
          <div className="rv rv-1" style={{ padding: "80px 0 80px 56px" }}>
            <CodeBlock toast={toast} />
          </div>
        </div>
      </section>

      {/* FOOTER */}
      <footer style={{ borderTop: "1px solid var(--line)", padding: "72px 52px 36px" }}>
        <div className="rv" style={{ display: "grid", gridTemplateColumns: "3fr 1fr 1fr 1fr", gap: 48, paddingBottom: 56, borderBottom: "1px solid var(--line)" }}>
          <div>
            <div style={{ fontFamily: "var(--display)", fontSize: 56, letterSpacing: ".08em", lineHeight: 1, marginBottom: 20 }}>CRYPTON</div>
            <p style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300, lineHeight: 1.7, maxWidth: 250 }}>Zero-trust device identity. Authentication powered by cryptography — not passwords, not hope.</p>
          </div>
          {[
            { h: "Product", links: ["Protocol", "Features", "Pricing", "Changelog"] },
            { h: "Developer", links: ["Documentation", "API Reference", "SDK", "GitHub"] },
            { h: "Company", links: ["About", "Security", "Privacy", "Terms"] },
          ].map(col => (
            <div key={col.h}>
              <h5 style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 18 }}>{col.h}</h5>
              <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: 11 }}>
                {col.links.map(l => <li key={l}><a href="#" onClick={e => e.preventDefault()} style={{ fontSize: 13, color: "var(--paper)", opacity: .55, textDecoration: "none", transition: "opacity .2s" }}
                  onMouseEnter={e => e.target.style.opacity = 1} onMouseLeave={e => e.target.style.opacity = .55}>{l}</a></li>)}
              </ul>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: 28, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted)" }}>
          <span>© 2026 CRYPTON — ALL RIGHTS RESERVED</span><span>V1.0 · MARCH 2026</span>
        </div>
      </footer>
    </div>
  );
}

function HiWCard({ n, t, b }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)}
      style={{ background: hov ? "var(--ink-2)" : "var(--ink)", padding: "44px 32px", position: "relative", overflow: "hidden", cursor: "default", transition: "background .35s", borderRight: "1px solid var(--line)" }}>
      <div style={{ fontFamily: "var(--display)", fontSize: 72, color: hov ? "rgba(200,245,90,.18)" : "rgba(244,241,236,.07)", lineHeight: 1, marginBottom: 28, transition: "color .3s" }}>{n}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 14 }}>{t}</div>
      <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{b}</div>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .5s cubic-bezier(.16,1,.3,1)" }} />
    </div>
  );
}

function CodeBlock({ toast }) {
  const code = `import { Crypton } from '@crypton/sdk';

const client = new Crypton({
  apiKey: process.env.CRYPTON_KEY,
  origin: 'https://yourapp.com',
});

// authenticate a device
const { verified, deviceId } =
  await client.authenticate({
    userId: 'user_abc123',
    challenge: generateNonce(),
  });

if (verified) {
  // ✓ cryptographically proven
  grantAccess(deviceId);
}`;
  return (
    <div style={{ background: "#0C0C0C", border: "1px solid var(--line)", overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "13px 18px", borderBottom: "1px solid var(--line)", background: "rgba(255,255,255,.02)" }}>
        <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: ".05em" }}>crypton.ts</span>
        <button onClick={() => { navigator.clipboard.writeText(code); toast("Code copied"); }} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", color: "var(--accent)", background: "none", border: "1px solid rgba(200,245,90,.25)", padding: "4px 11px", cursor: "pointer" }}>COPY</button>
      </div>
      <div style={{ padding: "22px 18px", fontFamily: "var(--mono)", fontSize: 12.5, lineHeight: 2, overflowX: "auto" }}>
        <pre style={{ margin: 0 }}><code dangerouslySetInnerHTML={{ __html: code.replace(/import|const|await|if/g, w => `<span style="color:#CC99FF">${w}</span>`).replace(/'[^']*'/g, m => `<span style="color:#C8F55A">${m}</span>`).replace(/Crypton|authenticate|grantAccess|generateNonce/g, m => `<span style="color:#82AAFF">${m}</span>`) }} /></pre>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   HOME / SPLASH
═══════════════════════════════════════════════════════════════ */
function Home({ go }) {
  return (
    <div className="pg-in" style={{ background: "var(--ink)", minHeight: "100vh", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", padding: "52px 24px", textAlign: "center", gap: 0 }}>
      <div style={{ fontFamily: "var(--display)", fontSize: "clamp(64px,14vw,160px)", letterSpacing: ".06em", lineHeight: .9, marginBottom: 32 }}>CRYPTON</div>
      <p style={{ fontSize: 14, color: "var(--muted)", maxWidth: 400, lineHeight: 1.75, fontWeight: 300, marginBottom: 52 }}>Zero-trust identity. Navigate the full platform below.</p>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 1, background: "var(--line)", maxWidth: 640, width: "100%", border: "1px solid var(--line)" }}>
        {[
          { id: "landing", n: "00", t: "Landing Page", d: "Marketing site — hero, manifesto, features, developer docs" },
          { id: "register", n: "01", t: "Registration", d: "Device enrollment flow with passkey creation and 3-step onboarding" },
          { id: "dashboard", n: "02", t: "Dashboard", d: "Security status, Trust Orb, and real-time activity feed" },
          { id: "devices", n: "03", t: "Devices", d: "Manage enrolled devices, view trust status, revoke access" },
          { id: "recovery", n: "04", t: "Recovery", d: "Time-lock vault, 24hr countdown, and zero-trust recovery flow" },
          { id: "admin", n: "05", t: "Admin Panel", d: "Network topology, policy editor, audit log, system health" },
        ].map((c, i) => (
          <HomeCell key={c.id} {...c} go={go} full={i === 5} />
        ))}
      </div>
    </div>
  );
}

function HomeCell({ id, n, t, d, go, full }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)} onClick={() => go(id)}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", textAlign: "left", gridColumn: full ? "1/-1" : undefined }}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 12 }}>{n}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 28, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8, display: "flex", alignItems: "center", gap: 10 }}>
        {t} <span style={{ display: "inline-block", transition: "transform .25s", transform: hov ? "translateX(4px)" : "none", fontFamily: "var(--mono)" }}>→</span>
      </div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.65, fontWeight: 300 }}>{d}</div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   REGISTER
═══════════════════════════════════════════════════════════════ */
function Register({ go, toast }) {
  const [step, setStep] = useState(0);
  const [ident, setIdent] = useState("");
  const [identErr, setIdentErr] = useState(false);
  const [pkDone, setPkDone] = useState(false);

  const gotoStep = s => setStep(s);

  const nextStep = from => {
    if (from === 0) {
      if (!ident || ident.length < 2 || !/^[a-zA-Z0-9-]+$/.test(ident)) { setIdentErr(true); return; }
      setIdentErr(false); gotoStep(1);
    } else if (from === 1 && pkDone) gotoStep(2);
  };

  const activatePK = () => {
    setPkDone(true);
    toast("Passkey created — key sealed in hardware", "success");
  };

  const devClass = ["rv-device", "rv-device s2", "rv-device s3"][step];
  const devLabel = ["Waiting for identity...", "Creating passkey...", "Device enrolled ✓"][step];

  return (
    <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "3fr 2fr", minHeight: "100vh" }}>
      <div style={{ display: "flex", flexDirection: "column", justifyContent: "center", padding: "72px 80px" }}>
        <button onClick={() => go("home")} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", border: "none", cursor: "pointer", marginBottom: 48, display: "flex", alignItems: "center", gap: 10, transition: "color .2s" }}
          onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}>← Back to Home</button>

        {/* PIPS */}
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
            <div onClick={!pkDone ? activatePK : undefined} style={{
              width: "100%", padding: 28, border: pkDone ? "1px solid var(--success)" : "1px dashed rgba(200,245,90,.25)",
              background: pkDone ? "rgba(74,222,128,.08)" : "var(--accent-dim)", cursor: pkDone ? "default" : "pointer",
              textAlign: "center", transition: "all .25s", marginBottom: 28
            }}>
              <div style={{ fontSize: 36, marginBottom: 10 }}>{pkDone ? "✅" : "🔑"}</div>
              <div style={{ fontFamily: "var(--display)", fontSize: 22, letterSpacing: ".04em", textTransform: "uppercase", marginBottom: 6 }}>{pkDone ? "Passkey Created ✓" : "Create Passkey"}</div>
              <div style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300 }}>{pkDone ? "Hardware key sealed in secure enclave" : "Tap to trigger Face ID / Touch ID / hardware key"}</div>
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

      {/* VIZ PANEL */}
      <div style={{ background: "#080808", borderLeft: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "center", position: "relative", overflow: "hidden" }}>
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

/* ═══════════════════════════════════════════════════════════════
   DASHBOARD
═══════════════════════════════════════════════════════════════ */
const ORB_DATA = [
  { title: "All Secure", desc: "All enrolled devices active and verified. No suspicious activity detected in the last 24 hours. Last sweep: 2 minutes ago.", cls: "", ico: "🛡", type: "success" },
  { title: "Action Required", desc: "2 devices have not authenticated recently. Review inactive device access policies.", cls: "w", ico: "⚠", type: "warning" },
  { title: "Threat Detected", desc: "Suspicious authentication attempt detected on Work Desktop. Immediate review recommended.", cls: "r", ico: "🚨", type: "danger" },
];

function Dashboard({ go, toast }) {
  const [orbIdx, setOrbIdx] = useState(0);
  const orb = ORB_DATA[orbIdx];

  useReveal([orbIdx]);

  const setOrb = i => {
    setOrbIdx(i);
    const msgs = ["All systems secure ✓", "Action recommended — review inactive devices", "ALERT: Threat detected — investigate immediately"];
    toast(msgs[i], ORB_DATA[i].type);
  };

  const orbStyle = orb.cls === "w"
    ? { background: "radial-gradient(circle at 35% 35%,rgba(251,191,36,.85),rgba(251,191,36,.35),transparent)", animation: "orbWarn 1.5s ease-in-out infinite" }
    : orb.cls === "r"
      ? { background: "radial-gradient(circle at 35% 35%,rgba(248,113,113,.85),rgba(248,113,113,.35),transparent)", animation: "orbDanger .9s ease-in-out infinite" }
      : { background: "radial-gradient(circle at 35% 35%,rgba(74,222,128,.85),rgba(74,222,128,.35),rgba(5,150,105,.1))", animation: "orbPulse 3s ease-in-out infinite" };

  return (
    <AppShell active="dashboard" go={go}>
      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Dashboard</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Security overview</div></div>
        <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Enroll Device</BtnF>
      </div>
      <div style={{ padding: "36px 44px 60px", flex: 1 }}>
        {/* STAT CARDS */}
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 1, background: "var(--line)", marginBottom: 28, border: "1px solid var(--line)" }}>
          {[
            { l: "Active Devices", v: "3", d: "↑ 1 this week", i: "📱" },
            { l: "Auth Events (24h)", v: "47", d: "All verified", i: "⚡" },
            { l: "Security Score", v: "98%", d: "No issues found", i: "🛡", vc: "var(--success)" },
          ].map((s, i) => <StatCard key={i} {...s} />)}
        </div>

        {/* ORB */}
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "auto 1fr", border: "1px solid var(--line)", marginBottom: 28, background: "var(--ink-2)" }}>
          <div onClick={() => setOrb((orbIdx + 1) % 3)} style={{ width: 180, display: "flex", alignItems: "center", justifyContent: "center", padding: 40, borderRight: "1px solid var(--line)", cursor: "pointer", position: "relative" }}>
            <div className="orb-pulse r1" style={{ position: "absolute" }} />
            <div className="orb-pulse r2" style={{ position: "absolute" }} />
            <div style={{ width: 96, height: 96, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 32, position: "relative", zIndex: 1, ...orbStyle }}>{orb.ico}</div>
          </div>
          <div style={{ padding: "36px 40px", display: "flex", flexDirection: "column", justifyContent: "center" }}>
            <div style={{ fontFamily: "var(--display)", fontSize: 32, letterSpacing: ".06em", textTransform: "uppercase", marginBottom: 10 }}>{orb.title}</div>
            <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, marginBottom: 24, maxWidth: 500 }}>{orb.desc}</p>
            <div style={{ display: "flex", gap: 8 }}>
              {["Secure", "Warning", "Alert"].map((m, i) => (
                <button key={m} onClick={() => setOrb(i)} style={{
                  fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase",
                  padding: "7px 14px", border: `1px solid ${orbIdx === i ? "var(--accent)" : "var(--line2)"}`,
                  color: orbIdx === i ? "var(--accent)" : "var(--muted)", background: orbIdx === i ? "var(--accent-dim)" : "none", cursor: "pointer", transition: "all .2s"
                }}>{m}</button>
              ))}
            </div>
          </div>
        </div>

        {/* ACTIVITY */}
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 16, display: "flex", alignItems: "center", gap: 14 }} className="pg-in">
          Recent Activity<div style={{ flex: 1, height: 1, background: "var(--line)" }} />
        </div>
        <div className="pg-in" style={{ display: "flex", flexDirection: "column", border: "1px solid var(--line)" }}>
          {[
            { ico: "✓", t: "s", title: "Authentication successful", meta: "MacBook Pro · Chrome · San Francisco, CA", time: "2m ago" },
            { ico: "📱", t: "i", title: "New device enrolled", meta: "iPhone 15 Pro · Passkey created", time: "1h ago" },
            { ico: "✓", t: "s", title: "Authentication successful", meta: "iPad Air · Safari · New York, NY", time: "3h ago" },
            { ico: "⚠", t: "w", title: "Unrecognized device blocked", meta: "Unknown · Tokyo, JP · Request denied", time: "6h ago" },
            { ico: "🔒", t: "i", title: "Security sweep completed", meta: "All 3 devices verified · Zero anomalies", time: "12h ago" },
          ].map((a, i) => <ActivityItem key={i} {...a} />)}
        </div>
      </div>
    </AppShell>
  );
}

function StatCard({ l, v, d, i, vc }) {
  return (
    <div className="stat-c" style={{ background: "var(--ink-2)", padding: "28px 24px", position: "relative", overflow: "hidden", transition: "background .25s" }}
      onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 14 }}>{l}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 52, letterSpacing: ".02em", lineHeight: 1, color: vc }}>{v}</div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".06em", marginTop: 8 }}>{d}</div>
      <div style={{ position: "absolute", top: 20, right: 20, fontSize: 20, opacity: .25 }}>{i}</div>
    </div>
  );
}

function ActivityItem({ ico, t, title, meta, time }) {
  const icoStyle = {
    s: { borderColor: "rgba(74,222,128,.3)", background: "var(--s-success)" },
    w: { borderColor: "rgba(251,191,36,.3)", background: "var(--s-warning)" },
    i: { borderColor: "rgba(200,245,90,.2)", background: "var(--accent-dim)" },
  }[t] || {};
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 16, padding: "16px 20px", borderBottom: "1px solid var(--line)", background: "var(--ink-2)", transition: "background .15s" }}
      onMouseEnter={e => e.currentTarget.style.background = "var(--ink-3)"} onMouseLeave={e => e.currentTarget.style.background = "var(--ink-2)"}>
      <div style={{ width: 32, height: 32, flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 14, border: "1px solid var(--line)", ...icoStyle }}>{ico}</div>
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 13, fontWeight: 500 }}>{title}</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", marginTop: 2, letterSpacing: ".04em" }}>{meta}</div>
      </div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted2)", whiteSpace: "nowrap", letterSpacing: ".04em" }}>{time}</div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   DEVICES
═══════════════════════════════════════════════════════════════ */
function Devices({ go, toast }) {
  const [showRevoke, setShowRevoke] = useState(false);
  const [revTarget, setRevTarget] = useState(null);
  const [countdown, setCountdown] = useState(3);
  const [canRevoke, setCanRevoke] = useState(false);
  const timerRef = useRef(null);

  const openRevoke = name => {
    setRevTarget(name); setShowRevoke(true); setCountdown(3); setCanRevoke(false);
    timerRef.current = setInterval(() => {
      setCountdown(c => {
        if (c <= 1) { clearInterval(timerRef.current); setCanRevoke(true); return 0; }
        return c - 1;
      });
    }, 1000);
  };
  const closeRevoke = () => { clearInterval(timerRef.current); setShowRevoke(false); };
  const doRevoke = () => { closeRevoke(); toast("Device revoked — access blocked in <500ms", "danger"); };

  const devices = [
    { ico: "💻", name: "MacBook Pro", type: "Laptop · macOS 14", status: "active", enrolled: "Mar 1, 2026", last: "2 min ago", fp: "a3:f7:2c:91..." },
    { ico: "📱", name: "iPhone 15 Pro", type: "Phone · iOS 17", status: "active", enrolled: "Feb 28, 2026", last: "1 hr ago", fp: "b8:12:aa:5e..." },
    { ico: "🖥", name: "Work Desktop", type: "Desktop · Windows 11", status: "inactive", enrolled: "Jan 15, 2026", last: "5 days ago", fp: "c4:9d:0f:77..." },
  ];

  return (
    <AppShell active="devices" go={go}>
      {showRevoke && (
        <div onClick={e => { if (e.target === e.currentTarget) closeRevoke(); }} style={{ position: "fixed", inset: 0, zIndex: 5000, background: "rgba(0,0,0,.88)", backdropFilter: "blur(14px)", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <div className="modal-anim" style={{ background: "var(--ink-2)", border: "1px solid var(--line2)", padding: 48, maxWidth: 440, width: "90%", position: "relative" }}>
            <button onClick={closeRevoke} style={{ position: "absolute", top: 18, right: 18, background: "none", border: "none", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 16, cursor: "pointer" }}>×</button>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 16 }}>// Destructive Action</div>
            <h3 style={{ fontFamily: "var(--display)", fontSize: 44, textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .95, marginBottom: 14 }}>Revoke<br />Device?</h3>
            <p style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, marginBottom: 24, fontWeight: 300 }}>This device will lose all access immediately. Cannot be undone without re-enrollment.</p>
            <div style={{ background: "var(--s-danger)", border: "1px solid rgba(248,113,113,.2)", padding: 14, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".06em", color: "var(--danger)", marginBottom: 24 }}>⚠ DEVICE BLOCKED WITHIN 500MS OF CONFIRMATION</div>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 24 }}>
              <div style={{ width: 38, height: 38, position: "relative", flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", border: "2px solid var(--danger)", borderRadius: "50%" }}>
                <span style={{ fontFamily: "var(--display)", fontSize: 14, color: "var(--danger)" }}>{countdown}</span>
              </div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", color: "var(--muted)", lineHeight: 1.6 }}>Confirm button activates in {countdown}s —<br />mandatory delay for destructive operations</div>
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <BtnO onClick={closeRevoke} style={{ padding: "8px 16px", fontSize: 9 }}>Cancel</BtnO>
              <button onClick={doRevoke} disabled={!canRevoke} style={{ display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: canRevoke ? "pointer" : "not-allowed", opacity: canRevoke ? 1 : .5, transition: "all .25s" }}>Revoke Device</button>
            </div>
          </div>
        </div>
      )}

      <div style={{ padding: "36px 44px 28px", display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, borderBottom: "1px solid var(--line)" }}>
        <div><div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Devices</div><div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>Enrolled hardware · Trust registry</div></div>
        <BtnF onClick={() => go("register")} style={{ padding: "8px 16px", fontSize: 9 }}>+ Enroll New</BtnF>
      </div>
      <div style={{ padding: "36px 44px 60px" }}>
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(290px,1fr))", gap: 1, background: "var(--line)", border: "1px solid var(--line)" }}>
          {devices.map(d => <DeviceCard key={d.name} {...d} onRevoke={() => openRevoke(d.name)} toast={toast} />)}
        </div>
      </div>
    </AppShell>
  );
}

function DeviceCard({ ico, name, type, status, enrolled, last, fp, onRevoke, toast }) {
  const [hov, setHov] = useState(false);
  const ringC = status === "active" ? "var(--success)" : status === "revoked" ? "var(--danger)" : "var(--muted2)";
  const statusC = status === "active" ? { background: "var(--s-success)", color: "var(--success)" } : status === "revoked" ? { background: "var(--s-danger)", color: "var(--danger)" } : { background: "rgba(90,85,80,.15)", color: "var(--muted)" };
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: 28, transition: "background .25s", position: "relative", overflow: "hidden", opacity: status === "inactive" ? .75 : 1 }}>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ display: "flex", alignItems: "flex-start", gap: 16, marginBottom: 20 }}>
        <div className="dmodel" style={{ width: 52, height: 76, background: "linear-gradient(160deg,var(--ink-3),#080808)", border: "1px solid var(--line2)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 26, flexShrink: 0, position: "relative", opacity: status === "inactive" ? .5 : 1 }}>
          {ico}
          <div style={{ position: "absolute", bottom: -3, left: "50%", transform: "translateX(-50%)", width: 40, height: 4, background: ringC, boxShadow: status === "active" ? `0 0 10px ${ringC}` : "none" }} />
        </div>
        <div>
          <div style={{ fontSize: 15, fontWeight: 500, marginBottom: 4 }}>{name}</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--muted)" }}>{type}</div>
          <div style={{ display: "inline-flex", alignItems: "center", gap: 5, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "3px 8px", marginTop: 8, ...statusC }}>● {status.charAt(0).toUpperCase() + status.slice(1)}</div>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 20, paddingTop: 16, borderTop: "1px solid var(--line)" }}>
        {[["Enrolled", enrolled], ["Last active", last], ["Fingerprint", fp]].map(([l, v]) => (
          <div key={l} style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", textTransform: "uppercase", color: "var(--muted2)" }}>{l}</span>
            <span style={{ fontSize: 12, fontFamily: l === "Fingerprint" ? "var(--mono)" : "var(--body)" }}>{v}</span>
          </div>
        ))}
      </div>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        <BtnO onClick={() => toast("Rename — full app only", "info")} style={{ padding: "8px 16px", fontSize: 9 }}>Rename</BtnO>
        <BtnO onClick={() => toast("Activity loaded", "info")} style={{ padding: "8px 16px", fontSize: 9 }}>Activity</BtnO>
        <button onClick={onRevoke} style={{ display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--danger)", background: "var(--s-danger)", padding: "8px 16px", border: "1px solid rgba(248,113,113,.25)", cursor: "pointer", transition: "all .25s" }}>Revoke</button>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   RECOVERY
═══════════════════════════════════════════════════════════════ */
function Recovery({ go, toast }) {
  const [secs, setSecs] = useState(86400 - 76);
  const [done, setDone] = useState(false);
  const intRef = useRef(null);

  useEffect(() => {
    intRef.current = setInterval(() => setSecs(s => s > 0 ? s - 1 : 0), 1000);
    return () => clearInterval(intRef.current);
  }, []);

  const h = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60), s = secs % 60;
  const timeStr = `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
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
      <div style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
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

/* ═══════════════════════════════════════════════════════════════
   ADMIN
═══════════════════════════════════════════════════════════════ */
function NetworkCanvas() {
  const ref = useRef(null);
  useEffect(() => {
    const canvas = ref.current; if (!canvas) return;
    const ctx = canvas.getContext("2d");
    canvas.width = canvas.offsetWidth; canvas.height = 220;
    const W = canvas.width, H = canvas.height;
    const nodes = [
      { x: W * .18, y: H * .5, label: "MacBook Pro", c: "#4ADE80" },
      { x: W * .42, y: H * .2, label: "iPhone 15", c: "#4ADE80" },
      { x: W * .75, y: H * .45, label: "iPad Air", c: "#4ADE80" },
      { x: W * .5, y: H * .72, label: "Server", c: "#C8F55A" },
      { x: W * .3, y: H * .38, label: "Work Desktop", c: "#5A5550" },
    ];
    const edges = [[0, 3], [1, 3], [2, 3], [4, 3], [0, 1]];
    let t = 0, raf;
    const draw = () => {
      ctx.clearRect(0, 0, W, H);
      edges.forEach(([a, b]) => {
        ctx.beginPath(); ctx.moveTo(nodes[a].x, nodes[a].y); ctx.lineTo(nodes[b].x, nodes[b].y);
        ctx.strokeStyle = "rgba(244,241,236,0.06)"; ctx.lineWidth = 1; ctx.stroke();
        const p = (Math.sin(t * .025 + a) + 1) / 2;
        const px = nodes[a].x + (nodes[b].x - nodes[a].x) * p, py = nodes[a].y + (nodes[b].y - nodes[a].y) * p;
        ctx.beginPath(); ctx.arc(px, py, 2.5, 0, Math.PI * 2); ctx.fillStyle = "rgba(200,245,90,.7)"; ctx.fill();
      });
      nodes.forEach(n => {
        const g = ctx.createRadialGradient(n.x, n.y, 0, n.x, n.y, 22);
        g.addColorStop(0, n.c + "30"); g.addColorStop(1, "transparent");
        ctx.beginPath(); ctx.arc(n.x, n.y, 22, 0, Math.PI * 2); ctx.fillStyle = g; ctx.fill();
        ctx.beginPath(); ctx.arc(n.x, n.y, 6, 0, Math.PI * 2); ctx.fillStyle = n.c; ctx.fill();
        ctx.fillStyle = "rgba(244,241,236,.5)"; ctx.font = "10px DM Mono, monospace"; ctx.textAlign = "center";
        ctx.fillText(n.label, n.x, n.y + 22);
      });
      t++; raf = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, []);
  return <canvas ref={ref} style={{ display: "block", width: "100%", height: 220 }} height={220} />;
}

function Admin({ go, toast }) {
  return (
    <AppShell active="admin" go={go}>
      <div style={{ padding: "36px 44px 28px", borderBottom: "1px solid var(--line)" }}>
        <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".06em", textTransform: "uppercase" }}>Admin</div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginTop: 6 }}>System management · Analytics</div>
      </div>
      <div style={{ padding: "36px 44px 60px" }}>
        <div className="pg-in" style={{ border: "1px solid var(--line)", background: "var(--ink-2)", marginBottom: 28, overflow: "hidden" }}>
          <div style={{ padding: "18px 22px", borderBottom: "1px solid var(--line)", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)" }}>Network Topology — Live Trust Graph</span>
            <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--success)", letterSpacing: ".06em" }}>● 3 nodes active</span>
          </div>
          <NetworkCanvas />
        </div>
        <div className="pg-in" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 1, background: "var(--line)", border: "1px solid var(--line)", marginBottom: 28 }}>
          {[
            { ico: "📜", t: "Policy Editor", b: "Visual rule builder for access policies. Configure deny-by-default and step-up verification rules.", msg: "Policy Editor — full implementation pending" },
            { ico: "👥", t: "User Directory", b: "Searchable user list with device counts, last active timestamps, and trust scores.", msg: "User Directory — full implementation pending" },
            { ico: "🗂", t: "Audit Log", b: "Full cryptographic event stream. Export to CSV or JSON for compliance reporting.", msg: "Audit log export triggered", type: "success" },
            { ico: "📊", t: "System Health", b: "Uptime monitoring, response times, WebAuthn success rates, error dashboards.", msg: "All services operational ✓", type: "success" },
          ].map(c => <AdminCard key={c.t} {...c} toast={toast} />)}
        </div>
        <div className="pg-in" style={{ border: "1px solid rgba(248,113,113,.15)", background: "var(--ink-2)", padding: 28, cursor: "pointer" }}
          onClick={() => toast("Emergency lockdown requires multi-device confirmation", "danger")}>
          <div style={{ fontSize: 28, marginBottom: 18 }}>🚨</div>
          <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>Danger Zone</div>
          <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>Global device revocation and emergency system lockdown. Requires multi-device cryptographic confirmation.</div>
        </div>
      </div>
    </AppShell>
  );
}

function AdminCard({ ico, t, b, msg, type = "info", toast }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)} onClick={() => toast(msg, type)}
      style={{ background: hov ? "var(--ink-3)" : "var(--ink-2)", padding: "32px 28px", cursor: "pointer", transition: "background .25s", position: "relative", overflow: "hidden" }}>
      <div style={{ position: "absolute", bottom: 0, left: 0, right: 0, height: 1, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .4s cubic-bezier(.16,1,.3,1)" }} />
      <div style={{ fontSize: 28, marginBottom: 18 }}>{ico}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 24, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8 }}>{t}</div>
      <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{b}</div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════
   ROOT
═══════════════════════════════════════════════════════════════ */
export default function App() {
  const [page, setPage] = useState("home");
  const [toasts, addToast] = useToasts();

  const go = useCallback(id => {
    setPage(id);
    window.scrollTo({ top: 0 });
  }, []);

  const toast = useCallback((msg, type = "info") => addToast(msg, type), [addToast]);

  useEffect(() => {
    setTimeout(() => toast("CRYPTON — Zero passwords. Zero trust.", "info"), 800);
  }, []);

  return (
    <>
      <FontLink />
      <div className="grain" />
      <Cursor />
      <ToastStack toasts={toasts} />

      {page === "home" && <Home go={go} />}
      {page === "landing" && <Landing go={go} toast={toast} />}
      {page === "register" && <Register go={go} toast={toast} />}
      {page === "dashboard" && <Dashboard go={go} toast={toast} />}
      {page === "devices" && <Devices go={go} toast={toast} />}
      {page === "recovery" && <Recovery go={go} toast={toast} />}
      {page === "admin" && <Admin go={go} toast={toast} />}
    </>
  );
}
