import { useState, useEffect, useRef, useCallback } from "react";
import { BtnF, BtnO } from './Buttons';
import { useReveal } from './hooks';
import { TOKEN_KEY } from './auth';

/* ═══════════════════════════════════════════════════════════════
   LANDING PAGE — SPHERE ENGINE + INTRO
═══════════════════════════════════════════════════════════════ */

/* Module-level flag — resets on hard refresh, persists within same tab session */
let _introHasPlayed = false;

/* shared ease helpers */
const easeOutCubic = t => 1 - Math.pow(1 - t, 3);
const easeInOutCubic = t => t < .5 ? 4*t*t*t : 1 - Math.pow(-2*t+2,3)/2;
const clampT = (v,a,b) => Math.max(a, Math.min(b, v));

function useSphereIntro() {
  /* Returns { canvasRef, introVisible, introPhase }
     Manages the full intro sequence imperatively via canvas + DOM refs */
  const canvasRef = useRef(null);
  const stateRef = useRef({
    W: 0, H: 0,
    rotation: 0, rotSpeed: 0.004,
    sphereAlpha: 0,
    transitionT: 0,      // 0=center 1=settled
    phase: "idle",       // idle → fadein → wordmark → transition → done
    frame: 0,
    arcs: [], arcTimer: 0,
    mouse: { x: 0, y: 0 },
  });
  const introRef = useRef(null);   // the intro overlay div
  const heroRef  = useRef(null);   // hero content div
  const navRef   = useRef(null);

  const isMobile = () => window.innerWidth <= 767;

  /* build static dot/ring/particle arrays once */
  const globeRef = useRef(null);
  if (!globeRef.current) {
    const dots = [];
    const latStep = isMobile() ? 14 : 10;
    const lonStep = isMobile() ? 14 : 10;
    for (let lat = -80; lat <= 80; lat += latStep)
      for (let lon = 0; lon < 360; lon += lonStep)
        dots.push({
          phi: (lat*Math.PI)/180, theta: (lon*Math.PI)/180,
          size: Math.random()*1.2+0.4, brightness: Math.random()*.5+.5,
          pulse: Math.random()*Math.PI*2, pulseSpeed: .02+Math.random()*.03
        });
    const rings = [
      { tilt:.3,  speed:.007,  angle:0,   r:1.18, opacity:.35, dash:[8,6]  },
      { tilt:-.5, speed:-.005, angle:1.2, r:1.28, opacity:.25, dash:[4,10] },
      { tilt:.9,  speed:.009,  angle:2.4, r:1.12, opacity:.2,  dash:[12,8] },
    ];
    const pCount = isMobile() ? 30 : 60;
    const particles = Array.from({ length: pCount }, () => ({
      ring: Math.floor(Math.random()*3),
      angle: Math.random()*Math.PI*2,
      speed: (Math.random()*.008+.004) * (Math.random()>.5?1:-1),
      size: Math.random()*2+1, brightness: Math.random(),
      color: Math.random()>.5 ? "#C8F55A" : "#4ADE80",
    }));
    globeRef.current = { dots, rings, particles };
  }

  const getSphereParams = (s) => {
    const { W, H, transitionT } = s;
    const mobile = W <= 767;
    const introR   = Math.min(W,H) * (mobile ? .36 : .38);
    const settledR = Math.min(W,H) * (mobile ? .38 : .32);
    const iCx = W/2, iCy = H/2;
    const sCx = mobile ? W/2 : W*.62;
    const sCy = mobile ? H*.38 : H*.48;
    const t = easeInOutCubic(transitionT);
    return {
      r:  introR  + (settledR - introR)  * t,
      cx: iCx    + (sCx     - iCx)     * t,
      cy: iCy    + (sCy     - iCy)     * t,
    };
  };

  const project = (phi, theta, rot, cx, cy, r) => {
    const x3 = Math.cos(phi)*Math.sin(theta+rot);
    const y3 = Math.sin(phi);
    const z3 = Math.cos(phi)*Math.cos(theta+rot);
    const p = 2.8, sc = p/(p+z3*.4);
    return { x: cx+x3*r*sc, y: cy-y3*r*sc, z: z3, sc };
  };

  const lerpSphere = (a, b, t, rot, cx, cy, r) => {
    const ax=Math.cos(a.phi)*Math.sin(a.theta), ay=Math.sin(a.phi), az=Math.cos(a.phi)*Math.cos(a.theta);
    const bx=Math.cos(b.phi)*Math.sin(b.theta), by=Math.sin(b.phi), bz=Math.cos(b.phi)*Math.cos(b.theta);
    const dot=Math.min(1,ax*bx+ay*by+az*bz), omega=Math.acos(dot);
    let rx,ry,rz;
    if(omega<.001){rx=ax+t*(bx-ax);ry=ay+t*(by-ay);rz=az+t*(bz-az);}
    else{const s=Math.sin(omega),sa=Math.sin((1-t)*omega)/s,sb=Math.sin(t*omega)/s;rx=sa*ax+sb*bx;ry=sa*ay+sb*by;rz=sa*az+sb*bz;}
    const lift=1.06+Math.sin(t*Math.PI)*.12; rx*=lift;ry*=lift;rz*=lift;
    const p=2.8,sc=p/(p+rz*.4);
    return { x:cx+rx*r*sc, y:cy-ry*r*sc, z:rz, sc };
  };

  const spawnArc = (s) => {
    const d = globeRef.current.dots;
    const a=d[Math.floor(Math.random()*d.length)], b=d[Math.floor(Math.random()*d.length)];
    s.arcs.push({ from:{phi:a.phi,theta:a.theta}, to:{phi:b.phi,theta:b.theta},
      progress:0, speed:.008+Math.random()*.006, life:1, fadeSpeed:.012 });
  };

  /* draw one frame */
  const drawFrame = (ctx, s) => {
    const { W, H, rotation, sphereAlpha, arcs } = s;
    /* Guard against uninitialized dimensions — prevents non-finite gradient errors */
    if (!W || !H || W <= 0 || H <= 0 || !isFinite(W) || !isFinite(H)) return;
    const { dots, rings, particles } = globeRef.current;
    const sp = getSphereParams(s);
    const { r, cx, cy } = sp;
    if (!r || !isFinite(r) || !isFinite(cx) || !isFinite(cy)) return;

    ctx.clearRect(0,0,W,H);

    /* bg glow */
    const bg = ctx.createRadialGradient(cx,cy,0,cx,cy,r*1.8);
    bg.addColorStop(0,`rgba(16,28,16,${.5*sphereAlpha})`);
    bg.addColorStop(1,"transparent");
    ctx.fillStyle=bg; ctx.fillRect(0,0,W,H);

    const sg = ctx.createRadialGradient(cx,cy,0,cx,cy,r*1.1);
    sg.addColorStop(0,`rgba(74,222,128,${.04*sphereAlpha})`);
    sg.addColorStop(1,"transparent");
    ctx.fillStyle=sg; ctx.beginPath(); ctx.arc(cx,cy,r*1.1,0,Math.PI*2); ctx.fill();

    /* rings */
    rings.forEach(ring => {
      ring.angle += ring.speed;
      const steps=120, pts=[];
      for(let i=0;i<=steps;i++){
        const a=(i/steps)*Math.PI*2+ring.angle;
        const rx=Math.cos(a)*ring.r, ry=Math.sin(a)*Math.cos(ring.tilt)*ring.r, rz=Math.sin(a)*Math.sin(ring.tilt)*ring.r;
        const rX=rx*Math.cos(rotation)+rz*Math.sin(rotation), rZ=-rx*Math.sin(rotation)+rz*Math.cos(rotation);
        const p=2.8,sc=p/(p+rZ*.4);
        pts.push({ x:cx+rX*r*sc, y:cy-ry*r*sc });
      }
      ctx.save(); ctx.globalAlpha=sphereAlpha; ctx.setLineDash(ring.dash);
      ctx.strokeStyle=`rgba(200,245,90,${ring.opacity})`; ctx.lineWidth=.8;
      ctx.beginPath(); pts.forEach((p,i)=>i===0?ctx.moveTo(p.x,p.y):ctx.lineTo(p.x,p.y)); ctx.stroke();
      ctx.restore();
      /* travelling dot on ring */
      const da=Math.cos(ring.angle)*ring.r, db=Math.sin(ring.angle)*Math.cos(ring.tilt)*ring.r, dc=Math.sin(ring.angle)*Math.sin(ring.tilt)*ring.r;
      const dX=da*Math.cos(rotation)+dc*Math.sin(rotation), dZ=-da*Math.sin(rotation)+dc*Math.cos(rotation);
      const p2=2.8,dSc=p2/(p2+dZ*.4);
      const px=cx+dX*r*dSc, py=cy-db*r*dSc;
      const g2=ctx.createRadialGradient(px,py,0,px,py,6*dSc);
      g2.addColorStop(0,"rgba(200,245,90,0.9)"); g2.addColorStop(1,"transparent");
      ctx.save(); ctx.globalAlpha=sphereAlpha;
      ctx.beginPath(); ctx.arc(px,py,5*dSc,0,Math.PI*2); ctx.fillStyle=g2; ctx.fill();
      ctx.restore();
    });

    /* globe dots */
    const visible=[];
    dots.forEach(dot=>{
      dot.pulse+=dot.pulseSpeed;
      const p=project(dot.phi,dot.theta,rotation,cx,cy,r);
      if(p.z>-0.1) visible.push({...p,dot});
    });
    visible.sort((a,b)=>a.z-b.z);
    visible.forEach(({x,y,z,sc,dot})=>{
      const df=(z+1)/2, pulse=.6+.4*Math.sin(dot.pulse), alpha=df*dot.brightness*pulse*sphereAlpha;
      const size=dot.size*sc*(.5+df*.8);
      const isAcc=dot.brightness>.8&&df>.7;
      const color=isAcc?`rgba(200,245,90,${alpha})`:`rgba(74,222,128,${alpha*.7})`;
      if(size>.3){
        const g=ctx.createRadialGradient(x,y,0,x,y,size*2);
        g.addColorStop(0,color); g.addColorStop(1,"transparent");
        ctx.beginPath(); ctx.arc(x,y,size*2,0,Math.PI*2); ctx.fillStyle=g; ctx.fill();
        ctx.beginPath(); ctx.arc(x,y,size,0,Math.PI*2); ctx.fillStyle=color; ctx.fill();
      }
    });

    /* arcs */
    if(sphereAlpha>.4){
      s.arcTimer++;
      if(s.arcTimer>80){ spawnArc(s); s.arcTimer=0; }
      for(let i=arcs.length-1;i>=0;i--){
        const arc=arcs[i];
        arc.progress=Math.min(1,arc.progress+arc.speed);
        if(arc.progress>=1) arc.life-=arc.fadeSpeed;
        if(arc.life<=0){ arcs.splice(i,1); continue; }
        const steps=40,du=Math.floor(arc.progress*steps);
        ctx.save(); ctx.globalAlpha=arc.life*.6*sphereAlpha;
        let prev=null;
        for(let k=0;k<=du;k++){
          const t2=k/steps, pt=lerpSphere(arc.from,arc.to,t2,rotation,cx,cy,r);
          if(pt.z<-.05){prev=null;continue;}
          if(prev&&prev.z>-.05){
            const sa=(1-Math.abs(t2-.5)*2)*.9;
            ctx.strokeStyle=`rgba(200,245,90,${sa})`; ctx.lineWidth=1.2*pt.sc;
            ctx.beginPath(); ctx.moveTo(prev.x,prev.y); ctx.lineTo(pt.x,pt.y); ctx.stroke();
          }
          if(k===du){
            const g=ctx.createRadialGradient(pt.x,pt.y,0,pt.x,pt.y,6);
            g.addColorStop(0,"rgba(200,245,90,1)"); g.addColorStop(1,"transparent");
            ctx.fillStyle=g; ctx.beginPath(); ctx.arc(pt.x,pt.y,5,0,Math.PI*2); ctx.fill();
          }
          prev=pt;
        }
        ctx.restore();
      }
    }

    /* particles */
    particles.forEach(p=>{
      p.angle+=p.speed;
      const ring=rings[p.ring];
      const rx=Math.cos(p.angle)*ring.r, ry=Math.sin(p.angle)*Math.cos(ring.tilt)*ring.r, rz=Math.sin(p.angle)*Math.sin(ring.tilt)*ring.r;
      const pX=rx*Math.cos(rotation)+rz*Math.sin(rotation), pZ=-rx*Math.sin(rotation)+rz*Math.cos(rotation);
      const pp=2.8,pSc=pp/(pp+pZ*.4);
      const ppx=cx+pX*r*pSc, ppy=cy-ry*r*pSc;
      const df=(pZ+1)/2, alpha=(.4+p.brightness*.6)*df*sphereAlpha;
      ctx.globalAlpha=alpha;
      ctx.beginPath(); ctx.arc(ppx,ppy,p.size*pSc*.8,0,Math.PI*2);
      ctx.fillStyle=p.color; ctx.fill();
      ctx.globalAlpha=1;
    });

    /* rotate */
    const mx=(s.mouse.x-W/2)/W;
    const targetRS=.004+mx*.003;
    s.rotSpeed+=(targetRS-s.rotSpeed)*.05;
    s.rotation+=s.rotSpeed;
    s.frame++;
  };

  /* animate one value with a promise */
  const animVal = (setter, from, to, duration, ease=easeOutCubic) =>
    new Promise(resolve=>{
      const start=performance.now();
      const tick=()=>{
        const t=clampT((performance.now()-start)/duration,0,1);
        setter(from+(to-from)*ease(t));
        if(t<1) requestAnimationFrame(tick); else { setter(to); resolve(); }
      };
      requestAnimationFrame(tick);
    });

  /* DOM helper */
  const anim = (el, kf, opts) => {
    if(!el) return Promise.resolve();
    return el.animate(kf,{fill:"forwards",...opts}).finished;
  };

  /* MAIN INTRO SEQUENCE */
  const runIntro = useCallback(async () => {
    const s = stateRef.current;
    s.phase="fadein"; s.sphereAlpha=0; s.transitionT=0; s.rotation=0;

    /* Phase 1 — sphere fades in centered */
    await animVal(v=>{ s.sphereAlpha=v; }, 0, 1, 900, easeOutCubic);
    await new Promise(r=>setTimeout(r,100));

    /* Phase 2 — wordmark + bar + tagline */
    const intro = introRef.current;
    if(intro){
      const wm = intro.querySelector(".intro-wm");
      const bar = intro.querySelector(".intro-bar");
      const tag = intro.querySelector(".intro-tag");
      if(wm) await anim(wm,[{opacity:0,transform:"scale(.94) translateY(12px)"},{opacity:1,transform:"scale(1) translateY(0)"}],{duration:700,easing:"cubic-bezier(.16,1,.3,1)"});
      await new Promise(r=>setTimeout(r,180));
      if(bar) await anim(bar,[{transform:"scaleX(0)",transformOrigin:"left"},{transform:"scaleX(1)",transformOrigin:"left"}],{duration:600,easing:"cubic-bezier(.16,1,.3,1)"});
      await new Promise(r=>setTimeout(r,300));
      if(tag) await anim(tag,[{opacity:0,transform:"translateY(8px)"},{opacity:1,transform:"translateY(0)"}],{duration:500,easing:"ease"});
    }
    await new Promise(r=>setTimeout(r,820));

    /* Phase 3 — sphere moves, intro dissolves, hero content arrives */
    s.phase="transition";

    const intro2 = introRef.current;
    if(intro2){
      const wm=intro2.querySelector(".intro-wm");
      const tag=intro2.querySelector(".intro-tag");
      if(wm) wm.animate([{opacity:1,transform:"scale(1)"},{opacity:0,transform:"scale(.9) translateY(-18px)"}],{duration:550,easing:"cubic-bezier(.4,0,1,1)",fill:"forwards"});
      if(tag) tag.animate([{opacity:1},{opacity:0}],{duration:380,fill:"forwards"});
    }

    /* sphere transition: 0→1 over 900ms */
    animVal(v=>{ s.transitionT=v; }, 0, 1, 900, easeInOutCubic);

    await new Promise(r=>setTimeout(r,180));

    /* hero gradient */
    const hg = heroRef.current?.querySelector(".hero-gradient");
    if(hg) hg.animate([{opacity:0},{opacity:1}],{duration:800,easing:"ease",fill:"forwards"});

    /* nav */
    const nav = navRef.current;
    if(nav) nav.animate([{opacity:0,transform:"translateY(-16px)"},{opacity:1,transform:"translateY(0)"}],{duration:700,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});

    await new Promise(r=>setTimeout(r,150));

    /* hero content */
    const hc = heroRef.current;
    if(hc){
      const label = hc.querySelector(".hero-label-inner");
      const line1 = hc.querySelector(".hl1");
      const line2 = hc.querySelector(".hl2");
      const meta  = hc.querySelector(".hero-meta");
      const labelWrap = hc.querySelector(".hero-label-wrap");
      if(labelWrap) labelWrap.animate([{opacity:0},{opacity:1}],{duration:300,fill:"forwards"});
      if(label) label.animate([{transform:"translateY(100%)"},{transform:"translateY(0)"}],{duration:700,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
      await new Promise(r=>setTimeout(r,100));
      if(line1) line1.animate([{transform:"translateY(110%)"},{transform:"translateY(0)"}],{duration:1000,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
      await new Promise(r=>setTimeout(r,70));
      if(line2) line2.animate([{transform:"translateY(110%)"},{transform:"translateY(0)"}],{duration:1000,easing:"cubic-bezier(.16,1,.3,1)",fill:"forwards"});
      await new Promise(r=>setTimeout(r,380));
      if(meta) meta.animate([{opacity:0,transform:"translateY(14px)"},{opacity:1,transform:"translateY(0)"}],{duration:700,easing:"ease",fill:"forwards"});
    }

    s.phase="done";
  }, []);

  useEffect(()=>{
    const canvas = canvasRef.current; if(!canvas) return;
    const ctx = canvas.getContext("2d");
    const s = stateRef.current;

    const resize = () => {
      const w = canvas.offsetWidth, h = canvas.offsetHeight;
      if (w > 0 && h > 0) {
        s.W = canvas.width = w;
        s.H = canvas.height = h;
      }
      s.mouse.x = s.W/2; s.mouse.y = s.H/2;
    };
    /* Do NOT call resize() here — canvas may have 0 dimensions before paint.
       introRO will do the first sizing. */

    const onMouse = e => { s.mouse.x=e.clientX; s.mouse.y=e.clientY; };
    const onTouch = e => { if(e.touches[0]){ s.mouse.x=e.touches[0].clientX; } };
    window.addEventListener("resize", resize);
    window.addEventListener("mousemove", onMouse);
    window.addEventListener("touchmove", onTouch, {passive:true});

    let raf;
    const loop = () => {
      /* Only draw once canvas has been sized by introRO */
      if (s.W > 0 && s.H > 0) drawFrame(ctx, s);
      raf = requestAnimationFrame(loop);
    };
    loop();

    /* Size the canvas using window dimensions (canvas is position:fixed).
       On first tab load: play full intro.
       On return visits within same tab (e.g. back from dashboard): settle instantly. */
    const startIntro = () => {
      const w = window.innerWidth, h = window.innerHeight;
      if (!w || !h) { requestAnimationFrame(startIntro); return; }
      /* Set canvas pixel dimensions */
      s.W = canvas.width = w;
      s.H = canvas.height = h;
      if (_introHasPlayed) {
        /* Return visit — snap to settled state, no animation */
        s.sphereAlpha = 1; s.transitionT = 1; s.phase = "done";
        const intro = introRef.current;
        if (intro) { intro.style.opacity = "0"; intro.style.pointerEvents = "none"; }
        const nav = navRef.current;
        if (nav) { nav.style.opacity = "1"; nav.style.transform = "translateY(0)"; }
        /* Wait one more frame so DOM elements exist before applying styles */
        requestAnimationFrame(() => {
          const hc = heroRef.current;
          if (hc) {
            [".hero-gradient",".hero-label-wrap",".hero-meta"].forEach(sel => {
              const el = hc.querySelector(sel); if (el) el.style.opacity = "1";
            });
            [".hl1",".hl2",".hero-label-inner"].forEach(sel => {
              const el = hc.querySelector(sel); if (el) el.style.transform = "translateY(0)";
            });
          }
        });
      } else {
        /* First load — run the full intro sequence */
        document.fonts.ready.then(() => setTimeout(() => {
          runIntro().then(() => { _introHasPlayed = true; });
        }, 120));
      }
    };
    requestAnimationFrame(startIntro);

    return ()=>{
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
      window.removeEventListener("mousemove", onMouse);
      window.removeEventListener("touchmove", onTouch);
    };
  }, [runIntro]);

  return { canvasRef, introRef, heroRef, navRef };
}


export default function Landing({ go, toast, openSim }) {
  useReveal([]);
  const [scrolled, setScrolled] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [wlOpen,  setWlOpen]  = useState(false);
  const [wlEmail, setWlEmail] = useState('');
  const [wlDone,  setWlDone]  = useState(false);
  const handleWaitlist = () => {
    const email = wlEmail.trim();
    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return;
    try {
      const w = JSON.parse(localStorage.getItem('crypton_waitlist') || '[]');
      if (!w.includes(email)) { w.push(email); localStorage.setItem('crypton_waitlist', JSON.stringify(w)); }
    } catch {}
    setWlDone(true);
  };
  const { canvasRef, introRef, heroRef, navRef } = useSphereIntro();

  // Auth-aware nav — check if user has a session
  const isLoggedIn = (() => {
    try { return !!localStorage.getItem(TOKEN_KEY); } catch { return false; }
  })();

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 40);
    window.addEventListener("scroll", onScroll);
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  const scrollTo = id => {
    setMenuOpen(false);
    const el = document.getElementById(id);
    if (el) el.scrollIntoView({ behavior: "smooth" });
  };

  const NAV_LINKS = [
    { label: "Features",  target: "features"  },
    { label: "Protocol",  target: "protocol"  },
    { label: "Customers", target: "who"       },
    { label: "Attacks",   target: "attacks"   },
    { label: "Pricing",   target: "pricing"   },
    { label: "About",     target: "about"     },
  ];

  return (
    <div style={{ background: "var(--ink)" }}>

      {/* ── SPHERE CANVAS ── */}
      <canvas ref={canvasRef} style={{ position: "fixed", inset: 0, width: "100%", height: "100%", zIndex: 0 }} />

      {/* ── INTRO OVERLAY ── */}
      <div ref={introRef} style={{ position: "fixed", inset: 0, zIndex: 500, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>
        <div className="intro-wm" style={{ fontFamily: "var(--display)", fontSize: "clamp(48px,10vw,120px)", letterSpacing: ".14em", textTransform: "uppercase", opacity: 0, position: "relative", textAlign: "center" }}>
          CRYPTON
          <div className="intro-bar" style={{ position: "absolute", bottom: -10, left: 0, right: 0, height: 2, background: "var(--accent)", transform: "scaleX(0)" }} />
        </div>
        <div className="intro-tag" style={{ fontFamily: "var(--mono)", fontSize: "clamp(9px,1.5vw,11px)", letterSpacing: ".22em", textTransform: "uppercase", color: "var(--accent)", marginTop: 20, opacity: 0 }}>
          Zero-trust · Device identity · No passwords
        </div>
      </div>

      {/* ── NAV ── */}
      <div ref={navRef} className={`landing-nav${scrolled ? " scrolled" : ""}`} style={{ opacity: 0, zIndex: 600 }}>
        <a onClick={() => scrollTo("hero")} style={{ fontFamily: "var(--display)", fontSize: 20, letterSpacing: ".14em", color: "var(--paper)", textDecoration: "none", cursor: "pointer" }}>CRYPTON</a>
        <ul className="nav-links-wrap" style={{ display: "flex", gap: 36, listStyle: "none" }}>
          {NAV_LINKS.map(l => (
            <li key={l.label}>
              <button onClick={() => scrollTo(l.target)} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none", border: "none", opacity: .7, transition: "opacity .2s", cursor: "pointer" }}
                onMouseEnter={e => e.target.style.opacity = 1} onMouseLeave={e => e.target.style.opacity = .7}>{l.label}</button>
            </li>
          ))}
        </ul>
        <div className="nav-desktop-btns" style={{ display: "flex", gap: 10 }}>
          {isLoggedIn
            ? <button onClick={() => go('demo')} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--paper)", padding: "10px 22px", border: "none", cursor: "pointer", transition: "background .2s" }}
                onMouseEnter={e => e.currentTarget.style.background = "var(--accent)"}
                onMouseLeave={e => e.currentTarget.style.background = "var(--paper)"}>Open Demo</button>
            : <>
                <button onClick={() => go('demo')} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", background: "none", padding: "10px 16px", border: "1px solid var(--line2)", cursor: "pointer", transition: "color .2s" }}
                  onMouseEnter={e => e.currentTarget.style.color = "var(--paper)"} onMouseLeave={e => e.currentTarget.style.color = "var(--muted)"}> Try Demo</button>
                <button onClick={() => setWlOpen(true)} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--paper)", padding: "10px 22px", border: "none", cursor: "pointer", transition: "background .2s" }}
                  onMouseEnter={e => e.currentTarget.style.background = "var(--accent)"}
                  onMouseLeave={e => e.currentTarget.style.background = "var(--paper)"}> Join Waitlist</button>
              </>
          }

        </div>
        <button className={`mob-menu-btn${menuOpen ? " open" : ""}`} onClick={() => setMenuOpen(o => !o)} aria-label="Menu">
          <span /><span /><span />
        </button>
      </div>

      {/* ── MOBILE DRAWER ── */}
      <div className={`mob-drawer${menuOpen ? " open" : ""}`}>
        {NAV_LINKS.map(l => (
          <button key={l.label} className="mob-link" onClick={() => scrollTo(l.target)}>{l.label}</button>
        ))}
        <div className="mob-drawer-ctas">
          {isLoggedIn
            ? <button onClick={() => { setMenuOpen(false); go('demo'); }} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--accent)", padding: "14px 0", border: "none", cursor: "pointer", textAlign: "center" }}>Open Demo</button>
            : <button onClick={() => { setMenuOpen(false); go('demo'); }} style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--accent)", padding: "14px 0", border: "none", cursor: "pointer", textAlign: "center" }}>Try Demo</button>
          }
          {!isLoggedIn && (
            <button onClick={() => { setMenuOpen(false); setWlOpen(true); }}
              style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".1em",
                       textTransform: "uppercase", color: "var(--ink)",
                       background: "var(--paper)", padding: "14px 0",
                       border: "none", cursor: "pointer", textAlign: "center" }}>
              Join Waitlist
            </button>
          )}
        </div>
      </div>

      {/* ── HERO SECTION (untouched) ── */}
      <section id="hero" style={{ height: "100vh", position: "relative", overflow: "hidden", display: "flex", flexDirection: "column", justifyContent: "flex-end", padding: "0 52px 56px", zIndex: 10 }} className="hero-pad">
        <div ref={heroRef} style={{ position: "absolute", inset: 0, pointerEvents: "none" }}>
          <div className="hero-gradient" style={{ position: "absolute", inset: 0, background: "linear-gradient(to top,rgba(10,10,10,.9) 0%,rgba(10,10,10,.25) 45%,transparent 72%)", opacity: 0 }} />
          <div style={{ position: "absolute", bottom: 56, left: 52, right: 52 }} className="hero-pad-inner">
            <div className="hero-label-wrap" style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)", display: "flex", alignItems: "center", gap: 14, marginBottom: 20, overflow: "hidden", opacity: 0 }}>
              <div style={{ width: 36, height: 1, background: "var(--accent)", flexShrink: 0 }} />
              <div className="hero-label-inner" style={{ transform: "translateY(100%)" }}>Zero-trust · Device identity · No passwords</div>
            </div>
            <div style={{ overflow: "hidden" }}>
              <div className="hl1" style={{ fontFamily: "var(--display)", fontSize: "clamp(58px,13.5vw,195px)", lineHeight: .9, letterSpacing: ".025em", textTransform: "uppercase", transform: "translateY(110%)", display: "block" }}>Identity</div>
            </div>
            <div style={{ overflow: "hidden" }}>
              <div className="hl2" style={{ fontFamily: "var(--serif)", fontStyle: "italic", fontSize: "clamp(52px,12.5vw,180px)", lineHeight: .9, transform: "translateY(110%)", display: "block" }}>Redefined.</div>
            </div>
            <div className="hero-meta" style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", marginTop: 36, paddingTop: 24, borderTop: "1px solid var(--line)", opacity: 0, flexWrap: "wrap", gap: 16 }}>
              <p style={{ fontSize: 14, color: "var(--muted)", maxWidth: 300, lineHeight: 1.75, fontWeight: 300 }}>Authentication powered by hardware cryptography. Every request verified. Every device accountable.</p>
              <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--muted)", display: "flex", alignItems: "center", gap: 12, cursor: "pointer" }} onClick={() => scrollTo("about")}>
                <div style={{ width: 1, height: 44, background: "rgba(122,117,112,.3)", position: "relative", overflow: "hidden" }}>
                  <div style={{ position: "absolute", top: 0, left: 0, width: "100%", height: "50%", background: "var(--accent)", animation: "sp 2s ease-in-out infinite" }} />
                </div>
                Scroll to explore
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* ── TICKER ── */}
      <div style={{ background: "var(--accent)", overflow: "hidden", padding: "13px 0", whiteSpace: "nowrap", position: "relative", zIndex: 10 }} aria-hidden="true">
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

      {/* ── CONTENT SECTIONS ── */}
      <div style={{ position: "relative", zIndex: 10, background: "var(--ink)" }}>

        {/* ══ 01 — MANIFESTO / FEATURES ══ */}
        <section id="features" style={{ padding: "140px 52px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16 }}>
            <span>01 — Features</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="rv manifesto-grid" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 80, alignItems: "start" }}>
            {/* Left — headline */}
            <div>
              <div style={{ fontFamily: "var(--serif)", fontSize: "clamp(40px,4.5vw,66px)", lineHeight: 1.1, letterSpacing: "-.01em", marginBottom: 48 }}>
                Passwords were a<br /><em style={{ fontStyle: "italic", color: "var(--muted)" }}>compromise.</em><br />We built the alternative.
              </div>
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
                <BtnF onClick={() => go('demo')}>Try Demo →</BtnF>
                <BtnO onClick={() => setWlOpen(true)}>Join Waitlist →</BtnO>
              </div>
            </div>
            {/* Right — bullet features */}
            <div style={{ display: "flex", flexDirection: "column", gap: 0, borderTop: "1px solid var(--line)" }}>
              {[
                { i: "F.01", t: "No Passwords", b: "Eliminate the entire password attack surface. No phishing, no credential stuffing, no breached database exposure." },
                { i: "F.02", t: "Hardware Keys", b: "Private keys generated and stored in device secure enclaves. Extraction is physically impossible by design." },
                { i: "F.03", t: "Zero Trust", b: "Every single request independently verified. No implicit trust. Deny by default, verify by cryptographic proof." },
              ].map((f, idx) => (
                <FeatureBullet key={f.i} f={f} />
              ))}
            </div>
          </div>
        </section>

        {/* ══ 02 — PROTOCOL (untouched) ══ */}
        <section id="protocol" style={{ padding: "0 52px 140px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16 }}>
            <span>02 — Protocol</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <h2 className="rv" style={{ fontFamily: "var(--display)", fontSize: "clamp(52px,8vw,112px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .93, marginBottom: 6 }}>Three-step<br />verification</h2>
          <p className="rv rv-1" style={{ fontSize: 14, color: "var(--muted)", maxWidth: 420, fontWeight: 300, lineHeight: 1.75, marginBottom: 0 }}>A deterministic challenge-response protocol. No shared secrets. No replay attacks. Mathematically sound.</p>
          <div className="rv rv-1 hiw-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 0, background: "var(--line)", marginTop: 72, border: "1px solid var(--line)" }}>
            {[
              { n: "01", t: "Challenge", b: "Server issues a time-bound nonce. Non-repeatable. Cannot be predicted." },
              { n: "02", t: "Sign", b: "Device hardware signs the nonce. Private key never leaves the chip." },
              { n: "03", t: "Verify", b: "Server verifies the signature against your public key. Sub-200ms." },
            ].map(c => <HiWCard key={c.n} {...c} />)}
          </div>
          {/* Live animation — merged into protocol */}
          <div className="rv" style={{ marginTop: 56, paddingTop: 40, borderTop: "1px solid var(--line)" }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 24, display: "flex", alignItems: "center", gap: 14 }}>
              <span style={{ color: "var(--accent)" }}>// live</span> — click any node to simulate a failure
              <div style={{ flex: 1, height: 1, background: "var(--line)" }} />
            </div>
            <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(48px,6.5vw,88px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .93, marginBottom: 48 }}>
              Every request.<br /><span style={{ color: "var(--accent)" }}>Cryptographically proven.</span>
            </h2>
            <div className="trust-anim-wrap">
              <TrustAnimation />
            </div>
            <div className="trust-mobile" style={{ display: "none", flexDirection: "column", gap: 0, border: "1px solid var(--line)" }}>
              {[
                { n: "01", t: "Challenge", d: "Server issues a time-bound nonce." },
                { n: "02", t: "Sign",      d: "Device hardware signs the nonce." },
                { n: "03", t: "Verify",    d: "Server verifies the signature." },
                { n: "04", t: "Granted",   d: "Access confirmed. Sub-200ms." },
              ].map((s, i) => (
                <div key={s.n} style={{ display: "flex", alignItems: "center", gap: 16, padding: "18px 20px", borderBottom: i < 3 ? "1px solid var(--line)" : "none", background: "var(--ink-2)" }}>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", width: 24, flexShrink: 0 }}>{s.n}</div>
                  <div>
                    <div style={{ fontFamily: "var(--display)", fontSize: 18, letterSpacing: ".06em", textTransform: "uppercase" }}>{s.t}</div>
                    <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", marginTop: 2 }}>{s.d}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ══ WHO IT'S FOR ══ */}
        <section id="who" style={{ padding: "40px 52px 72px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16 }}>
            <span>03 — Who It's For</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="rv who-grid" style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 0, border: "1px solid var(--line)" }}>
            {[
              { tag: "FOR DEVELOPERS", icon: "</>", title: "Developers", line: "Ship zero-trust auth in a single SDK call.", chips: ["SaaS","Dev Tools","Open Source","API-first"] },
              { tag: "FOR TEAMS",      icon: "⬡⬡",  title: "Teams",      line: "Replace passwords across your org without friction.", chips: ["Fintech","HealthTech","Legal","Remote-first"] },
              { tag: "FOR ENTERPRISE", icon: "▣",   title: "Enterprises", line: "Hardware-attested identity with compliance built in.", chips: ["Banking","Defence","Gov","Critical Infra"] },
            ].map((c, idx) => <WhoCard key={c.tag} card={c} idx={idx} scrollTo={scrollTo} />)}
          </div>
        </section>


        {/* ══ ATTACK SURFACE ══ */}
        <section id="attacks" style={{ padding: "0 52px 80px", borderTop: "1px solid var(--line)" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16, paddingTop: 100 }}>
            <span>05 — Attack Surface</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
            <AttackSimTrigger openSim={openSim} />
          </div>
          <h2 className="rv" style={{ fontFamily: "var(--display)", fontSize: "clamp(52px,7.5vw,104px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .93, marginBottom: 16 }}>
            Every attack.<br /><span style={{ color: "var(--accent)" }}>Already blocked.</span>
          </h2>
          <p className="rv" style={{ fontSize: 14, color: "var(--muted)", fontWeight: 300, lineHeight: 1.75, maxWidth: 480, marginBottom: 72 }}>
            Hover any attack type to see how Crypton stops it. Or try it yourself.
          </p>
          <AttackTerminal />
        </section>

        {/* ══ BEFORE VS AFTER ══ */}
        <section style={{ padding: "0 52px 80px", borderTop: "1px solid var(--line)" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16, paddingTop: 100 }}>
            <span>06 — Before vs After</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="before-after-table">
            <BeforeAfter />
          </div>
        </section>

        {/* ══ PRICING ══ */}
        <section id="pricing" style={{ padding: "0 52px 100px", borderTop: "1px solid var(--line)" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16, paddingTop: 140 }}>
            <span>07 — Pricing</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="rv pricing-grid" style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 1, background: "var(--line)", border: "1px solid var(--line)" }}>
            {[
              { badge: "Free",       sub: "Developer / Startup",    price: "$0",     note: "",        features: ["Up to 5 users / 10 devices","Passkey-based login","Basic device enrollment","Simple admin dashboard","Manual device approval"], cta: "Start Free" },
              { badge: "Starter",    sub: "Small Teams",            price: "$5",     note: "per user / mo",  features: ["Up to 50 users","Multiple devices per user","Device trust policies","Admin controls + logs","Email support"], cta: "Get Started" },
              { badge: "Growth",     sub: "Scaling Companies",      price: "$10",    note: "per user / mo",  features: ["Unlimited users & devices","Advanced trust policies","Risk-based access decisions","API access & integrations","Audit logs + compliance"], cta: "Start Growing" },
              { badge: "Enterprise", sub: "Security-First Orgs",    price: "Custom", note: "contact us",     features: ["Custom deployment","SOC2 / HIPAA alignment","Dedicated support","On-prem / hybrid options","SLA guarantees"], cta: "Get in Touch" },
            ].map((p, i) => <PricingCard2 key={p.badge} plan={p} go={go} toast={toast} />)}
          </div>
        </section>

        {/* ══ 04 — ABOUT ══ */}
        <section id="about" style={{ padding: "0 52px 140px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16 }}>
            <span>07 — About</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          <div className="rv about-grid" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 80, alignItems: "start" }}>
            {/* Left — what Crypton is */}
            <div>
              <h2 style={{ fontFamily: "var(--serif)", fontSize: "clamp(32px,3.5vw,52px)", lineHeight: 1.12, letterSpacing: "-.01em", marginBottom: 36 }}>
                A new foundation<br />for <em style={{ fontStyle: "italic", color: "var(--muted)" }}>device identity.</em>
              </h2>
              <div style={{ width: 40, height: 1, background: "var(--accent)", marginBottom: 36 }} />
              <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.9, fontWeight: 300, marginBottom: 24 }}>
                Crypton is a zero-trust identity platform built on hardware cryptography. We replace passwords, OTPs, and shared secrets with device-bound keys that never leave your hardware.
              </p>
              <p style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.9, fontWeight: 300, marginBottom: 36 }}>
                Every authentication is a cryptographic proof. Every device is independently verified. Every session is visible and revocable in real time.
              </p>

            </div>
            {/* Right — product highlights */}
            <div style={{ display: "flex", flexDirection: "column", gap: 1, background: "var(--line)" }}>
              {[
                { label: "Authentication",  value: "Hardware-bound passkeys. No passwords. No OTPs." },
                { label: "Device Control",  value: "Enroll, monitor, and revoke devices in under 500ms." },
                { label: "Session Map",     value: "See every active session across every device, live." },
                { label: "Trust Policies",  value: "Define rules for which devices can access what, when." },
                { label: "Audit Trail",     value: "Cryptographically signed logs of every auth event." },
                { label: "Recovery",        value: "24-hour time-lock. Trusted devices cancel bad actors." },
              ].map(item => (
                <div key={item.label} style={{ background: "var(--ink)", padding: "24px 28px", display: "flex", flexDirection: "column", gap: 6 }}>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--accent)" }}>{item.label}</div>
                  <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.6, fontWeight: 300 }}>{item.value}</div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ══ 05 — VISION ══ */}
        <section id="vision" style={{ padding: "0 52px 140px" }} className="section-pad">
          <div className="rv" style={{ fontFamily: "var(--mono)", fontSize: 13, letterSpacing: ".16em", color: "rgba(200,245,90,0.7)", marginBottom: 56, display: "flex", alignItems: "center", gap: 16 }}>
            <span>08 — Vision</span><div style={{ flex: 1, height: 1, background: "var(--line)" }} />
          </div>
          {/* Big headline */}
          <div className="rv" style={{ marginBottom: 96 }}>
            <h2 style={{ fontFamily: "var(--display)", fontSize: "clamp(52px,8vw,110px)", textTransform: "uppercase", letterSpacing: ".04em", lineHeight: .92, maxWidth: 900 }}>
              Trust as a<br /><span style={{ color: "var(--accent)" }}>built-in</span><br />property.
            </h2>
          </div>
          {/* Two column layout — pull quote left, body right */}
          <div className="rv" style={{ display: "grid", gridTemplateColumns: "5fr 7fr", gap: 80, alignItems: "start", paddingTop: 56, borderTop: "1px solid var(--line)" }}>
            <div>
              <p style={{ fontFamily: "var(--serif)", fontSize: "clamp(20px,2.2vw,30px)", lineHeight: 1.45, color: "var(--paper)", letterSpacing: "-.01em" }}>
                "Not something layered on after the fact — but a fundamental property of how systems interact."
              </p>
              <div style={{ width: 40, height: 1, background: "var(--accent)", marginTop: 36 }} />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
              <p style={{ fontSize: 15, color: "var(--muted)", lineHeight: 1.9, fontWeight: 300 }}>
                Software has evolved faster than the systems we use to trust it. We're focused on closing that gap — moving beyond reactive security toward continuous trust across people, devices, and environments, without added friction.
              </p>
              <p style={{ fontSize: 15, color: "var(--paper)", lineHeight: 1.9, fontWeight: 300 }}>
                We're building toward a future where trust becomes a built-in property of how systems interact, not something layered on after the fact.
              </p>
            </div>
          </div>
        </section>

        {/* ══ FOOTER ══ */}
        <footer style={{ borderTop: "1px solid var(--line)", padding: "72px 52px 36px" }} className="section-pad">
          <div className="rv footer-grid" style={{ display: "grid", gridTemplateColumns: "3fr 1fr 1fr 1fr", gap: 48, paddingBottom: 56, borderBottom: "1px solid var(--line)" }}>
            <div className="footer-brand">
              <div style={{ fontFamily: "var(--display)", fontSize: 56, letterSpacing: ".08em", lineHeight: 1, marginBottom: 20 }}>CRYPTON</div>
              <p style={{ fontSize: 12, color: "var(--muted)", fontWeight: 300, lineHeight: 1.7, maxWidth: 250 }}>Zero-trust device identity. Authentication powered by cryptography — not passwords, not hope.</p>
            </div>
            {[
              { h: "Product", links: [
                { l: "Features",     action: () => scrollTo("features") },
                { l: "Protocol",     action: () => scrollTo("protocol") },
                { l: "Who It's For", action: () => scrollTo("who") },
                { l: "Attack Surface",action: () => scrollTo("attacks") },
                { l: "Pricing",      action: () => scrollTo("pricing") },
              ]},
              { h: "Developer", links: [
                { l: "Documentation", action: () => go("admin") },
                { l: "API Reference",  action: () => go("admin") },
                { l: "Dashboard",      action: () => go("dashboard") },
                { l: "GitHub",         action: () => window.open("https://github.com/Aryanvirpsu/Crypton-DI", "_blank") },
              ]},
              { h: "Company", links: [
                { l: "About",   action: () => scrollTo("about") },
                { l: "Vision",  action: () => scrollTo("vision") },
                { l: "Security",action: () => go("risk") },
                { l: "Privacy", action: () => toast("Privacy policy — coming soon", "info") },
                { l: "Terms",   action: () => toast("Terms of service — coming soon", "info") },
              ]},
            ].map(col => (
              <div key={col.h}>
                <h5 style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 18 }}>{col.h}</h5>
                <ul style={{ listStyle: "none", display: "flex", flexDirection: "column", gap: 11 }}>
                  {col.links.map(({ l, action }) => (
                    <li key={l}><button onClick={action} style={{ fontSize: 13, color: "var(--paper)", opacity: .55, background: "none", border: "none", cursor: "pointer", padding: 0, transition: "opacity .2s", fontFamily: "var(--body)" }}
                      onMouseEnter={e => e.target.style.opacity = 1} onMouseLeave={e => e.target.style.opacity = .55}>{l}</button></li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: 28, fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", color: "var(--muted)", flexWrap: "wrap", gap: 8 }}>
            <span>© 2026 CRYPTON — ALL RIGHTS RESERVED</span><span>V1.0 · MARCH 2026</span>
          </div>
        </footer>

      {/* ── WAITLIST MODAL ── */}
      {wlOpen && (
        <div onClick={e => { if (e.target === e.currentTarget) { setWlOpen(false); setWlDone(false); setWlEmail(''); } }}
          style={{ position:"fixed", inset:0, background:"rgba(10,10,10,0.60)", backdropFilter:"blur(14px) saturate(0.7)", WebkitBackdropFilter:"blur(14px) saturate(0.7)", zIndex:9999, display:"flex", alignItems:"center", justifyContent:"center" }}>
          <div style={{ background:"var(--ink)", border:"1px solid var(--line)", padding:"44px 52px", maxWidth:420, width:"90%", position:"relative" }}>
            <button onClick={() => { setWlOpen(false); setWlDone(false); setWlEmail(''); }}
              style={{ position:"absolute", top:16, right:18, background:"none", border:"none", color:"var(--muted)", fontSize:18, cursor:"pointer", lineHeight:1 }}>×</button>
            {wlDone
              ? <div>
                  <div style={{ fontFamily:"var(--mono)", fontSize:11, letterSpacing:".14em", textTransform:"uppercase", color:"var(--accent)", marginBottom:10 }}>You're on the list ✓</div>
                  <div style={{ fontSize:13, color:"var(--muted)", lineHeight:1.75 }}>We'll reach out when early access opens. No spam — ever.</div>
                </div>
              : <>
                  <div style={{ fontFamily:"var(--display)", fontSize:22, letterSpacing:".05em", textTransform:"uppercase", marginBottom:8 }}>Join the Waitlist</div>
                  <div style={{ fontFamily:"var(--mono)", fontSize:10, color:"var(--muted)", letterSpacing:".06em", marginBottom:24 }}>Be first to know when early access opens.</div>
                  <div style={{ display:"flex", gap:8 }}>
                    <input value={wlEmail} onChange={e => setWlEmail(e.target.value)}
                      onKeyDown={e => { if (e.key === 'Enter') handleWaitlist(); }}
                      placeholder="you@company.com"
                      style={{ flex:1, background:"rgba(255,255,255,.04)", border:"1px solid var(--line2)", color:"var(--paper)", padding:"11px 14px", fontFamily:"var(--mono)", fontSize:11, outline:"none" }} />
                    <button onClick={handleWaitlist}
                      style={{ fontFamily:"var(--mono)", fontSize:9, letterSpacing:".1em", textTransform:"uppercase", background:"var(--paper)", color:"var(--ink)", border:"none", padding:"11px 18px", cursor:"pointer", whiteSpace:"nowrap" }}>
                      Notify me
                    </button>
                  </div>
                </>
            }
          </div>
        </div>
      )}

      </div>
    </div>
  );
}

/* ── Trust Animation ───────────────────────────────────────────── */
function TrustAnimation() {
  const canvasRef = useRef(null);
  const rafRef    = useRef(null);
  const roRef     = useRef(null);
  const stRef     = useRef({ packets:[], nextSpawn:55, t:0, brokenAt:-1,
                             W:0, H:0, nodeXs:[], ny:0, ready:false, hoveredNode:-1 });
  const [broken,      setBroken]      = useState(-1);
  const [hoveredNode, setHoveredNode] = useState(-1);
  const [dims,        setDims]        = useState({ W:0, nodeXs:[], ny:0 });

  const NODES = [
    { label:'DEVICE ENCLAVE', sub:'TPM / Secure Element',
      tip:'Private key generated on-device. Never exported — not even to Crypton.',
      broke:'Without Crypton: password stored in a database. One breach exposes every user.' },
    { label:'SIGNED NONCE',   sub:'Cryptographic signature',
      tip:'Device signs a one-time challenge. Replay attacks are impossible.',
      broke:'Without Crypton: plain credentials sent over the wire. Interceptable. Replayable.' },
    { label:'CRYPTON SERVER', sub:'Signature verification',
      tip:'Server checks the signature against your public key. No secret stored server-side.',
      broke:'Without Crypton: server holds password hashes. Leaked DB = instant compromise for all users.' },
    { label:'ACCESS GRANTED', sub:'Sub-200ms, zero secrets',
      tip:'Access granted by cryptographic proof alone. No password, no OTP, no shared secret.',
      broke:'Without Crypton: session token issued — stealable, reusable, and impossible to audit.' },
  ];
  const N = NODES.length, NW = 185, NH = 72;

  const layout = (cssW, cssH) => {
    const span = cssW * 0.72, sx = (cssW-span)/2, sp = span/(N-1);
    return { nodeXs: NODES.map((_,i)=>sx+i*sp), ny: cssH*0.50 };
  };

  useEffect(()=>{
    const canvas = canvasRef.current; if (!canvas) return;
    const startLoop = () => {
      const ctx = canvas.getContext('2d');
      const s   = stRef.current;
      const setSize = ()=>{
        const dpr=window.devicePixelRatio||1, cssW=canvas.offsetWidth, cssH=canvas.offsetHeight;
        if (!cssW||!cssH) return false;
        canvas.width=cssW*dpr; canvas.height=cssH*dpr; ctx.scale(dpr,dpr);
        const {nodeXs,ny}=layout(cssW,cssH);
        s.W=cssW; s.H=cssH; s.nodeXs=nodeXs; s.ny=ny; s.ready=true;
        setDims({W:cssW,nodeXs,ny}); return true;
      };
      if (!setSize()) return;

      const spawn=()=>{
        if (s.brokenAt>=0) return;
        s.packets.push({pos:0,speed:0.003+Math.random()*0.002,reject:Math.random()<0.18,alpha:1,done:false,trail:[]});
      };

      const draw=()=>{
        if (!s.ready){rafRef.current=requestAnimationFrame(draw);return;}
        const {W,H,nodeXs,ny}=s;
        const dpr=window.devicePixelRatio||1;
        ctx.setTransform(dpr,0,0,dpr,0,0);
        ctx.clearRect(0,0,W,H);
        const isBroken=s.brokenAt>=0;

        for (let i=0;i<N-1;i++){
          const x1=nodeXs[i],x2=nodeXs[i+1];
          const dead=isBroken&&i>=s.brokenAt, hov=s.hoveredNode===i||s.hoveredNode===i+1;
          ctx.save();
          ctx.strokeStyle=dead?'rgba(248,113,113,0.25)':hov?'rgba(200,245,90,0.32)':'rgba(200,245,90,0.09)';
          ctx.lineWidth=1; ctx.setLineDash([5,8]);
          ctx.beginPath(); ctx.moveTo(x1,ny); ctx.lineTo(x2,ny); ctx.stroke();
          ctx.setLineDash([]);
          const ax=(x1+x2)/2;
          ctx.strokeStyle=dead?'rgba(248,113,113,0.38)':'rgba(200,245,90,0.3)';
          ctx.lineWidth=1.3;
          ctx.beginPath(); ctx.moveTo(ax-7,ny-5); ctx.lineTo(ax+1,ny); ctx.lineTo(ax-7,ny+5); ctx.stroke();
          ctx.restore();
        }

        nodeXs.forEach((x,i)=>{
          const hov=s.hoveredNode===i, dead=isBroken&&i>=s.brokenAt, isLast=i===N-1;
          if (hov||isLast||dead){
            const rgb=dead?'248,113,113':'200,245,90';
            const alpha=dead?0.13:isLast?0.06:0.09;
            const g=ctx.createRadialGradient(x,ny,0,x,ny,NW*0.85);
            g.addColorStop(0,`rgba(${rgb},${alpha})`); g.addColorStop(1,'transparent');
            ctx.fillStyle=g; ctx.fillRect(x-NW,ny-NH,NW*2,NH*2);
          }
        });

        if (!isBroken){
          s.packets.forEach(pkt=>{
            if (pkt.done) return;
            const si=Math.min(Math.floor(pkt.pos*(N-1)),N-2);
            const st=pkt.pos*(N-1)-si;
            const px=nodeXs[si]+(nodeXs[si+1]-nodeXs[si])*st, py=ny;
            const rgb=pkt.reject?'248,113,113':'200,245,90';
            pkt.trail.push({x:px,y:py});
            if (pkt.trail.length>20) pkt.trail.shift();
            pkt.trail.forEach((pt,ti)=>{
              const ta=(ti/pkt.trail.length)*0.38*pkt.alpha;
              ctx.save(); ctx.beginPath(); ctx.arc(pt.x,pt.y,1.4+ti*0.1,0,Math.PI*2);
              ctx.fillStyle=`rgba(${rgb},${ta})`; ctx.fill(); ctx.restore();
            });
            ctx.save(); ctx.beginPath(); ctx.arc(px,py,5.5,0,Math.PI*2);
            ctx.fillStyle=`rgba(${rgb},${pkt.alpha})`;
            ctx.shadowBlur=20; ctx.shadowColor=pkt.reject?'#F87171':'#C8F55A';
            ctx.fill(); ctx.restore();
            pkt.pos+=pkt.speed;
            if (pkt.pos>=1){if(pkt.reject){pkt.alpha-=0.055;if(pkt.alpha<=0)pkt.done=true;}else pkt.done=true;}
          });
          s.packets=s.packets.filter(p=>!p.done);
          s.nextSpawn--;
          if (s.nextSpawn<=0){spawn();s.nextSpawn=60+Math.floor(Math.random()*50);}
        } else {
          const pulse=0.05+0.04*Math.sin(s.t*0.07);
          nodeXs.forEach((x,i)=>{
            if (i<s.brokenAt) return;
            const pg=ctx.createRadialGradient(x,ny,0,x,ny,NW*0.8);
            pg.addColorStop(0,`rgba(248,113,113,${pulse})`); pg.addColorStop(1,'transparent');
            ctx.fillStyle=pg; ctx.fillRect(x-NW,ny-NH,NW*2,NH*2);
          });
        }
        s.t++; rafRef.current=requestAnimationFrame(draw);
      };

      roRef.current=new ResizeObserver(()=>{
        const dpr=window.devicePixelRatio||1,cssW=canvas.offsetWidth,cssH=canvas.offsetHeight;
        if (!cssW||!cssH) return;
        canvas.width=cssW*dpr; canvas.height=cssH*dpr;
        const {nodeXs,ny}=layout(cssW,cssH);
        const s2=stRef.current; s2.W=cssW;s2.H=cssH;s2.nodeXs=nodeXs;s2.ny=ny;s2.ready=true;
        setDims({W:cssW,nodeXs,ny});
      });
      roRef.current.observe(canvas);
      spawn(); rafRef.current=requestAnimationFrame(draw);
    };
    const wait=()=>{if(canvas.offsetWidth>0)startLoop();else requestAnimationFrame(wait);};
    wait();
    return ()=>{cancelAnimationFrame(rafRef.current);roRef.current?.disconnect();};
  },[]);

  useEffect(()=>{stRef.current.brokenAt=broken;},[broken]);
  useEffect(()=>{stRef.current.hoveredNode=hoveredNode;},[hoveredNode]);

  const handleMouseMove=e=>{
    const canvas=canvasRef.current; if(!canvas) return;
    const rect=canvas.getBoundingClientRect(), mx=e.clientX-rect.left;
    const {nodeXs}=stRef.current;
    let found=-1;
    nodeXs.forEach((x,i)=>{if(mx>=x-NW/2&&mx<=x+NW/2)found=i;});
    setHoveredNode(found);
  };
  const handleClick=e=>{
    const canvas=canvasRef.current; if(!canvas) return;
    const rect=canvas.getBoundingClientRect(), mx=e.clientX-rect.left;
    const {nodeXs}=stRef.current;
    let found=-1;
    nodeXs.forEach((x,i)=>{if(mx>=x-NW/2&&mx<=x+NW/2)found=i;});
    if (found>=0){setBroken(b=>b===found?-1:found);stRef.current.packets=[];}
    else setBroken(-1);
  };

  const activeInfo=broken>=0?NODES[broken]:hoveredNode>=0?NODES[hoveredNode]:null;
  const isBrokenInfo=broken>=0;

  return (
    <div style={{position:'relative'}}>
      <div style={{position:'relative',width:'100%'}}
        onMouseMove={handleMouseMove} onMouseLeave={()=>setHoveredNode(-1)} onClick={handleClick}>
        <canvas ref={canvasRef} style={{width:'100%',height:240,display:'block',cursor:'pointer'}}/>
        {dims.W>0&&dims.nodeXs.map((x,i)=>{
          const isLast=i===N-1,isHov=hoveredNode===i;
          const isDead=broken>=0&&i>=broken;
          return (
            <div key={i} style={{
              position:'absolute',left:x-NW/2,top:dims.ny-NH/2,width:NW,height:NH,
              border:`1px solid ${isDead?'rgba(248,113,113,0.55)':isHov?'rgba(200,245,90,0.5)':isLast?'rgba(200,245,90,0.35)':'rgba(244,241,236,0.1)'}`,
              background:isDead?'rgba(248,113,113,0.06)':isHov?'rgba(200,245,90,0.06)':isLast?'rgba(200,245,90,0.04)':'rgba(244,241,236,0.02)',
              borderRadius:6,display:'flex',flexDirection:'column',alignItems:'center',justifyContent:'center',
              transition:'border-color .2s,background .2s',pointerEvents:'none',userSelect:'none',
            }}>
              <div style={{fontFamily:"'DM Mono','Courier New',monospace",fontSize:11,fontWeight:700,
                letterSpacing:'.1em',color:isDead?'#F87171':isLast?'#C8F55A':'rgba(244,241,236,0.9)',
                lineHeight:1,marginBottom:6,transition:'color .2s'}}>{NODES[i].label}</div>
              <div style={{fontFamily:"'DM Mono','Courier New',monospace",fontSize:9,
                color:'rgba(122,117,112,0.8)',letterSpacing:'.06em',lineHeight:1}}>{NODES[i].sub}</div>
            </div>
          );
        })}
      </div>
      <div style={{minHeight:60,marginTop:14,display:'flex',alignItems:'center',justifyContent:'center'}}>
        {activeInfo?(
          <div style={{display:'flex',alignItems:'flex-start',gap:14,padding:'14px 22px',
            border:`1px solid ${isBrokenInfo?'rgba(251,191,36,0.3)':'rgba(200,245,90,0.18)'}`,
            background:isBrokenInfo?'rgba(251,191,36,0.05)':'rgba(200,245,90,0.04)',
            maxWidth:520}}>
            <div style={{fontFamily:"'DM Mono',monospace",fontSize:9,
              color:isBrokenInfo?'var(--warning)':'var(--accent)',letterSpacing:'.12em',marginTop:3,flexShrink:0,fontSize:8}}>
              {isBrokenInfo?'WITHOUT CRYPTON':'INFO'}
            </div>
            <div>
              <div style={{fontFamily:"'DM Mono',monospace",fontSize:10,
                color:isBrokenInfo?'var(--danger)':'var(--accent)',letterSpacing:'.12em',textTransform:'uppercase',marginBottom:5}}>
                {activeInfo.label}
              </div>
              <div style={{fontSize:13,color:'var(--muted)',lineHeight:1.7,fontWeight:300}}>
                {isBrokenInfo?activeInfo.broke:activeInfo.tip}
              </div>
            </div>
          </div>
        ):(
          <div style={{fontFamily:"'DM Mono',monospace",fontSize:9,color:'rgba(122,117,112,0.32)',letterSpacing:'.14em'}}>
            // hover to inspect · click to simulate failure
          </div>
        )}
      </div>
      {broken>=0&&(
        <div style={{display:'flex',justifyContent:'center',marginTop:8}}>
          <button onClick={()=>{setBroken(-1);stRef.current.packets=[];}}
            style={{fontFamily:"'DM Mono',monospace",fontSize:9,letterSpacing:'.14em',textTransform:'uppercase',
              color:'var(--accent)',background:'rgba(200,245,90,0.06)',border:'1px solid rgba(200,245,90,0.22)',
              padding:'7px 16px',cursor:'pointer',transition:'all .2s'}}>
            ↺ RESET FLOW
          </button>
        </div>
      )}
    </div>
  );
}


function WhoCard({ card, idx, scrollTo }) {
  const [hov, setHov] = useState(false);
  return (
    <div
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      onClick={() => scrollTo("pricing")}
      style={{
        padding: "52px 44px 48px",
        borderRight: idx < 2 ? "1px solid var(--line)" : "none",
        background: hov ? "var(--ink-2)" : "var(--ink)",
        cursor: "pointer",
        transition: "background .3s",
        position: "relative", overflow: "hidden",
      }}
    >
      {/* accent top bar on hover */}
      <div style={{ position: "absolute", top: 0, left: 0, right: 0, height: 2, background: "var(--accent)", transform: hov ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .45s cubic-bezier(.16,1,.3,1)" }} />
      {/* tag */}
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".18em", textTransform: "uppercase", color: "var(--accent)", marginBottom: 28 }}>{card.tag}</div>
      {/* icon */}
      <div style={{ fontFamily: "var(--display)", fontSize: 52, lineHeight: 1, color: hov ? "rgba(200,245,90,.22)" : "rgba(244,241,236,.07)", marginBottom: 28, transition: "color .3s" }}>{card.icon}</div>
      {/* title */}
      <div style={{ fontFamily: "var(--display)", fontSize: "clamp(28px,3vw,40px)", letterSpacing: ".05em", textTransform: "uppercase", lineHeight: 1, marginBottom: 16 }}>{card.title}</div>
      {/* one-liner */}
      <div style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300, marginBottom: 28 }}>{card.line}</div>
      {/* chips */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        {card.chips.map(ch => (
          <span key={ch} style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".08em", padding: "4px 10px", border: "1px solid var(--line)", color: "var(--muted)" }}>{ch}</span>
        ))}
      </div>
    </div>
  );
}

/* ── Architecture Diagram ───────────────────────────────────────── */
/* ── Architecture Diagram ───────────────────────────────────────── */
function ArchDiagram() {
  const [activeStep, setActiveStep] = useState(-1);
  const steps = [
    { n: "01", label: "Device Enclave",  sub: "TPM / Secure Element", detail: "Private key is generated on-device during enrollment. It is bound to the hardware and never exported — not even to Crypton.", color: "rgba(200,245,90," },
    { n: "02", label: "SDK Challenge",   sub: "Client library",        detail: "The Crypton SDK requests a one-time nonce from the server and passes it to the device enclave for signing.",               color: "rgba(200,245,90," },
    { n: "03", label: "Nonce + Sign",    sub: "Time-bound, single use", detail: "The enclave signs the nonce using the device private key. The signature is returned to the SDK. The key never moves.",        color: "rgba(200,245,90," },
    { n: "04", label: "Verification",    sub: "Crypton Server",         detail: "The server verifies the signature against the registered public key. No password. No secret. Just math.",                   color: "rgba(200,245,90," },
    { n: "05", label: "Access Granted",  sub: "Cryptographic proof",    detail: "Access is granted. The session is created, signed, and logged. Revocation is instant at any point.",                        color: "rgba(200,245,90," },
  ];
  return (
    <div className="rv">
      <div style={{ display: "grid", gridTemplateColumns: "repeat(5,1fr)", gap: 0, border: "1px solid var(--line)" }}>
        {steps.map((step, idx) => {
          const isActive = activeStep === idx;
          const isLast   = idx === steps.length - 1;
          return (
            <div
              key={step.n}
              onMouseEnter={() => setActiveStep(idx)}
              onMouseLeave={() => setActiveStep(-1)}
              style={{
                borderRight: idx < 4 ? "1px solid var(--line)" : "none",
                background: isActive ? (isLast ? "rgba(200,245,90,0.08)" : "var(--ink-2)") : isLast ? "rgba(200,245,90,0.03)" : "var(--ink)",
                transition: "background .25s",
                cursor: "default",
                position: "relative",
                overflow: "hidden",
              }}
            >
              {/* Top accent bar on hover */}
              <div style={{ position: "absolute", top: 0, left: 0, right: 0, height: 2, background: isLast ? "var(--accent)" : "var(--accent)", transform: isActive ? "scaleX(1)" : "scaleX(0)", transformOrigin: "left", transition: "transform .35s cubic-bezier(.16,1,.3,1)" }} />
              <div style={{ padding: "44px 28px 40px" }}>
                {/* Number */}
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".18em", marginBottom: 28, opacity: isActive ? 1 : 0.5, transition: "opacity .2s" }}>{step.n}</div>
                {/* Connector arrow (except last) */}
                {idx < 4 && (
                  <div style={{ position: "absolute", right: -7, top: "38%", width: 12, height: 12, borderTop: `1.5px solid ${isActive ? "rgba(200,245,90,0.7)" : "rgba(200,245,90,0.2)"}`, borderRight: `1.5px solid ${isActive ? "rgba(200,245,90,0.7)" : "rgba(200,245,90,0.2)"}`, transform: "rotate(45deg)", transition: "border-color .25s", zIndex: 2 }} />
                )}
                {/* Label */}
                <div style={{ fontFamily: "var(--display)", fontSize: "clamp(15px,1.5vw,20px)", letterSpacing: ".06em", textTransform: "uppercase", lineHeight: 1.1, marginBottom: 10, color: isLast ? "var(--accent)" : "var(--paper)", transition: "color .2s" }}>{step.label}</div>
                {/* Sub */}
                <div style={{ fontFamily: "var(--mono)", fontSize: 9, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--muted)", marginBottom: 20 }}>{step.sub}</div>
                {/* Detail — reveals on hover */}
                <div style={{ fontSize: 12, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, maxHeight: isActive ? "120px" : "0", overflow: "hidden", opacity: isActive ? 1 : 0, transition: "max-height .35s ease, opacity .3s ease" }}>{step.detail}</div>
              </div>
            </div>
          );
        })}
      </div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--muted)", letterSpacing: ".1em", marginTop: 16, textAlign: "right", opacity: 0.5 }}>// hover any step to expand</div>
    </div>
  );
}

