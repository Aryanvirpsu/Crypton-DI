import { useEffect, useRef, useCallback } from 'react';

const HOVER_RADIUS = 160;
const STAGGER_MS   = 48;
const DROP_MS      = 520;

export default function PanelWall({ onDone, intro = true }) {
  const isMobile = typeof window !== 'undefined' && window.innerWidth < 768;
  const COLS = isMobile ? 6 : 10;
  const ROWS = isMobile ? 10 : 8;

  const containerRef = useRef(null);
  const panelRefs    = useRef([]);

  const totalMs = ((ROWS - 1) + (COLS - 1)) * STAGGER_MS + DROP_MS + 400;

  useEffect(() => {
    if (!intro) return;
    const t = setTimeout(onDone, totalMs);
    return () => clearTimeout(t);
  }, [intro, onDone, totalMs]);

  // Hover glow — document-level so it works in background mode too
  const handleMouseMove = useCallback((e) => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pw = rect.width  / COLS;
    const ph = rect.height / ROWS;

    panelRefs.current.forEach((wrapper, idx) => {
      if (!wrapper) return;
      const col = idx % COLS;
      const row = Math.floor(idx / COLS);
      const cx  = rect.left + (col + 0.5) * pw;
      const cy  = rect.top  + (row + 0.5) * ph;
      const dist = Math.sqrt((e.clientX - cx) ** 2 + (e.clientY - cy) ** 2);
      const t    = Math.max(0, 1 - dist / HOVER_RADIUS);
      const inner = wrapper.firstElementChild;
      if (inner) {
        inner.style.boxShadow = t > 0.02
          ? `inset 0 0 28px rgba(200,245,90,${(t * 0.6).toFixed(2)})`
          : 'none';
      }
    });
  }, [COLS, ROWS]);

  useEffect(() => {
    document.addEventListener('mousemove', handleMouseMove);
    return () => document.removeEventListener('mousemove', handleMouseMove);
  }, [handleMouseMove]);

  const panels = [];
  for (let row = 0; row < ROWS; row++) {
    for (let col = 0; col < COLS; col++) {
      const idx   = row * COLS + col;
      const delay = (row + col) * STAGGER_MS;
      const depth = ((idx * 7 + idx * idx * 3) % 11) - 3;
      // Subtle shade variation so tiles don't look uniform
      const bg = depth > 4 ? '#161616' : depth > 1 ? '#111111' : '#0D0D0D';

      panels.push(
        <div
          key={idx}
          ref={el => { panelRefs.current[idx] = el; }}
          style={{ borderRadius: 6, overflow: 'hidden' }}
        >
          <div
            style={{
              width: '100%',
              height: '100%',
              background: bg,
              borderRadius: 6,
              ...(intro
                ? { animation: `panelDrop ${DROP_MS}ms cubic-bezier(.16,1,.3,1) ${delay}ms both` }
                : { opacity: 1 }
              ),
            }}
          />
        </div>
      );
    }
  }

  return (
    <div
      ref={containerRef}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: intro ? 10000 : 0,
        pointerEvents: intro ? 'auto' : 'none',
        display: 'grid',
        gridTemplateColumns: `repeat(${COLS}, 1fr)`,
        gridTemplateRows: `repeat(${ROWS}, 1fr)`,
        gap: '1px',
        background: '#C8F55A',
        userSelect: 'none',
      }}
    >
      {panels}
    </div>
  );
}
