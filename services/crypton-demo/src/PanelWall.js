import { useEffect, useRef, useCallback } from 'react';

const HOVER_RADIUS = 160;

export default function PanelWall({ onDone }) {
  const isMobile = typeof window !== 'undefined' && window.innerWidth < 768;
  const COLS = isMobile ? 6 : 10;
  const ROWS = isMobile ? 10 : 8;

  const containerRef = useRef(null);
  const panelRefs = useRef([]);

  // Total time = max diagonal delay + animation duration + buffer
  const totalMs = ((ROWS - 1) + (COLS - 1)) * 60 + 750 + 400;

  useEffect(() => {
    const t = setTimeout(onDone, totalMs);
    return () => clearTimeout(t);
  }, [onDone, totalMs]);

  const handleMouseMove = useCallback((e) => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pw = rect.width / COLS;
    const ph = rect.height / ROWS;
    const mx = e.clientX;
    const my = e.clientY;

    panelRefs.current.forEach((wrapper, idx) => {
      if (!wrapper) return;
      const col = idx % COLS;
      const row = Math.floor(idx / COLS);
      const cx = rect.left + (col + 0.5) * pw;
      const cy = rect.top + (row + 0.5) * ph;
      const dist = Math.sqrt((mx - cx) ** 2 + (my - cy) ** 2);
      const t = Math.max(0, 1 - dist / HOVER_RADIUS);
      wrapper.style.transform = `scale(${(1 + t * 0.045).toFixed(4)})`;
      const inner = wrapper.firstElementChild;
      if (inner) {
        inner.style.boxShadow = `inset 0 0 24px rgba(200,245,90,${(0.08 + t * 0.72).toFixed(2)})`;
      }
    });
  }, [COLS, ROWS]);

  const panels = [];
  for (let row = 0; row < ROWS; row++) {
    for (let col = 0; col < COLS; col++) {
      const idx = row * COLS + col;
      // Diagonal sweep top-right → bottom-left: highest delay at top-right (row=0, col=max)
      const delay = (row + (COLS - 1 - col)) * 60;
      panels.push(
        <div
          key={idx}
          ref={el => { panelRefs.current[idx] = el; }}
          style={{ willChange: 'transform', transition: 'transform 0.15s ease' }}
        >
          <div
            style={{
              width: '100%',
              height: '100%',
              background: '#0A0A0A',
              boxShadow: 'inset 0 0 24px rgba(200,245,90,0.08)',
              animation: `panelReveal 0.75s ease-out ${delay}ms both`,
              willChange: 'opacity',
            }}
          />
        </div>
      );
    }
  }

  return (
    <div
      ref={containerRef}
      onMouseMove={handleMouseMove}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 10000,
        display: 'grid',
        gridTemplateColumns: `repeat(${COLS}, 1fr)`,
        gridTemplateRows: `repeat(${ROWS}, 1fr)`,
        gap: '1px',
        background: '#C8F55A',
        cursor: 'crosshair',
        userSelect: 'none',
      }}
    >
      {panels}
    </div>
  );
}