/* ── Feature card (F.01 / F.02 / F.03) ─────────────────────── */
function FeatureCard({ f, last }) {
  const [hov, setHov] = useState(false);
  return (
    <div
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        background: hov ? "var(--ink-2)" : "var(--ink)",
        padding: "52px 40px",
        position: "relative",
        overflow: "hidden",
        cursor: "default",
        transition: "background .35s",
        borderRight: last ? "none" : "1px solid var(--line)",
      }}
    >
      <div style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".12em", marginBottom: 28 }}>{f.i}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: "clamp(28px,3vw,40px)", letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 18, lineHeight: 1 }}>{f.t}</div>
      <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300, maxWidth: 280 }}>{f.b}</div>
      <div style={{
        position: "absolute", bottom: 0, left: 0, right: 0, height: 2,
        background: "var(--accent)",
        transform: hov ? "scaleX(1)" : "scaleX(0)",
        transformOrigin: "left",
        transition: "transform .5s cubic-bezier(.16,1,.3,1)",
      }} />
    </div>
  );
}

/* ── Feature bullet (F.01 / F.02 / F.03) ───────────────────── */
function FeatureBullet({ f }) {
  const [hov, setHov] = useState(false);
  return (
    <div
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        display: "flex", alignItems: "flex-start", gap: 20,
        padding: "28px 0", borderBottom: "1px solid var(--line)",
        transition: "all .2s", cursor: "default",
      }}
    >
      <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".12em", marginTop: 4, flexShrink: 0 }}>{f.i}</span>
      <div style={{ flex: 1 }}>
        <div style={{ fontFamily: "var(--display)", fontSize: "clamp(20px,2.2vw,28px)", letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 8, color: hov ? "var(--paper)" : "var(--paper)", transition: "color .2s" }}>{f.t}</div>
        <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300 }}>{f.b}</div>
      </div>
      <div style={{ width: 6, height: 6, borderRadius: "50%", background: hov ? "var(--accent)" : "transparent", border: "1px solid var(--line)", marginTop: 6, flexShrink: 0, transition: "background .25s" }} />
    </div>
  );
}

