import * as THREE from 'three';
import { C } from './palette';
import { BOUNDARY_X, SLAB, STATIONS } from './choreography';

/**
 * Everything on the main floor that never moves, as declarative parts (see merge.jsx).
 * Positions come from STATIONS, so robots, labels and scenery agree. Merged by material, the
 * whole set is a few draw calls. Animated and lit parts (screens, lamps, the hub ring) live
 * in Floor3D.jsx.
 */

const TOP = SLAB.height;
const MACHINE = [C.machine, 'metal'];
const MIST = [C.mist, 'metal'];
const BONE = [C.bone, 'gloss'];
const BONE_MATTE = [C.bone, 'matte'];
const GRAPHITE = [C.graphite, 'gloss'];
const GRAPHITE_MATTE = [C.graphite, 'matte'];
const LIME = [C.lime, 'glow'];
const CORAL = [C.coral, 'glow'];

const stationPos = (id, y = TOP) => [STATIONS[id].at[0], y, STATIONS[id].at[1]];

// ── manager's desk ───────────────────────────────────────────────────────────────────────
export const DESK_BASE = { pos: stationPos('PR-MGR'), yaw: Math.PI * 0.2 };
const desk = [
  { g: 'rbox', a: [1.6, 0.08, 0.8, 0.03], p: [0, 0.62, 0], m: BONE },
  ...[[-0.7, -0.32], [0.7, -0.32], [-0.7, 0.32], [0.7, 0.32]].map(([x, z]) => ({ g: 'box', a: [0.06, 0.6, 0.06], p: [x, 0.3, z], m: MACHINE })),
  { g: 'box', a: [0.72, 0.44, 0.04], p: [0, 0.94, -0.22], r: [-0.12, 0, 0], m: GRAPHITE },
  { g: 'cyl', a: [0.04, 0.05, 0.16, 8], p: [0, 0.72, -0.22], m: GRAPHITE },
  { g: 'box', a: [0.5, 0.02, 0.18], p: [0, 0.66, 0.1], m: GRAPHITE },
  { g: 'rbox', a: [0.5, 0.42, 0.62, 0.03], p: [0.42, 0.32, 0], m: MACHINE },
  { g: 'box', a: [0.2, 0.025, 0.03], p: [0.42, 0.4, 0.32], m: MIST },
  // stool
  { g: 'cyl', a: [0.2, 0.2, 0.05, 16], p: [0, 0.46, 0.78], m: GRAPHITE },
  { g: 'cyl', a: [0.04, 0.05, 0.4, 8], p: [0, 0.22, 0.78], m: MACHINE },
  { g: 'cyl', a: [0.22, 0.24, 0.04, 16], p: [0, 0.02, 0.78], m: MACHINE },
  // paper trays
  { g: 'box', a: [0.3, 0.025, 0.2], p: [-0.55, 0.665, 0.05], m: BONE_MATTE },
  { g: 'box', a: [0.28, 0.025, 0.19], p: [-0.55, 0.69, 0.05], r: [0, 0.15, 0], m: MIST },
  // desk lamp
  { g: 'cyl', a: [0.06, 0.07, 0.03, 12], p: [0.62, 0.665, -0.2], m: GRAPHITE },
  { g: 'box', a: [0.02, 0.28, 0.02], p: [0.62, 0.8, -0.2], r: [0, 0, 0.2], m: GRAPHITE },
  { g: 'box', a: [0.02, 0.22, 0.02], p: [0.55, 0.97, -0.2], r: [0, 0, -0.9], m: GRAPHITE },
  { g: 'cyl', a: [0.02, 0.07, 0.1, 12], p: [0.43, 0.97, -0.2], r: [0, 0, 1.2], m: BONE },
];

