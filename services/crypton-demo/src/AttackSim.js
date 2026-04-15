import { useState, useEffect, useRef } from "react";

export default function AttackSim({ animState, onClose }) {
  const [chosen,  setChosen]  = useState(null);
  const [phase,   setPhase]   = useState('pick');
  const [lines,   setLines]   = useState([]);
  const [defIdx,  setDefIdx]  = useState(0);
  const [blink,   setBlink]   = useState(true);
  const inputRef = useRef(null);
  const termRef  = useRef(null);

  const isOpen    = animState === 'open';
  const isClosing = animState === 'closing';

  // Cursor blink
  useEffect(() => {
    const id = setInterval(() => setBlink(b => !b), 530);
    return () => clearInterval(id);
  }, []);

  const ATTACKS = [
    { id:'phishing',  label:'Phishing',           severity:'CRITICAL', color:'#F87171',
      desc:'Fake login page harvests credentials',
      attack:['> Launching PhishKit v4.2...','> Cloning crypton.io login page...','> Hosting on phish-secure-login.net','> Sending phishing email to target@org.com','> Waiting for credentials input...','> Connection established — capturing...'],
      defense:['$ crypton auth:verify --origin','  ✗ Origin mismatch: phish-secure-login.net','  ✗ Expected: crypton.io','$ crypton auth:method --user target@org.com','  Method → passkey (hardware-bound)','  Password field → DOES NOT EXIST','  [BLOCKED] No credentials to harvest.','  [SECURE] Hardware key required. Phishing yields nothing.'],
    },
    { id:'bruteforce', label:'Brute Force',        severity:'HIGH',     color:'#FBBF24',
      desc:'10,000 password attempts per second',
      attack:['> Loading rockyou.txt — 14.3M entries','> Target: aryan@crypton.io','> Rate limiter: disabled','> Trying: password123 ✗','> Trying: crypton2026 ✗','> Trying: qwerty!@#$ ✗'],
      defense:['$ crypton auth:surface --check','  Password auth → DISABLED','  Required → hardware device signature','$ crypton shield:status','  14,000,000 attempts → 0 success rate','  Attack surface → 0 bytes','  [BLOCKED] No password to brute force.','  [SECURE] Guessing is mathematically irrelevant.'],
    },
    { id:'mitm',       label:'Man in the Middle',  severity:'CRITICAL', color:'#F87171',
      desc:'Intercepts auth tokens in transit',
      attack:['> ARP poisoning local gateway...','> mitmproxy active on :8080','> Intercepting TLS session...','> Captured: Authorization: Bearer eyJhbGc...','> Replaying token → POST /api/auth','> Awaiting 200 OK...'],
      defense:['$ crypton nonce:validate eyJhbGc...','  Issued: 14:33:01.042','  Now:    14:33:01.891  (+849ms)','  TTL: 500ms → EXPIRED','  Status → CONSUMED','  [REJECTED] Nonce already spent.','  [BLOCKED] Replayed token is worthless.','  [SECURE] Every nonce is single-use, time-bound.'],
    },
    { id:'stuffing',   label:'Credential Stuffing', severity:'HIGH',    color:'#FBBF24',
      desc:'2.4M leaked passwords tried at once',
      attack:['> breach2024.txt loaded — 2.4M pairs','> Targeting crypton.io endpoint','> aryan@crypton.io:LeakedPass#99 →','> aryan@crypton.io:Summer2024! →','> aryan@crypton.io:Crypton123 →','> 2,400,000 pairs queued...'],
      defense:['$ crypton account:method aryan@crypton.io','  Auth type → passkey','  Password hash → NONE (never set)','$ crypton threat:assess','  Leaked credentials applicable → 0','  Surface area for stuffing → 0 bytes','  [BLOCKED] Nothing to stuff.','  [SECURE] Leaked passwords are useless here.'],
    },
    { id:'session',    label:'Session Hijack',      severity:'CRITICAL', color:'#F87171',
      desc:'Stolen cookie replayed for access',
      attack:['> XSS payload injected in comment field','> Script executing in victim browser...','> Exfiltrating document.cookie','> session_id=a3f7x9 captured','> Forged request → GET /dashboard','> Cookie attached — awaiting response...'],
      defense:['$ crypton session:validate a3f7x9','  Enrolled device → MacBook-A3F7','  Request device → UNKNOWN','  Attestation → FAILED','$ crypton session:policy','  Sessions are hardware-bound.','  [REJECTED] Cookie without device is worthless.','  [SECURE] Session hijack blocked at attestation layer.'],
    },
    { id:'replay',     label:'Replay Attack',       severity:'HIGH',     color:'#FBBF24',
      desc:'Valid signed request sent again later',
      attack:['> Listening on network interface...','> Captured: POST /auth','> Payload: {sig:"d4f2a1",nonce:"n_882"}','> Waiting 900ms to avoid detection...','> Replaying identical request...','> Expecting session token...'],
      defense:['$ crypton nonce:check n_882','  Issued:  20:19:30.100','  Used at: 20:19:30.412','  Status → CONSUMED (single-use)','  Current: 20:19:31.312 (+900ms)','  [REJECTED] Nonce TTL: 500ms. Window closed.','  [BLOCKED] Replay is impossible by design.','  [SECURE] Each request requires a fresh nonce.'],
    },
  ];

  const scrollBottom = () => setTimeout(() => { if (termRef.current) termRef.current.scrollTop = termRef.current.scrollHeight; }, 30);

  const startAttack = (atk) => {
    setChosen(atk);
    setPhase('attacking');
    setLines([{ text: `┌─ SIMULATING: ${atk.label.toUpperCase()} `, type:'system' }]);
    setDefIdx(0);
    let i = 0;
    const typeNext = () => {
      if (i >= atk.attack.length) {
        setTimeout(() => {
          setPhase('defending');
          setLines(prev => [...prev,
            { text:'', type:'gap' },
            { text:'┌─ CRYPTON SHIELD ACTIVATED ─────────────────', type:'intercept' },
            { text:'│  Press ENTER to execute each defense step.', type:'intercept' },
            { text:'└────────────────────────────────────────────', type:'intercept' },
            { text:'', type:'gap' },
          ]);
          scrollBottom();
          setTimeout(() => inputRef.current?.focus(), 80);
        }, 600);
        return;
      }
      setLines(prev => [...prev, { text: atk.attack[i], type:'attack' }]);
      scrollBottom();
      i++;
      setTimeout(typeNext, 240 + Math.random() * 160);
    };
    setTimeout(typeNext, 300);
  };

  const handleKey = (e) => {
    if (e.key !== 'Enter') return;
    const atk = ATTACKS.find(a => a.id === chosen?.id);
    if (!atk) return;
    if (defIdx < atk.defense.length) {
      setLines(prev => [...prev, { text: atk.defense[defIdx], type:'defense' }]);
      setDefIdx(d => d + 1);
      scrollBottom();
    } else if (phase !== 'done') {
      setPhase('done');
      setLines(prev => [...prev,
        { text:'', type:'gap' },
        { text:'╔══════════════════════════════════════════╗', type:'success' },
        { text:'║   ✓  ATTACK NEUTRALISED — ACCESS SECURED  ║', type:'success' },
        { text:'╚══════════════════════════════════════════╝', type:'success' },
      ]);
      scrollBottom();
    }
  };

  useEffect(() => {
    const fn = e => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', fn);
    return () => window.removeEventListener('keydown', fn);
  }, [onClose]);

  useEffect(() => { if (phase === 'defending') inputRef.current?.focus(); }, [phase]);

  const defLen = chosen ? chosen.defense.length : 0;
  const allDefDone = defIdx >= defLen;

  const easeIn  = 'cubic-bezier(.4,0,1,1)';
  const easeOut = 'cubic-bezier(.16,1,.3,1)';

  return (
    <div style={{
      position:'fixed', inset:0, zIndex:10001,
      background: '#030303',
      opacity: isOpen ? 1 : 0,
      transition: `opacity ${isClosing ? `0.3s ${easeIn}` : `0.25s ease`}`,
      display:'flex', alignItems:'stretch', justifyContent:'stretch',
      fontFamily:"'DM Mono','Courier New',monospace",
    }}>
      {/* Scanline overlay */}
      <div style={{
        position:'absolute', inset:0, pointerEvents:'none', zIndex:1,
        backgroundImage:'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.03) 2px, rgba(0,0,0,0.03) 4px)',
      }}/>

      {/* Main window */}
      <div style={{
        width:'100vw', height:'100vh', position:'relative', zIndex:2,
        display:'flex', flexDirection:'column', overflow:'hidden',
        border:'none',
        opacity: isOpen ? 1 : 0,
        transform: isOpen ? 'translateY(0) scale(1)' : isClosing ? 'translateY(16px) scale(0.98)' : 'translateY(32px) scale(0.96)',
        transition: isClosing
          ? `opacity 0.3s ${easeIn}, transform 0.3s ${easeIn}`
          : `opacity 0.4s ${easeOut}, transform 0.45s ${easeOut}`,
      }}>
        {/* Title bar */}
        <div style={{
          padding:'12px 20px', flexShrink:0,
          background:'#0d0d0d', borderBottom:'1px solid rgba(200,245,90,0.15)',
          display:'flex', alignItems:'center', gap:12,
        }}>
          <div style={{display:'flex',gap:8}}>
            {[['#F87171',onClose],['#FBBF24',null],['#3d9e3d',null]].map(([c,fn],i)=>(
              <div key={i} onClick={fn||undefined} style={{
                width:13,height:13,borderRadius:'50%',background:c,
                cursor:fn?'pointer':'default',
                boxShadow:`0 0 8px ${c}66`,
                transition:'transform .15s',
              }}
              onMouseEnter={e=>{if(fn)e.currentTarget.style.transform='scale(1.2)';}}
              onMouseLeave={e=>{e.currentTarget.style.transform='scale(1)';}}
              />
            ))}
          </div>
          {/* Tab */}
          <div style={{display:'flex',alignItems:'center',gap:8,padding:'4px 16px',
            border:'1px solid rgba(200,245,90,0.15)',background:'rgba(200,245,90,0.04)'}}>
            <span style={{width:6,height:6,borderRadius:'50%',background:'var(--accent)',display:'inline-block',boxShadow:'0 0 6px #C8F55A'}}/>
            <span style={{fontSize:10,color:'rgba(200,245,90,0.7)',letterSpacing:'.1em',textTransform:'uppercase'}}>
              crypton — attack simulator
            </span>
          </div>
          <div style={{marginLeft:'auto',fontSize:9,color:'rgba(255,255,255,0.2)',letterSpacing:'.12em',cursor:'pointer'}} onClick={onClose}>
            ESC TO EXIT
          </div>
        </div>

        {/* Body */}
        <div style={{flex:1,display:'grid',gridTemplateColumns:'280px 1fr',overflow:'hidden'}}>

          {/* ── Left: Attack picker ── */}
          <div style={{
            background:'#0a0a0a', borderRight:'1px solid rgba(200,245,90,0.12)',
            display:'flex', flexDirection:'column', overflow:'hidden',
          }}>
            <div style={{padding:'16px 20px',borderBottom:'1px solid rgba(255,255,255,0.05)'}}>
              <div style={{fontSize:9,color:'rgba(200,245,90,0.4)',letterSpacing:'.18em',textTransform:'uppercase',marginBottom:4}}>
                // attack vectors
              </div>
              <div style={{fontSize:11,color:'rgba(255,255,255,0.25)',lineHeight:1.5}}>
                Select a vector. Watch it fail.
              </div>
            </div>
            <div style={{flex:1,overflowY:'auto',padding:'8px'}}>
              {ATTACKS.map(atk => {
                const isChosen = chosen?.id === atk.id;
                return (
                  <div key={atk.id} onClick={()=>startAttack(atk)} style={{
                    padding:'14px 16px', marginBottom:4, cursor:'pointer',
                    background: isChosen ? `rgba(${atk.color==='#F87171'?'248,113,113':'251,191,36'},0.1)` : 'transparent',
                    border:`1px solid ${isChosen ? atk.color+'55' : 'rgba(255,255,255,0.05)'}`,
                    transition:'all .18s', position:'relative', overflow:'hidden',
                  }}
                  onMouseEnter={e=>{if(!isChosen){e.currentTarget.style.background='rgba(255,255,255,0.04)';e.currentTarget.style.borderColor='rgba(255,255,255,0.12)';}}}
                  onMouseLeave={e=>{if(!isChosen){e.currentTarget.style.background='transparent';e.currentTarget.style.borderColor='rgba(255,255,255,0.05)';}}}
                  >
                    {isChosen && <div style={{position:'absolute',top:0,left:0,bottom:0,width:2,background:atk.color}}/>}
                    <div style={{display:'flex',alignItems:'center',justifyContent:'space-between',marginBottom:5}}>
                      <div style={{fontSize:11,color:isChosen?atk.color:'rgba(255,255,255,0.55)',letterSpacing:'.05em',fontWeight:500}}>
                        {atk.label}
                      </div>
                      <div style={{fontSize:8,letterSpacing:'.1em',padding:'2px 7px',
                        color:atk.color,border:`1px solid ${atk.color}44`,
                        background:`${atk.color}11`}}>
                        {atk.severity}
                      </div>
                    </div>
                    <div style={{fontSize:10,color:'rgba(255,255,255,0.25)',lineHeight:1.4}}>{atk.desc}</div>
                  </div>
                );
              })}
            </div>
            {/* Legend */}
            <div style={{padding:'14px 20px',borderTop:'1px solid rgba(255,255,255,0.05)',
              display:'flex',flexDirection:'column',gap:6}}>
              {[['#F87171','Attack'],['#C8F55A','Defense'],['#4ADE80','Secured']].map(([c,l])=>(
                <div key={l} style={{display:'flex',alignItems:'center',gap:8,fontSize:9,
                  color:'rgba(255,255,255,0.2)',letterSpacing:'.1em'}}>
                  <div style={{width:6,height:6,borderRadius:'50%',background:c,boxShadow:`0 0 5px ${c}88`}}/>
                  {l}
                </div>
              ))}
            </div>
          </div>

          {/* ── Right: Terminal ── */}
          <div style={{display:'flex',flexDirection:'column',background:'#030303'}}>
            {/* Terminal tab bar */}
            <div style={{
              padding:'8px 16px', flexShrink:0,
              background:'#0a0a0a', borderBottom:'1px solid rgba(255,255,255,0.05)',
              display:'flex', alignItems:'center', gap:10,
            }}>
              <span style={{fontSize:9,color:'rgba(200,245,90,0.5)',letterSpacing:'.1em'}}>●</span>
              <span style={{fontSize:10,color:'rgba(255,255,255,0.2)',letterSpacing:'.08em'}}>
                bash — crypton@shield:~
              </span>
              {chosen && (
                <span style={{fontSize:9,color:`${chosen.color}77`,marginLeft:4,letterSpacing:'.06em'}}>
                  [{chosen.label}]
                </span>
              )}
              <div style={{marginLeft:'auto',display:'flex',gap:4}}>
                {phase==='done' && (
                  <div style={{fontSize:9,color:'#4ADE80',letterSpacing:'.1em',
                    padding:'3px 10px',border:'1px solid #4ADE8044',background:'#4ADE8011'}}>
                    SECURED
                  </div>
                )}
                {phase==='attacking' && (
                  <div style={{fontSize:9,color:'#F87171',letterSpacing:'.1em',
                    padding:'3px 10px',border:'1px solid #F8717144',background:'#F8717111'}}>
                    ATTACK IN PROGRESS
                  </div>
                )}
                {phase==='defending' && (
                  <div style={{fontSize:9,color:'#C8F55A',letterSpacing:'.1em',
                    padding:'3px 10px',border:'1px solid #C8F55A44',background:'#C8F55A11'}}>
                    DEFENSE {defIdx}/{defLen}
                  </div>
                )}
              </div>
            </div>

            {/* Output area */}
            <div ref={termRef} style={{
              flex:1, overflowY:'auto', padding:'20px 24px',
              lineHeight:1.85, fontSize:12,
            }}>
              {lines.length === 0 && (
                <div style={{color:'rgba(255,255,255,0.12)',lineHeight:1.8}}>
                  <div>crypton@shield:~ $</div>
                  <div style={{marginTop:8,color:'rgba(255,255,255,0.08)'}}>
                    ← select an attack vector to begin
                  </div>
                </div>
              )}
              {lines.map((line, i) => {
                if (line.type==='gap') return <div key={i} style={{height:8}}/>;
                const style = {
                  attack:   { color:'#F87171' },
                  defense:  { color:'#C8F55A' },
                  system:   { color:'rgba(255,255,255,0.18)', fontStyle:'italic' },
                  intercept:{ color:'rgba(200,245,90,0.45)' },
                  success:  { color:'#4ADE80', fontWeight:600 },
                  hint:     { color:'rgba(200,245,90,0.3)', fontStyle:'italic' },
                }[line.type] || { color:'rgba(255,255,255,0.35)' };
                return (
                  <div key={i} style={{...style, whiteSpace:'pre-wrap', wordBreak:'break-all', marginBottom:1}}>
                    {line.text}
                  </div>
                );
              })}
              {/* Input prompt */}
              {phase==='defending' && (
                <div style={{display:'flex',alignItems:'center',gap:8,marginTop:6}}>
                  <span style={{color:'#C8F55A'}}>$</span>
                  <input ref={inputRef} onKeyDown={handleKey}
                    placeholder={allDefDone ? 'press ENTER to complete' : 'press ENTER to run next defense'}
                    style={{
                      flex:1, background:'transparent', border:'none', outline:'none',
                      color:'#C8F55A', fontSize:12, fontFamily:"'DM Mono',monospace",
                      letterSpacing:'.04em', caretColor:'#C8F55A',
                    }}
                  />
                  <span style={{color:'#C8F55A',opacity:blink?1:0,fontSize:14,lineHeight:1}}>█</span>
                </div>
              )}
            </div>

            {/* Bottom status */}
            <div style={{
              padding:'8px 20px', flexShrink:0,
              borderTop:'1px solid rgba(255,255,255,0.05)',
              background:'#0a0a0a',
              display:'flex', justifyContent:'space-between', alignItems:'center',
              fontSize:9, letterSpacing:'.1em', color:'rgba(255,255,255,0.15)',
            }}>
              <div style={{display:'flex',gap:20}}>
                <span>CRYPTON SHIELD v1.0</span>
                <span style={{color:'rgba(200,245,90,0.3)'}}>ZERO-TRUST ACTIVE</span>
              </div>
              <div>PRESS ESC TO EXIT</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