/* ── PricingCard2 — single row, hover border, expand on hover ── */
function PricingCard2({ plan, go, toast }) {
  const [hov, setHov] = useState(false);
  const isEnt = plan.badge === 'Enterprise';
  return (
    <div
      onMouseEnter={()=>setHov(true)}
      onMouseLeave={()=>setHov(false)}
      style={{
        background: hov ? 'var(--ink-3)' : 'var(--ink-2)',
        borderRight: '1px solid var(--line)',
        outline: hov ? '1px solid var(--accent)' : '1px solid transparent',
        outlineOffset: '-1px',
        padding: '40px 28px',
        position: 'relative',
        transition: 'background .25s, outline-color .25s, box-shadow .25s',
        boxShadow: hov ? '0 0 40px rgba(200,245,90,0.08)' : 'none',
        display: 'flex', flexDirection: 'column',
      }}>
      {/* Accent top bar on hover */}
      <div style={{position:'absolute',top:0,left:0,right:0,height:2,
        background:'var(--accent)',
        transform:hov?'scaleX(1)':'scaleX(0)',
        transformOrigin:'left',
        transition:'transform .35s cubic-bezier(.16,1,.3,1)'}}/>
      {/* Tier name — big and first */}
      <div style={{fontFamily:'var(--display)',fontSize:'clamp(28px,2.8vw,40px)',letterSpacing:'.05em',
        textTransform:'uppercase',lineHeight:1,marginBottom:6}}>{plan.badge}</div>
      <div style={{fontSize:12,color:'var(--muted)',fontWeight:300,marginBottom:20}}>{plan.sub}</div>
      {/* Price — below tier name */}
      <div style={{fontFamily:'var(--display)',fontSize:'clamp(44px,4vw,60px)',letterSpacing:'.02em',
        color:hov?'var(--accent)':'var(--paper)',lineHeight:1,marginBottom:4,transition:'color .25s'}}>{plan.price}</div>
      <div style={{fontFamily:'var(--mono)',fontSize:9,color:'var(--muted)',letterSpacing:'.08em',marginBottom:0}}>{plan.note}</div>
      {/* Divider */}
      <div style={{height:1,background:'var(--line)',margin:'20px 0 0',transition:'margin .35s'}}/>
      {/* Features — expand on hover */}
      <div style={{maxHeight:hov?`${plan.features.length*40}px`:'0',overflow:'hidden',
        transition:'max-height .45s cubic-bezier(.16,1,.3,1)',marginBottom:hov?20:0}}>
        <ul style={{listStyle:'none',paddingTop:16,display:'flex',flexDirection:'column',gap:0}}>
          {plan.features.map((f,fi)=>(
            <li key={f} style={{display:'flex',gap:10,alignItems:'flex-start',fontSize:12,
              color:'var(--muted)',fontWeight:300,paddingBottom:8,
              borderBottom:fi<plan.features.length-1?'1px solid var(--line)':'none',
              marginBottom:fi<plan.features.length-1?8:0}}>
              <span style={{color:'var(--accent)',fontFamily:'var(--mono)',fontSize:9,flexShrink:0,marginTop:2}}>→</span>{f}
            </li>
          ))}
        </ul>
      </div>
      {/* CTA */}
      <div style={{paddingTop:hov?0:16,transition:'padding .35s',marginTop:'auto'}}>
        {isEnt
          ? <BtnO onClick={()=>toast('Enterprise — contact@crypton.dev','info')} style={{fontSize:9,padding:'9px 16px',width:'100%',justifyContent:'center'}}>Get in Touch →</BtnO>
          : <BtnF onClick={()=>go('register')} style={{fontSize:9,padding:'9px 16px',width:'100%',justifyContent:'center'}}>{plan.cta} →</BtnF>
        }
      </div>
    </div>
  );
}


