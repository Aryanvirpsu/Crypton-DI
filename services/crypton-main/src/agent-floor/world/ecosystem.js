import { ECOSYSTEM, SLAB } from './choreography';
import { C } from './colors';

/**
 * The far floors revealed by the final pullback. Pure, deterministic data: where each floor
 * sits, what kind of work area it is, how busy it is, and whether it carries the illustrated
 * boundary signal. Rendered by Ecosystem3D.jsx.
 *
 * Honesty rules, enforced in ecosystem.test.js:
 *  - only a minority of floors show the boundary signal. Most are plainly at work.
 *  - the signal is an "attempted" request that stops at a boundary seam, never one that
 *    crosses it. It says "this shape can repeat", not "these are verified violations".
 *  - nothing here is labelled, and none of it is scenario evidence.
 */

/** Floors that carry the signal: 4 of 12, spread near and far. */
export const PROBLEM_FLOORS = [2, 5, 8, 11];

const TOP = 0.3; // slab top, local to a floor (slab is 0.6 thick, centred)

const box = (x, y, z, w, h, d, color) => ({ p: [x, y, z], size: [w, h, d], color });

/** Four kinds of work area, so the far floors are not copies of one another. */
const VARIANTS = [
  // 0: desks and terminals
  { furniture: [box(-4, TOP + 0.4, -1.5, 1.4, 0.8, 0.8, C.bone), box(-1.2, TOP + 0.4, -1.5, 1.4, 0.8, 0.8, C.mist), box(3, TOP + 0.4, 2, 1.6, 0.8, 0.9, C.machine)], path: { c: [-2.5, -0.2], r: [3, 2] } },
  // 1: records
  { furniture: [box(-3.2, TOP + 0.8, -3, 0.9, 1.6, 1.2, C.mist), box(-1.9, TOP + 0.8, -3, 0.9, 1.6, 1.2, C.mist), box(-0.6, TOP + 0.8, -3, 0.9, 1.6, 1.2, C.mist), box(3, TOP + 0.3, 2.5, 1.2, 0.6, 1.2, C.bone)], path: { c: [-1, -1.2], r: [2.5, 1] } },
  // 2: relay hub
  { furniture: [box(0, TOP + 0.1, 0, 2.6, 0.2, 2.6, C.mist), box(0, TOP + 0.5, 0, 0.7, 0.9, 0.7, C.machine), box(3.6, TOP + 0.3, -2.5, 1.2, 0.6, 1.2, C.mist)], path: { c: [0, 0], r: [2.7, 2.3] } },
  // 3: dock and crates
  { furniture: [box(2, TOP + 0.05, 1, 3.2, 0.1, 2, C.mist), box(-3, TOP + 0.5, -2, 1.2, 1, 1.2, C.bone), box(-1.6, TOP + 0.3, -2, 0.9, 0.6, 0.9, C.machine), box(-3, TOP + 0.3, 2.4, 1.6, 0.6, 0.8, C.mist)], path: { c: [1, 1.2], r: [3, 1.4] } },
];

export function buildFloors() {
  return ECOSYSTEM.map(([x, z, s], i) => {
    const v = VARIANTS[i % VARIANTS.length];
    const problem = PROBLEM_FLOORS.includes(i);
    // busy: how fast its workers move. A few floors are nearly idle, so activity is staggered.
    const busy = [0.9, 0.35, 0.7, 1, 0.2, 0.6, 0.85, 0.4, 0.75, 0.3, 0.95, 0.5][i % 12];
    return {
      i, x, z, s,
      y: -1 - (i % 3),
      variant: i % VARIANTS.length,
      furniture: v.furniture,
      path: v.path,
      workers: 2 + (i % 3),
      busy,
      phase: (i * 1.37) % (Math.PI * 2),
      shade: 0.9 + ((i * 29) % 11) / 55, // slab brightness, a little different on each
      problem,
    };
  });
}

/** Worker position (floor-local, on the slab) at time t. */
export function workerAt(floor, k, t) {
  const { c, r } = floor.path;
  const w = (0.25 + 0.35 * floor.busy) * (k % 2 ? -1 : 1);
  const a = floor.phase + k * 2.1 + t * w;
  return [c[0] + r[0] * Math.cos(a) * (1 - 0.18 * k), TOP + 0.3, c[1] + r[1] * Math.sin(a) * (1 - 0.18 * k)];
}

/** The attempted request on a problem floor: 0..1 along its way to the seam, holding at the end. */
export function packetProgress(floor, t) {
  const cycle = 7 + (floor.i % 3);
  const u = ((t + floor.phase * 2) % cycle) / cycle;
  return Math.min(1, u / 0.55); // travels for 55% of the cycle, then waits at the seam
}

export const SEAM_X = SLAB.width / 2 - 0.1;
