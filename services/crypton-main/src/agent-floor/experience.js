import { useEffect, useRef, useState } from 'react';

/**
 * The experience director's plumbing: which renderer to use, and how page scroll maps
 * onto scenes. It moves the visitor through the scenario; it never decides what is true
 * at a scene (state.js and scenario.js do).
 */

/** WebGL is usable here. False in jsdom, old browsers and locked-down GPUs. */
export function canUseWebGL() {
  if (typeof window === 'undefined' || !window.WebGLRenderingContext) return false;
  try {
    const c = document.createElement('canvas');
    return Boolean(c.getContext('webgl2') || c.getContext('webgl'));
  } catch (e) {
    return false;
  }
}

/** 'immersive' (3D world, scroll-driven) or 'classic' (2D stage). `?renderer=2d` forces classic. */
export function initialMode() {
  if (typeof window === 'undefined') return 'classic';
  const q = new URLSearchParams(window.location.search);
  if (q.get('renderer') === '2d') return 'classic';
  return canUseWebGL() ? 'immersive' : 'classic';
}

/** `?scene=N` deep link, or null. */
export function sceneFromURL(count) {
  if (typeof window === 'undefined') return null;
  const n = Number(new URLSearchParams(window.location.search).get('scene'));
  return Number.isInteger(n) && n >= 0 && n < count ? n : null;
}

const clamp = (v, a, b) => Math.min(b, Math.max(a, v));

/**
 * Two-way binding between page scroll and the scene. Native scroll, no hijacking: the
 * track is a tall section with a sticky stage inside, and each scene owns an equal slice
 * of it. Scrolling into a slice selects that scene; choosing a scene with a button jumps
 * the page to its slice, so the two can never disagree.
 *
 * progressRef receives 0..1, how far the visitor has scrolled through the current scene.
 */
export function useSceneScroll({ enabled, trackRef, step, count, onScene, progressRef, pausedRef }) {
  const shown = useRef(0);
  // While the page is jumping to a scene, ignore the slices it passes on the way.
  const jumping = useRef(null);

  // scene -> scroll (buttons, keys, deep links). Declared first so a deep link wins on mount.
  useEffect(() => {
    if (!enabled || step === shown.current) return;
    shown.current = step;
    const el = trackRef.current;
    if (!el) return;
    const span = el.offsetHeight - window.innerHeight;
    const top = el.getBoundingClientRect().top + window.scrollY + ((step + 0.04) / count) * span;
    jumping.current = step;
    // 'instant' overrides the site-wide `scroll-behavior: smooth`; the camera does the moving.
    window.scrollTo({ top, behavior: 'instant' });
  }, [enabled, step, count, trackRef]);

  // scroll -> scene
  useEffect(() => {
    if (!enabled) return undefined;
    let raf = 0;
    const read = () => {
      raf = 0;
      const el = trackRef.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const span = r.height - window.innerHeight;
      if (span <= 0) return;
      const x = clamp(-r.top / span, 0, 0.99999) * count;
      const scene = Math.floor(x);
      if (progressRef) progressRef.current = x - scene;
      // While the visitor is typing in the final form, page scroll (keyboard, focus) must not change scene.
      if (pausedRef && pausedRef.current) return;
      if (jumping.current !== null) {
        if (scene !== jumping.current) return;
        jumping.current = null;
      }
      if (scene !== shown.current) {
        shown.current = scene;
        onScene(scene);
      }
    };
    const schedule = () => { if (!raf) raf = requestAnimationFrame(read); };
    // The visitor taking over the scroll ends any jump in progress.
    const userScroll = () => { jumping.current = null; };
    window.addEventListener('scroll', schedule, { passive: true });
    window.addEventListener('resize', schedule);
    window.addEventListener('wheel', userScroll, { passive: true });
    window.addEventListener('touchstart', userScroll, { passive: true });
    read();
    return () => {
      window.removeEventListener('scroll', schedule);
      window.removeEventListener('resize', schedule);
      window.removeEventListener('wheel', userScroll);
      window.removeEventListener('touchstart', userScroll);
      if (raf) cancelAnimationFrame(raf);
    };
  }, [enabled, count, onScene, progressRef, trackRef, pausedRef]);
}

/** True while the element is on screen (the 3D loop pauses when it is not). */
export function useOnScreen(ref, enabled) {
  const [on, setOn] = useState(true);
  useEffect(() => {
    if (!enabled || !ref.current || typeof IntersectionObserver === 'undefined') return undefined;
    const io = new IntersectionObserver(([e]) => setOn(e.isIntersecting), { rootMargin: '120px' });
    io.observe(ref.current);
    return () => io.disconnect();
  }, [ref, enabled]);
  return on;
}

/** Viewport size, for the camera inset that keeps the subject clear of HUD panels. */
export function useViewport() {
  const get = () => (typeof window === 'undefined' ? [1280, 800] : [window.innerWidth, window.innerHeight]);
  const [vp, setVp] = useState(get);
  useEffect(() => {
    const on = () => setVp(get());
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return vp;
}