/* ── Vision card ────────────────────────────────────────────── */
function VisionCard({ item, idx }) {
  const [hov, setHov] = useState(false);
  const col = idx % 3;
  return (
    <div
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        background: hov ? "var(--ink-2)" : "var(--ink)",
        padding: "44px 36px",
        position: "relative",
        overflow: "hidden",
        cursor: "default",
        transition: "background .3s",
        borderRight: col < 2 ? "1px solid var(--line)" : "none",
        borderBottom: idx < 3 ? "1px solid var(--line)" : "none",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 24 }}>
        <span style={{ fontFamily: "var(--mono)", fontSize: 9, color: "var(--accent)", letterSpacing: ".12em" }}>{item.tag}</span>
        <span style={{ fontFamily: "var(--mono)", fontSize: 8, letterSpacing: ".1em", textTransform: "uppercase", color: item.accent ? "var(--accent)" : "var(--muted)", padding: "3px 8px", border: `1px solid ${item.accent ? "var(--accent)" : "var(--line)"}` }}>{item.status}</span>
      </div>
      <div style={{ fontFamily: "var(--display)", fontSize: 26, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 14, lineHeight: 1 }}>{item.t}</div>
      <div style={{ fontSize: 13, color: "var(--muted)", lineHeight: 1.7, fontWeight: 300 }}>{item.b}</div>
      <div style={{
        position: "absolute", bottom: 0, left: 0, right: 0, height: 1,
        background: "var(--accent)",
        transform: hov ? "scaleX(1)" : "scaleX(0)",
        transformOrigin: "left",
        transition: "transform .45s cubic-bezier(.16,1,.3,1)",
      }} />
    </div>
  );
}


