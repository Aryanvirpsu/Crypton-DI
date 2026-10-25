import * as THREE from 'three';
import { C } from './palette';
import { SLAB, BOUNDARY_X } from './choreography';

/**
 * Small generated textures: the painted floor, two screens and a soft light pool. One canvas
 * each, a few KB, no assets to load. Everything uses the six palette colours.
 */

const K = 72; // pixels per world unit on the floor
const px = (x) => (x + SLAB.width / 2) * K;
const pz = (z) => (z + SLAB.depth / 2) * K;

function rng(seed) {
  let a = seed;
  return () => {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function canvas(w, h) {
  const c = document.createElement('canvas');
  c.width = w; c.height = h;
  return [c, c.getContext('2d')];
}

function texture(c, { repeat = false } = {}) {
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 4;
  if (repeat) { t.wrapS = THREE.RepeatWrapping; t.wrapT = THREE.RepeatWrapping; }
  return t;
}

const rgb = (hex, a = 1) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
};

function brackets(ctx, cx, cz, w, d, len, color, alpha) {
  ctx.strokeStyle = rgb(color, alpha);
  ctx.lineWidth = 3;
  const x0 = px(cx - w / 2); const x1 = px(cx + w / 2);
  const z0 = pz(cz - d / 2); const z1 = pz(cz + d / 2);
  const l = len * K;
  [[x0, z0, 1, 1], [x1, z0, -1, 1], [x0, z1, 1, -1], [x1, z1, -1, -1]].forEach(([x, z, sx, sz]) => {
    ctx.beginPath(); ctx.moveTo(x + sx * l, z); ctx.lineTo(x, z); ctx.lineTo(x, z + sz * l); ctx.stroke();
  });
}

function lane(ctx, pts, color, alpha) {
  ctx.setLineDash([12, 12]);
  ctx.strokeStyle = rgb(color, alpha);
  ctx.lineWidth = 3;
  ctx.beginPath();
  pts.forEach(([x, z], i) => (i ? ctx.lineTo(px(x), pz(z)) : ctx.moveTo(px(x), pz(z))));
  ctx.stroke();
  ctx.setLineDash([]);
}

function label(ctx, text, x, z, color, alpha, size = 20) {
  ctx.font = `600 ${size}px "DM Mono", ui-monospace, Consolas, monospace`;
  ctx.fillStyle = rgb(color, alpha);
  ctx.textAlign = 'center';
  ctx.fillText(text.split('').join(' '), px(x), pz(z));
}

/** The painted floor: panel variation, worn walkways, station footprints, and the boundary line. */
export function makeFloorTexture() {
  const [c, ctx] = canvas(SLAB.width * K, SLAB.depth * K);
  const r = rng(7);
  // panels: each one-unit tile a hair lighter or darker, so the floor reads as laid plates
  for (let x = 0; x < SLAB.width; x += 1) {
    for (let z = 0; z < SLAB.depth; z += 1) {
      const v = 0.88 + r() * 0.22;
      ctx.fillStyle = `rgb(${Math.round(39 * v)},${Math.round(49 * v)},${Math.round(42 * v)})`;
      ctx.fillRect(x * K, z * K, K, K);
    }
  }
  // fine grain
  for (let i = 0; i < 5200; i += 1) {
    ctx.fillStyle = r() > 0.5 ? 'rgba(255,255,255,0.025)' : 'rgba(0,0,0,0.05)';
    ctx.fillRect(r() * c.width, r() * c.height, 1 + r() * 2, 1 + r() * 2);
  }
  // worn walkways between stations
  const paths = [
    [[-4.7, 0.4], [-3.4, -0.35]],
    [[-3.0, -0.2], [-0.6, 0.6]],
    [[1.1, 0.7], [3.8, 1.6]],
    [[0.3, -0.4], [1.5, -1.8]],
    [[-5.0, 2.0], [-4.7, 0.6]],
  ];
  ctx.lineCap = 'round';
  ctx.strokeStyle = 'rgba(0,0,0,0.13)';
  ctx.lineWidth = 0.55 * K;
  paths.forEach((pts) => { ctx.beginPath(); pts.forEach(([x, z], i) => (i ? ctx.lineTo(px(x), pz(z)) : ctx.moveTo(px(x), pz(z)))); ctx.stroke(); });
  ctx.lineCap = 'butt';
  paths.forEach((pts) => lane(ctx, pts, C.mist, 0.2));

  // station footprints: the designated report area in lime, the records area in plain mist
  brackets(ctx, -3.4, -1.0, 2.6, 2.4, 0.38, C.lime, 0.4);
  label(ctx, 'REPORT', -3.4, -2.35, C.lime, 0.32);
  brackets(ctx, 1.7, -2.9, 2.5, 1.6, 0.34, C.mist, 0.26);
  label(ctx, 'RECORDS', 1.7, -1.9, C.mist, 0.26);
  ctx.strokeStyle = rgb(C.mist, 0.2); ctx.lineWidth = 3;
  ctx.beginPath(); ctx.arc(px(0.3), pz(0.6), 1.2 * K, 0, Math.PI * 2); ctx.stroke();
  label(ctx, 'RELAY', 0.3, 2.15, C.mist, 0.26);
  brackets(ctx, -5.2, 2.9, 2.4, 1.8, 0.3, C.bone, 0.22);
  label(ctx, 'MANAGER', -5.2, 4.05, C.mist, 0.22, 17);
  label(ctx, 'BAY 1', 5.2, 0.5, C.mist, 0.28);
  label(ctx, 'FLOOR A01', -5.3, -3.85, C.mist, 0.2, 16);

  // where the organisation ends: a dashed coral line a little inside the seam
  lane(ctx, [[BOUNDARY_X - 0.55, -SLAB.depth / 2 + 0.2], [BOUNDARY_X - 0.55, SLAB.depth / 2 - 0.2]], C.coral, 0.4);

  // panel seams
  ctx.strokeStyle = rgb(C.mist, 0.1); ctx.lineWidth = 1;
  for (let x = 1; x < SLAB.width; x += 1) { ctx.beginPath(); ctx.moveTo(x * K, 0); ctx.lineTo(x * K, c.height); ctx.stroke(); }
  for (let z = 1; z < SLAB.depth; z += 1) { ctx.beginPath(); ctx.moveTo(0, z * K); ctx.lineTo(c.width, z * K); ctx.stroke(); }
  return texture(c);
}

/** The manager's monitor: a task list with one job selected. */
export function makeDeskScreen() {
  const [c, ctx] = canvas(256, 160);
  ctx.fillStyle = rgb(C.graphite); ctx.fillRect(0, 0, 256, 160);
  ctx.fillStyle = rgb(C.bone, 0.9); ctx.fillRect(14, 14, 90, 8);
  ctx.fillStyle = rgb(C.mist, 0.4); ctx.fillRect(14, 32, 228, 1);
  const rows = [150, 120, 170, 96];
  rows.forEach((w, i) => {
    const y = 44 + i * 26;
    if (i === 1) { ctx.fillStyle = rgb(C.lime, 0.2); ctx.fillRect(10, y - 6, 236, 22); }
    ctx.fillStyle = rgb(i === 1 ? C.lime : C.mist, i === 1 ? 0.95 : 0.55);
    ctx.fillRect(18, y, 10, 10);
    ctx.fillRect(38, y + 2, w, 6);
  });
  return texture(c);
}

/** The report terminal: scrolling lines of text, one block highlighted. Repeats vertically. */
export function makeTerminalScreen() {
  const [c, ctx] = canvas(256, 256);
  const r = rng(11);
  ctx.fillStyle = rgb(C.graphite); ctx.fillRect(0, 0, 256, 256);
  let y = 10;
  while (y < 250) {
    const w = 40 + r() * 170;
    const head = r() > 0.82;
    ctx.fillStyle = rgb(C.lime, head ? 0.95 : 0.5);
    ctx.fillRect(16, y, head ? Math.min(w, 110) : w, head ? 7 : 5);
    y += head ? 16 : 11;
    if (r() > 0.8) y += 8;
  }
  ctx.fillStyle = rgb(C.lime, 0.12); ctx.fillRect(10, 120, 236, 26);
  return texture(c, { repeat: true });
}

/** A soft round light pool: white in the middle, clear at the edge. */
export function makePoolTexture() {
  const [c, ctx] = canvas(128, 128);
  const g = ctx.createRadialGradient(64, 64, 0, 64, 64, 64);
  g.addColorStop(0, 'rgba(255,255,255,1)');
  g.addColorStop(0.45, 'rgba(255,255,255,0.45)');
  g.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = g; ctx.fillRect(0, 0, 128, 128);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}
