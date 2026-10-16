import { useEffect, useLayoutEffect, useRef, useState } from 'react';

/**
 * Small-screen layout plumbing for the immersive Agent Floor. None of this runs on desktop.
 *
 *  - useSafeRect: measures the band of the stage the HUD leaves free (below the header, above
 *    the story text), so the camera can fit the cast inside it instead of guessing a ratio.
 *  - useLabelSolver: keeps the world's name tags apart and on screen. Tags are real buttons
 *    anchored in 3D by drei <Html>; the solver nudges overlapping ones apart with a CSS
 *    transform on the tag itself (so it never fights the anchor's own positioning), and
 *    fades out a tag that cannot be placed without covering a more important one or the HUD.
 */

/**
 * Pure: the part of the stage, in px from its top-left, that no HUD element covers.
 * Portrait phones: the band between the header and the story text (full width).
 * Landscape phones: the story sits in a left column, so the band runs from the header to the tray
 * and from the column's right edge to the screen edge.
 */
export function safeBand({ navBottom, blockTop, stageHeight, stageWidth = 0, columnRight = 0, trayTop = 0 }) {
  const top = Math.max(0, navBottom) + 6;
  if (columnRight > 0) {
    const bottom = Math.max((trayTop || stageHeight) - 8, top + 120);
    return { top: Math.round(top), bottom: Math.round(Math.min(bottom, stageHeight)), left: Math.round(columnRight + 8), right: Math.round(stageWidth) };
  }
  const bottom = Math.max(blockTop - 12, top + 120);
  return { top: Math.round(top), bottom: Math.round(Math.min(bottom, stageHeight)) };
}

const LANDSCAPE = '(max-width: 1023px) and (orientation: landscape) and (max-height: 520px)';

export function useSafeRect({ enabled, step, vw, vh }) {
  const [safe, setSafe] = useState(null);
  useLayoutEffect(() => {
    if (!enabled) { setSafe(null); return undefined; }
    const measure = () => {
      const sticky = document.querySelector('.af-sticky');
      if (!sticky) return;
      const stage = sticky.getBoundingClientRect();
      const nav = document.querySelector('.af-nav');
      const block = step === 0 ? document.querySelector('.af-hud-hero > .af-eyebrow') : document.querySelector('.af-hud-caption');
      const landscape = typeof window.matchMedia === 'function' && window.matchMedia(LANDSCAPE).matches;
      const column = landscape ? (step === 0 ? document.querySelector('.af-hud-hero') : document.querySelector('.af-hud-caption')) : null;
      const bar = document.querySelector('.af-hud-bar');
      const next = safeBand({
        navBottom: nav ? nav.getBoundingClientRect().bottom - stage.top : 56,
        blockTop: block ? block.getBoundingClientRect().top - stage.top : stage.height * 0.6,
        stageHeight: stage.height,
        stageWidth: stage.width,
        columnRight: column ? column.getBoundingClientRect().right - stage.left : 0,
        trayTop: bar ? bar.getBoundingClientRect().top - stage.top : 0,
      });
      setSafe((prev) => (prev && prev.top === next.top && prev.bottom === next.bottom && prev.left === next.left && prev.right === next.right ? prev : next));
    };
    measure();
    // web fonts and the first paint of a new caption can change heights a moment later
    const t1 = setTimeout(measure, 60);
    const t2 = setTimeout(measure, 400);
    window.addEventListener('resize', measure);
    return () => { clearTimeout(t1); clearTimeout(t2); window.removeEventListener('resize', measure); };
  }, [enabled, step, vw, vh]);
  return safe;
}

const PAD = 6; // keep tags this far from the screen edge and from each other

function priority(el) {
  if (el.classList.contains('af-wtag--agent')) return 4;
  if (el.classList.contains('af-wtag--station')) return 3;
  if (el.classList.contains('af-wtag--chip')) return 2;
  return 1; // place names
}