// ── report terminal ──────────────────────────────────────────────────────────────────────
export const TERMINAL_BASE = { pos: stationPos('RS-REPORT') };
const terminal = [
  { g: 'box', a: [0.96, 0.06, 0.56], p: [0, 0.03, 0], m: GRAPHITE },
  { g: 'rbox', a: [0.9, 0.74, 0.5, 0.05], p: [0, 0.43, 0], m: MACHINE },
  { g: 'box', a: [0.86, 0.5, 0.05], p: [0, 0.95, 0.05], r: [-0.55, 0, 0], m: GRAPHITE },
  { g: 'box', a: [0.3, 0.2, 0.2], p: [0, 0.85, -0.1], m: MACHINE },
  ...[0, 1, 2, 3].map((i) => ({ g: 'box', a: [0.01, 0.03, 0.3], p: [0.455, 0.2 + i * 0.12, 0], m: GRAPHITE })),
  { g: 'box', a: [0.12, 1.1, 0.12], p: [0.62, 0.55, -0.12], m: MACHINE },
  ...[0, 1, 2].map((i) => ({ g: 'box', a: [0.06, 0.03, 0.01], p: [0.62, 0.85 + i * 0.08, -0.055], m: MIST })),
  { g: 'box', a: [0.34, 0.03, 0.42], p: [-0.62, 0.2, 0.1], m: MIST },
  ...[0, 1, 2].map((i) => ({ g: 'box', a: [0.28, 0.02, 0.36], p: [-0.62, 0.23 + i * 0.025, 0.1], r: [0, i * 0.08, 0], m: BONE_MATTE })),
];

// ── records cabinets ─────────────────────────────────────────────────────────────────────
export const CABINET_BASE = { pos: stationPos('RS-RECORDS') };
const cabinets = [-0.55, 0, 0.55].flatMap((x) => [
  { g: 'rbox', a: [0.5, 1.6, 0.7, 0.04], p: [x, 0.8, 0], m: MIST },
  { g: 'box', a: [0.46, 0.06, 0.66], p: [x, 0.03, 0], m: GRAPHITE },
  { g: 'box', a: [0.4, 0.02, 0.5], p: [x, 1.61, 0], m: GRAPHITE },
  ...[0.25, 0.6, 0.95, 1.3].flatMap((y) => [
    { g: 'box', a: [0.4, 0.02, 0.01], p: [x, y, 0.352], m: GRAPHITE_MATTE },
    { g: 'box', a: [0.14, 0.035, 0.03], p: [x, y + 0.13, 0.36], m: MACHINE },
    { g: 'box', a: [0.14, 0.05, 0.012], p: [x, y + 0.055, 0.358], m: BONE_MATTE },
  ]),
]);

// ── relay hub (the static part of it) ────────────────────────────────────────────────────
export const HUB_BASE = { pos: stationPos('hub') };
const hub = [
  { g: 'cyl', a: [0.95, 1.0, 0.04, 48], p: [0, 0.02, 0], m: GRAPHITE },
  { g: 'torus', a: [0.62, 0.014, 6, 48], p: [0, 0.045, 0], r: [Math.PI / 2, 0, 0], m: MIST },
  { g: 'cyl', a: [0.2, 0.22, 0.03, 24], p: [0, 0.055, 0], m: MACHINE },
];

// ── dock and charging bay ────────────────────────────────────────────────────────────────
const [DX, DZ] = STATIONS.dock.at;
const stripes = Array.from({ length: 9 }, (_, i) => ({
  g: 'plane', a: [0.12, 0.42], p: [DX - 1.0 + i * 0.25 + 0.6, TOP + 0.006, DZ + 0.95], r: [-Math.PI / 2, 0, 0.6], m: i % 2 ? LIME : GRAPHITE_MATTE,
}));
export const DOCK_BASE = {};
const dock = [
  { g: 'plane', a: [2.6, 1.7], p: [DX + 0.6, TOP + 0.004, DZ], r: [-Math.PI / 2, 0, 0], m: GRAPHITE_MATTE },
  { g: 'box', a: [1.15, 0.03, 0.85], p: [DX, TOP + 0.02, DZ], m: MACHINE },
  { g: 'box', a: [0.9, 0.012, 0.6], p: [DX, TOP + 0.04, DZ], m: GRAPHITE_MATTE },
  { g: 'box', a: [0.1, 0.4, 0.1], p: [DX + 1.9, TOP + 0.2, DZ - 0.85], m: GRAPHITE },
  { g: 'box', a: [0.1, 0.4, 0.1], p: [DX + 1.9, TOP + 0.2, DZ + 0.85], m: GRAPHITE },
  ...stripes,
];

