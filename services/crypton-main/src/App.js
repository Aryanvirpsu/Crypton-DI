import { useState } from "react";
import Landing from './Landing';
import AttackSim from './AttackSim';
import { DEMO_URL } from './config';

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
    html{scroll-behavior:smooth}
    body{font-family:var(--body);background:var(--ink);color:var(--paper);overflow-x:hidden;line-height:1.6;cursor:auto;font-size:15px}
  `}</style>
);

function App() {
  const [simOpen, setSimOpen] = useState(false);

  const handleGo = (action) => {
    const demoPorts = ['demo', 'register', 'login', 'dashboard'];
    const adminPorts = ['admin', 'risk', 'security', 'auditlogs', 'sessions', 'rbac', 'policy'];

    if (demoPorts.includes(action)) {
      window.location.href = DEMO_URL;
    } else if (adminPorts.includes(action)) {
      window.location.href = DEMO_URL + '/admin/login';
    }
  };

  return (
    <>
      <FontLink />
      <div className="grain" />
      <Landing 
        go={handleGo} 
        toast={(m) => console.log(m)} 
        openSim={() => setSimOpen(true)} 
      />
      {simOpen && <AttackSim close={() => setSimOpen(false)} />}
    </>
  );
}

export default App;