/** Pure: given base rects {l,t,w,h,p}, return per-index {dx,dy,hide} so nothing overlaps or leaves the stage. */
export function solveLabels(rects, { width, top, bottom, left = 0 }) {
  const out = rects.map(() => ({ dx: 0, dy: 0, hide: false }));
  const order = rects.map((_, i) => i).sort((a, b) => rects[b].p - rects[a].p);
  const box = (i) => ({ l: rects[i].l + out[i].dx, t: rects[i].t + out[i].dy, w: rects[i].w, h: rects[i].h });
  const overlap = (a, b) => {
    const x = Math.min(a.l + a.w, b.l + b.w) - Math.max(a.l, b.l) + PAD;
    const y = Math.min(a.t + a.h, b.t + b.h) - Math.max(a.t, b.t) + PAD;
    return x > 0 && y > 0 ? { x, y } : null;
  };

  // an anchor that is not on the stage at all belongs to something the camera is not showing
  rects.forEach((r, i) => {
    const cx = r.l + r.w / 2;
    const cy = r.t + r.h / 2;
    if (cx < left - r.w * 0.1 || cx > width + r.w * 0.1 || cy < top - 8 || cy > bottom + 8) out[i].hide = true;
  });

  for (let pass = 0; pass < 4; pass += 1) {
    for (let a = 0; a < order.length; a += 1) {
      const i = order[a];
      if (out[i].hide) continue;
      // stay on the stage horizontally
      const b0 = box(i);
      if (b0.l < left + PAD) out[i].dx += left + PAD - b0.l;
      else if (b0.l + b0.w > width - PAD) out[i].dx -= b0.l + b0.w - (width - PAD);
      for (let c = 0; c < a; c += 1) {
        const j = order[c];
        if (out[j].hide) continue;
        const ov = overlap(box(i), box(j));
        if (!ov) continue;
        const bi = box(i);
        const bj = box(j);
        if (ov.y <= ov.x * 1.4) { // separate vertically, away from the more important tag
          out[i].dy += bi.t + bi.h / 2 < bj.t + bj.h / 2 ? -ov.y : ov.y;
        } else {
          out[i].dx += bi.l + bi.w / 2 < bj.l + bj.w / 2 ? -ov.x : ov.x;
        }
      }
    }
  }

  // anything still colliding with a more important tag, or pushed off the stage, steps aside
  order.forEach((i, a) => {
    if (out[i].hide) return;
    const bi = box(i);
    if (bi.l < left || bi.l + bi.w > width || bi.t < top - 6 || bi.t + bi.h > bottom + 6) { out[i].hide = true; return; }
    for (let c = 0; c < a; c += 1) {
      const j = order[c];
      if (!out[j].hide && overlap(bi, box(j))) { out[i].hide = true; return; }
    }
  });
  return out;
}

export function useLabelSolver(layerRef, { enabled, safe, reduced }) {
  // the loop keeps running (and keeps each tag's current offset) while the safe band changes between scenes
  const safeRef = useRef(safe);
  safeRef.current = safe;
  useEffect(() => {
    const layer = layerRef.current;
    if (!enabled || !layer) return undefined;
    const state = new WeakMap();
    let raf = 0;
    let last = 0;
    const tick = (now) => {
      raf = requestAnimationFrame(tick);
      if (now - last < 40) return; // ~25 Hz is plenty for tags that ride a damped camera
      // smoothing is by elapsed time, so the tags settle just as fast on a slow or throttled frame rate
      const dt = last ? Math.min(now - last, 250) : 250;
      last = now;
      const band = safeRef.current;
      if (!band) return;
      const { top, bottom, left = 0 } = band;
      const lr = layer.getBoundingClientRect();
      const els = [...layer.querySelectorAll('.af-wtag, .af-wplace')].filter((el) => !el.hidden && el.offsetParent !== null);
      const rects = els.map((el) => {
        const s = state.get(el) || { dx: 0, dy: 0 };
        const r = el.getBoundingClientRect();
        return { l: r.left - lr.left - s.dx, t: r.top - lr.top - s.dy, w: r.width, h: r.height, p: priority(el) };
      });
      const solved = solveLabels(rects, { width: lr.width, top, bottom, left });
      els.forEach((el, i) => {
        const s = state.get(el) || { dx: 0, dy: 0 };
        const k = reduced ? 1 : 1 - Math.exp(-dt / 90);
        const nx = s.dx + (solved[i].dx - s.dx) * k;
        const ny = s.dy + (solved[i].dy - s.dy) * k;
        if (Math.abs(nx - s.dx) > 0.15 || Math.abs(ny - s.dy) > 0.15) {
          el.style.transform = `translate3d(${nx.toFixed(1)}px, ${ny.toFixed(1)}px, 0)`;
        }
        state.set(el, { dx: nx, dy: ny });
        el.classList.toggle('is-yielded', solved[i].hide);
      });
    };
    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      layer.querySelectorAll('.af-wtag, .af-wplace').forEach((el) => { el.style.transform = ''; el.classList.remove('is-yielded'); });
    };
  }, [layerRef, enabled, reduced]);
}