/* ── AttackTerminal — visual attack cards + terminal trace ──── */
/* ── Attack Simulator — animated fullscreen takeover ──────────── */
function AttackSimTrigger({ openSim }) {
  return (
    <button onClick={openSim} style={{
      fontFamily:'var(--mono)',fontSize:9,letterSpacing:'.14em',textTransform:'uppercase',
      color:'var(--accent)',background:'rgba(200,245,90,0.06)',
      border:'1px solid rgba(200,245,90,0.3)',padding:'8px 18px',
      cursor:'pointer',transition:'all .2s',display:'flex',alignItems:'center',gap:8,flexShrink:0,
    }}
      onMouseEnter={e=>e.currentTarget.style.background='rgba(200,245,90,0.14)'}
      onMouseLeave={e=>e.currentTarget.style.background='rgba(200,245,90,0.06)'}
    >
      <span style={{fontSize:11}}>⌨</span> Try It →
    </button>
  );
}

function AttackTerminal() {
  const [active, setActive] = useState(null);
  const ATTACKS = [
    {
      id:'phishing', label:'Phishing', color:'#F87171', blocked:'14.2M',
      what:'A fake login page tricks the user into entering credentials.',
      how:'Phishing link clicked — no credentials exist to steal. Crypton uses hardware-bound keys, not passwords. The fake form captures nothing.',
    },
    {
      id:'bruteforce', label:'Brute Force', color:'#FBBF24', blocked:'892K',
      what:'Automated tool tries millions of passwords per second.',
      how:'10,000 requests per second hit the auth endpoint — every one rejected. No password surface exists. Crypton requires a device signature, not a guess.',
    },
    {
      id:'mitm', label:'Man in the Middle', color:'#F87171', blocked:'3.1M',
      what:'Attacker sits between client and server, intercepting traffic.',
      how:'Auth token captured in transit — useless. The nonce it signed expired in 500ms and is single-use. Replaying it returns an immediate rejection.',
    },
    {
      id:'stuffing', label:'Credential Stuffing', color:'#FBBF24', blocked:'7.8M',
      what:'Leaked password databases are tried across other services.',
      how:'2.4M leaked credentials loaded and attempted — all fail. Crypton accounts have no passwords. Leaked credentials have zero attack surface here.',
    },
    {
      id:'session', label:'Session Hijack', color:'#F87171', blocked:'521K',
      what:'A stolen session cookie is used to impersonate a logged-in user.',
      how:'Cookie exfiltrated via XSS and presented — rejected. Sessions are cryptographically bound to the originating device. Cookie alone is worthless.',
    },
    {
      id:'replay', label:'Replay Attack', color:'#FBBF24', blocked:'2.3M',
      what:'A valid auth request is captured and resent later.',
      how:'Signed request replayed 1 second later — expired. Each nonce is server-issued, single-use, and has a 500ms window. Replay is mathematically impossible.',
    },
  ];

  return (
    <div className="rv">
      {/* Single container owns hover — panel is inside so mouse doesn't leave */}
      <div
        onMouseLeave={()=>setActive(null)}
        style={{border:'1px solid var(--line)'}}
      >
        {/* 6-card row */}
        <div className="attack-cards" style={{display:'grid',gridTemplateColumns:'repeat(6,1fr)',gap:0}}>
          {ATTACKS.map((atk,i)=>{
            const isActive = active===atk.id;
            return (
              <div key={atk.id}
                onMouseEnter={()=>setActive(atk.id)}
                style={{
                  padding:'28px 20px 24px',
                  borderRight:i<5?'1px solid var(--line)':'none',
                  background:isActive?`rgba(${atk.color==='#F87171'?'248,113,113':'251,191,36'},0.07)`:'var(--ink-2)',
                  cursor:'default', position:'relative', transition:'background .2s',
                }}>
                <div style={{position:'absolute',top:0,left:0,right:0,height:2,
                  background:atk.color,transform:isActive?'scaleX(1)':'scaleX(0)',
                  transformOrigin:'left',transition:'transform .3s cubic-bezier(.16,1,.3,1)'}}/>
                <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',marginBottom:16}}>
                  <div style={{width:7,height:7,borderRadius:'50%',background:atk.color,
                    boxShadow:isActive?`0 0 10px ${atk.color}`:'none',
                    animation:isActive?'atkPulse 1.2s ease-in-out infinite':undefined}}/>
                  <div style={{fontFamily:'var(--mono)',fontSize:8,
                    color:isActive?atk.color:'rgba(122,117,112,0.4)',letterSpacing:'.06em',
                    transition:'color .2s'}}>{atk.blocked}/yr</div>
                </div>
                <div style={{fontFamily:'var(--display)',fontSize:17,letterSpacing:'.05em',
                  textTransform:'uppercase',lineHeight:1.1,marginBottom:8,
                  color:isActive?'var(--paper)':'rgba(244,241,236,0.6)',transition:'color .2s'}}>{atk.label}</div>
                <div style={{fontSize:11,color:'var(--muted)',lineHeight:1.55,fontWeight:300}}>{atk.what}</div>
              </div>
            );
          })}
        </div>

        {/* Explanation panel — inside same container so hover stays active */}
        <div style={{
          borderTop: active ? '1px solid var(--line)' : 'none',
          background:'var(--ink-2)', overflow:'hidden',
          maxHeight:active?'140px':'0',
          transition:'max-height .35s cubic-bezier(.16,1,.3,1), border-color .1s',
        }}>
          {ATTACKS.map(atk=>(
            <div key={atk.id} style={{
              display: active===atk.id ? 'grid' : 'none',
              gridTemplateColumns:'1fr 2fr',gap:0,
            }}>
              <div style={{padding:'24px 28px',borderRight:'1px solid var(--line)',
                display:'flex',flexDirection:'column',justifyContent:'center',
                background:`rgba(${atk.color==='#F87171'?'248,113,113':'251,191,36'},0.05)`}}>
                <div style={{fontFamily:'var(--mono)',fontSize:9,color:atk.color,
                  letterSpacing:'.14em',textTransform:'uppercase',marginBottom:6}}>{atk.label}</div>
                <div style={{fontFamily:'var(--display)',fontSize:24,letterSpacing:'.04em',
                  textTransform:'uppercase',color:'var(--paper)',lineHeight:1,marginBottom:6}}>Blocked.</div>
                <div style={{fontFamily:'var(--mono)',fontSize:9,color:'rgba(122,117,112,0.5)',
                  letterSpacing:'.06em'}}>{atk.blocked} attempts / year</div>
              </div>
              <div style={{padding:'24px 32px',display:'flex',alignItems:'center'}}>
                <div style={{fontSize:13.5,color:'var(--paper)',lineHeight:1.8,fontWeight:300}}>{atk.how}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}


/* ── BeforeAfter — pure visual, no prose ────────────────────── */
function BeforeAfter() {
  const rows = [
    { label:'Passwords stored',    before:'Yes — in DB',      after:'Never',           cat:'storage'  },
    { label:'Breach exposure',     before:'All users',        after:'Zero',             cat:'attack'   },
    { label:'Auth factor',         before:'Memory',           after:'Hardware key',     cat:'method'   },
    { label:'Replay attacks',      before:'Possible',         after:'Math prevents it', cat:'attack'   },
    { label:'Recovery if lost',    before:'Email reset link', after:'Time-lock + trust','cat':'recovery'},
  ];
  return (
    <div className="rv">
      {/* Header row */}
      <div style={{display:'grid',gridTemplateColumns:'2fr 1fr 1fr',border:'1px solid var(--line)',borderBottom:'none'}}>
        <div style={{padding:'14px 24px',fontFamily:'var(--mono)',fontSize:9,letterSpacing:'.12em',color:'var(--muted)',textTransform:'uppercase',borderRight:'1px solid var(--line)'}}>Capability</div>
        <div style={{padding:'14px 24px',fontFamily:'var(--mono)',fontSize:9,letterSpacing:'.12em',color:'#F87171',textTransform:'uppercase',borderRight:'1px solid var(--line)',background:'rgba(248,113,113,0.04)'}}>✗ Password Model</div>
        <div style={{padding:'14px 24px',fontFamily:'var(--mono)',fontSize:9,letterSpacing:'.12em',color:'var(--accent)',textTransform:'uppercase',background:'rgba(200,245,90,0.04)'}}>✓ Crypton</div>
      </div>
      {/* Data rows */}
      {rows.map((r,i)=>(
        <div key={r.label} style={{display:'grid',gridTemplateColumns:'2fr 1fr 1fr',
          border:'1px solid var(--line)',borderBottom:i<rows.length-1?'none':'1px solid var(--line)'}}>
          <div style={{padding:'16px 24px',fontSize:13,color:'var(--paper)',fontWeight:500,
            borderRight:'1px solid var(--line)',display:'flex',alignItems:'center',
            background:i%2===0?'var(--ink-2)':'var(--ink)'}}>{r.label}</div>
          <div style={{padding:'16px 24px',fontFamily:'var(--mono)',fontSize:11,color:'rgba(248,113,113,0.7)',
            borderRight:'1px solid var(--line)',background:i%2===0?'rgba(248,113,113,0.03)':'rgba(248,113,113,0.05)',
            display:'flex',alignItems:'center'}}>{r.before}</div>
          <div style={{padding:'16px 24px',fontFamily:'var(--mono)',fontSize:11,color:'rgba(200,245,90,0.85)',
            background:i%2===0?'rgba(200,245,90,0.03)':'rgba(200,245,90,0.05)',
            display:'flex',alignItems:'center'}}>{r.after}</div>
        </div>
      ))}
    </div>
  );
}


function HiWCard({ n, t, b }) {
  const [hov, setHov] = useState(false);
  return (
    <div onMouseEnter={() => setHov(true)} onMouseLeave={() => setHov(false)}
      style={{ background: hov ? "var(--ink-2)" : "var(--ink)", padding: "44px 32px", position: "relative", overflow: "hidden", cursor: "default", transition: "background .35s", borderRight: "1px solid var(--line)" }}>
      {/* Step number — visible accent color, smaller so it doesn't dominate */}
      <div style={{ fontFamily: "var(--display)", fontSize: 64, color: hov ? "rgba(200,245,90,0.55)" : "rgba(200,245,90,0.22)", lineHeight: 1, marginBottom: 16, transition: "color .3s", letterSpacing: ".02em" }}>{n}</div>
      <div style={{ fontFamily: "var(--display)", fontSize: 36, letterSpacing: ".05em", textTransform: "uppercase", marginBottom: 14, lineHeight: 1 }}>{t}</div>
      <div style={{ fontSize: 14, color: "var(--muted)", lineHeight: 1.75, fontWeight: 300 }}>{b}</div>
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
