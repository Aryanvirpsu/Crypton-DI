import { useState, useEffect, useRef } from "react";

export default function AttackSim({ close }) {
  const [chosen, setChosen] = useState(null);
  const [phase, setPhase] = useState('pick');
  const [lines, setLines] = useState([]);
  const [defIdx, setDefIdx] = useState(0);
  const [blink, setBlink] = useState(true);
  const inputRef = useRef(null);
  const termRef = useRef(null);

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
    { id:'session',    label:'Session Hijack',      severity:'CRITICAL', color:'#F87171',
      desc:'Stolen cookie replayed for access',
      attack:['> XSS payload injected in comment field','> Script executing in victim browser...','> Exfiltrating document.cookie','> session_id=a3f7x9 captured','> Forged request → GET /dashboard','> Cookie attached — awaiting response...'],
      defense:['$ crypton session:validate a3f7x9','  Enrolled device → MacBook-A3F7','  Request device → UNKNOWN','  Attestation → FAILED','$ crypton session:policy','  Sessions are hardware-bound.','  [REJECTED] Cookie without device is worthless.','  [SECURE] Session hijack blocked at attestation layer.'],
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

  return (
    <div style={{
      position:'fixed', inset:0, zIndex:10001,
      background: '#030303',
      display:'flex', alignItems:'stretch', justifyContent:'stretch',
      fontFamily:"'DM Mono','Courier New',monospace",
    }}>
      <div style={{
        position:'absolute', inset:0, pointerEvents:'none', zIndex:1,
        backgroundImage:'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(0,0,0,0.03) 2px, rgba(0,0,0,0.03) 4px)',
      }}/>
      <div style={{
        width:'100vw', height:'100vh', position:'relative', zIndex:2,
        display:'flex', flexDirection:'column', overflow:'hidden',
      }}>
        <div style={{
          padding:'12px 20px', flexShrink:0,
          background:'#0d0d0d', borderBottom:'1px solid rgba(200,245,90,0.15)',
          display:'flex', alignItems:'center', gap:12,
        }}>
          <button onClick={close} style={{width:13,height:13,borderRadius:'50%',background:'#F87171',border:'none',cursor:'pointer'}} />
          <div style={{display:'flex',alignItems:'center',gap:8,padding:'4px 16px', border:'1px solid rgba(200,245,90,0.15)',background:'rgba(200,245,90,0.04)'}}>
            <span style={{fontSize:10,color:'rgba(200,245,90,0.7)',letterSpacing:'.1em',textTransform:'uppercase'}}>crypton — attack simulator</span>
          </div>
        </div>

        <div style={{flex:1,display:'grid',gridTemplateColumns:'280px 1fr',overflow:'hidden'}}>
          <div style={{background:'#0a0a0a', borderRight:'1px solid rgba(200,245,90,0.12)', display:'flex', flexDirection:'column'}}>
            <div style={{padding:'20px'}}>
              <div style={{fontSize:9,color:'rgba(200,245,90,0.4)',letterSpacing:'.18em',textTransform:'uppercase',marginBottom:4}}>// vectors</div>
              {ATTACKS.map(atk => (
                <div key={atk.id} onClick={()=>startAttack(atk)} style={{
                  padding:'14px', marginBottom:8, cursor:'pointer',
                  border:`1px solid ${chosen?.id===atk.id ? atk.color : '#222'}`,
                  background: chosen?.id===atk.id ? `${atk.color}11` : 'transparent'
                }}>
                  <div style={{fontSize:11, color:atk.color}}>{atk.label}</div>
                  <div style={{fontSize:9, color:'#666'}}>{atk.desc}</div>
                </div>
              ))}
            </div>
          </div>
          <div style={{display:'flex',flexDirection:'column',background:'#030303'}}>
            <div ref={termRef} style={{flex:1, overflowY:'auto', padding:'24px', lineHeight:1.8, fontSize:12}}>
              {lines.map((l,i) => (
                <div key={i} style={{color: l.type==='attack'?'#F87171':l.type==='defense'?'#C8F55A':'#555'}}>
                  {l.text}
                </div>
              ))}
              {phase==='defending' && (
                <div style={{display:'flex',gap:8,color:'#C8F55A'}}>
                   <span>$</span>
                   <input autoFocus onKeyDown={handleKey} style={{background:'transparent',border:'none',outline:'none',color:'#C8F55A',flex:1}} />
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
