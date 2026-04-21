import { useEffect, useRef, useCallback } from 'react';

const HOVER_RADIUS = 160;
const STAGGER_MS   = 60;
const DROP_MS      = 620;
const LAND_MS      = 500;

// Tile backgrounds — subtle radial gradient gives a soft pillowed 3D look
const TILE_BG = [
  'radial-gradient(ellipse at 42% 36%, #1a1a1a 0%, #0d0d0d 52%, #080808 100%)',
  'radial-gradient(ellipse at 38% 32%, #161616 0%, #0b0b0b 50%, #070707 100%)',
  'radial-gradient(ellipse at 44% 40%, #181818 0%, #0e0e0e 54%, #090909 100%)',
  'radial-gradient(ellipse at 36% 34%, #141414 0%, #0c0c0c 50%, #080808 100%)',
];

export default function PanelWall({ onDone, intro = true }) {
  // Derive grid dimensions so tiles are square and ~200 total
  const vw = typeof window !== 'undefined' ? window.innerWidth  : 1920;
  const vh = typeof window !== 'undefined' ? window.innerHeight : 1080;
  const CELL = Math.sqrt((vw * vh) / 200);
  const COLS = Math.max(8,  Math.round(vw / CELL));
  const ROWS = Math.max(6,  Math.round(vh / CELL));

  const containerRef = useRef(null);
  const innerRefs    = useRef([]);

  // Fire onDone after all tiles have landed + buffer
  const totalMs = ((ROWS - 1) + (COLS - 1)) * STAGGER_MS + DROP_MS + 500;

  useEffect(() => {
    if (!intro) return;
    const t = setTimeout(onDone, totalMs);
    return () => clearTimeout(t);
  }, [intro, onDone, totalMs]);

  // Hover glow via document listener so it works behind page content too
  const handleMouseMove = useCallback((e) => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pw = rect.width  / COLS;
    const ph = rect.height / ROWS;

    innerRefs.current.forEach((inner, idx) => {
      if (!inner) return;
      const col = idx % COLS;
      const row = Math.floor(idx / COLS);
      const cx  = rect.left + (col + 0.5) * pw;
      const cy  = rect.top  + (row + 0.5) * ph;
      const dist = Math.sqrt((e.clientX - cx) ** 2 + (e.clientY - cy) ** 2);
      const t    = Math.max(0, 1 - dist / HOVER_RADIUS);
      inner.style.boxShadow = t > 0.02
        ? `inset 0 0 24px rgba(200,245,90,${(t * 0.5).toFixed(2)})`
        : 'none';
    });
  }, [COLS, ROWS]);

  useEffect(() => {
    document.addEventListener('mousemove', handleMouseMove);
    return () => document.removeEventListener('mousemove', handleMouseMove);
  }, [handleMouseMove]);

  const panels = [];
  for (let row = 0; row < ROWS; row++) {
    for (let col = 0; col < COLS; col++) {
      const idx      = row * COLS + col;
      const delay    = (row + col) * STAGGER_MS;
      const landDelay = delay + Math.round(DROP_MS * 0.78);
      const bg       = TILE_BG[idx % TILE_BG.length];

      panels.push(
        <div
          key={idx}
          style={{
            // Heavy rounding makes each tile look pillowed / soft
            borderRadius: '22%',
            // Animation on the WRAPPER so translateY(-100vh) escapes container
            // without overflow:hidden clipping it
            ...(intro ? {
              animation: `panelDrop ${DROP_MS}ms cubic-bezier(0.16,1,0.3,1) ${delay}ms both`,
            } : {}),
          }}
        >
          <div
            ref={el => { innerRefs.current[idx] = el; }}
            style={{
              width: '100%',
              height: '100%',
              borderRadius: '22%',
              background: bg,
              // Landing flash: brief green glow when tile hits its position
              ...(intro ? {
                animation: `panelLand ${LAND_MS}ms ease-out ${landDelay}ms forwards`,
              } : {}),
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
        zIndex: intro ? 10000 : -1,
        pointerEvents: intro ? 'auto' : 'none',
        display: 'grid',
        gridTemplateColumns: `repeat(${COLS}, 1fr)`,
        gridTemplateRows: `repeat(${ROWS}, 1fr)`,
        // Black background = black gaps between tiles, no green flood
        gap: '3px',
        background: '#000000',
        userSelect: 'none',
      }}
    >
      {panels}
    </div>
  );
}