// ── the boundary: gate frame and bollards ────────────────────────────────────────────────
const GZ = STATIONS.gate.at[1];
export const GATE_BASE = { pos: [BOUNDARY_X - 0.12, TOP, GZ] };
const gate = [
  ...[-0.85, 0.85].map((z) => ({ g: 'box', a: [0.14, 1.7, 0.14], p: [0, 0.85, z], m: MACHINE })),
  { g: 'box', a: [0.18, 0.14, 1.84], p: [0, 1.72, 0], m: MACHINE },
  { g: 'box', a: [0.16, 0.03, 1.5], p: [0.02, 1.62, 0], m: CORAL },
];
const bollards = [-3.8, -2.6, -1.4, -0.2, 3.5, 4.1].flatMap((z) => [
  { g: 'cyl', a: [0.07, 0.07, 0.5, 10], p: [BOUNDARY_X - 0.55, TOP + 0.25, z], m: GRAPHITE },
  { g: 'cyl', a: [0.074, 0.074, 0.07, 10], p: [BOUNDARY_X - 0.55, TOP + 0.46, z], m: BONE },
]);

// ── props ────────────────────────────────────────────────────────────────────────────────
const cable = (pts) => new THREE.TubeGeometry(new THREE.CatmullRomCurve3(pts.map(([x, z]) => new THREE.Vector3(x, TOP + 0.04, z))), 28, 0.035, 6, false);
const props = [
  // crates behind the dock, with straps
  { g: 'rbox', a: [0.5, 0.5, 0.5, 0.04], p: [3.6, TOP + 0.25, -1.2], m: MACHINE },
  { g: 'rbox', a: [0.5, 0.5, 0.5, 0.04], p: [4.15, TOP + 0.25, -1.2], m: MACHINE },
  { g: 'rbox', a: [0.5, 0.5, 0.5, 0.04], p: [3.88, TOP + 0.75, -1.2], m: BONE_MATTE },
  { g: 'box', a: [0.52, 0.52, 0.06], p: [3.6, TOP + 0.25, -1.2], m: GRAPHITE_MATTE },
  { g: 'box', a: [0.52, 0.52, 0.06], p: [4.15, TOP + 0.25, -1.2], m: GRAPHITE_MATTE },
  // barrels and a tool cart, back left
  { g: 'cyl', a: [0.22, 0.22, 0.6, 14], p: [-6.3, TOP + 0.3, -3.6], m: GRAPHITE },
  { g: 'cyl', a: [0.225, 0.225, 0.05, 14], p: [-6.3, TOP + 0.38, -3.6], m: MIST },
  { g: 'cyl', a: [0.22, 0.22, 0.6, 14], p: [-5.75, TOP + 0.3, -3.7], m: GRAPHITE },
  { g: 'cyl', a: [0.225, 0.225, 0.05, 14], p: [-5.75, TOP + 0.38, -3.7], m: MIST },
  { g: 'box', a: [0.8, 0.05, 0.5], p: [-6.1, TOP + 0.5, -2.3], m: MACHINE },
  { g: 'box', a: [0.8, 0.04, 0.5], p: [-6.1, TOP + 0.22, -2.3], m: GRAPHITE },
  ...[[-0.38, -0.22], [0.38, -0.22], [-0.38, 0.22], [0.38, 0.22]].map(([x, z]) => ({ g: 'box', a: [0.04, 0.3, 0.04], p: [-6.1 + x, TOP + 0.37, -2.3 + z], m: GRAPHITE })),
  ...[[-0.38, -0.22], [0.38, -0.22], [-0.38, 0.22], [0.38, 0.22]].map(([x, z]) => ({ g: 'cyl', a: [0.05, 0.05, 0.04, 10], p: [-6.1 + x, TOP + 0.05, -2.3 + z], r: [0, 0, Math.PI / 2], m: GRAPHITE })),
  { g: 'box', a: [0.04, 0.4, 0.04], p: [-6.48, TOP + 0.72, -2.3], m: MIST },
  // pipe run along the back, with brackets and a valve
  { g: 'cyl', a: [0.06, 0.06, 13, 12], p: [0, TOP + 1.15, -4.3], r: [0, 0, Math.PI / 2], m: MACHINE },
  ...[-5, -2.5, 0, 2.5, 5].map((x) => ({ g: 'box', a: [0.05, 1.15, 0.05], p: [x, TOP + 0.575, -4.3], m: GRAPHITE })),
  { g: 'torus', a: [0.1, 0.018, 8, 18], p: [2.0, TOP + 1.15, -4.18], m: MIST },
  { g: 'cyl', a: [0.014, 0.014, 0.14, 6], p: [2.0, TOP + 1.15, -4.24], r: [Math.PI / 2, 0, 0], m: MIST },
  // cable tray and a low guard rail on the back and left edges
  { g: 'box', a: [11, 0.1, 0.3], p: [-1, TOP + 0.05, -SLAB.depth / 2 + 0.25], m: GRAPHITE_MATTE },
  ...Array.from({ length: 14 }, (_, i) => ({ g: 'box', a: [0.05, 0.44, 0.05], p: [-6.5 + i, TOP + 0.22, -SLAB.depth / 2 + 0.08], m: GRAPHITE_MATTE })),
  { g: 'box', a: [13.4, 0.05, 0.05], p: [0, TOP + 0.44, -SLAB.depth / 2 + 0.08], m: MACHINE },
  ...Array.from({ length: 9 }, (_, i) => ({ g: 'box', a: [0.05, 0.44, 0.05], p: [-SLAB.width / 2 + 0.08, TOP + 0.22, -4 + i], m: GRAPHITE_MATTE })),
  { g: 'box', a: [0.05, 0.05, 8.4], p: [-SLAB.width / 2 + 0.08, TOP + 0.44, 0], m: MACHINE },
  // floor cables between stations
  { g: 'geo', a: [cable([[1.5, -2.5], [1.3, -1.6], [0.7, -0.6], [0.3, -0.05]])], m: GRAPHITE_MATTE },
  { g: 'geo', a: [cable([[-3.9, -1.8], [-4.6, -0.6], [-4.9, 0.9], [-5.1, 1.9]])], m: GRAPHITE_MATTE },
  { g: 'geo', a: [cable([[3.9, 1.7], [2.7, 1.4], [1.4, 0.9]])], m: GRAPHITE_MATTE },
  ...[[1.3, -1.6], [-4.6, -0.6], [2.7, 1.4]].map(([x, z]) => ({ g: 'box', a: [0.12, 0.03, 0.06], p: [x, TOP + 0.02, z], m: MACHINE })),
  // floor lamps
  ...[[-1.8, 3.6], [2.4, 3.8], [-6.2, -1.8]].flatMap(([x, z]) => [
    { g: 'cyl', a: [0.025, 0.04, 0.7, 8], p: [x, TOP + 0.35, z], m: GRAPHITE_MATTE },
    { g: 'sphere', a: [0.055, 12, 8], p: [x, TOP + 0.72, z], m: BONE },
  ]),
];

export const SCENERY_GROUPS = [
  { parts: desk, base: DESK_BASE },
  { parts: terminal, base: TERMINAL_BASE },
  { parts: cabinets, base: CABINET_BASE },
  { parts: hub, base: HUB_BASE },
  { parts: dock, base: DOCK_BASE },
  { parts: gate, base: GATE_BASE },
  { parts: bollards, base: {} },
  { parts: props, base: {} },
];
