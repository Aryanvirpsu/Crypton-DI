/**
 * Blocking for the 3D Agent Floor: where things stand, where robots go at each scene,
 * where the camera looks, and the shape of each authority envelope.
 *
 * Presentation only, and pure data, so it can be tested. It never decides what is true:
 * which grants, delegations and actions exist at a scene comes from scenario.js through
 * authorityGraph.js. This file only says where to draw them.
 *
 * World units: the floor slab is 14 x 9, centred on the origin. x runs right, z runs
 * toward the viewer, y is up. Points on the floor are [x, z].
 */

export const SLAB = { width: 14, depth: 9, height: 0.6 };

/** The organisation's edge. Everything with x beyond this is outside. */
export const BOUNDARY_X = SLAB.width / 2;

export const STATIONS = {
  'PR-MGR': { at: [-5.2, 2.9], kind: 'desk' },
  'RS-REPORT': { at: [-3.4, -1.7], kind: 'terminal' },
  'RS-RECORDS': { at: [1.7, -3.3], kind: 'cabinet' },
  'RS-EXTERNAL': { at: [10.6, 1.4], kind: 'pod' },
  hub: { at: [0.3, 0.6], kind: 'hub' },
  dock: { at: [4.6, 1.8], kind: 'dock' },
  gate: { at: [BOUNDARY_X, 1.8], kind: 'gate' },
};

/**
 * Robot blocking per scene. `at` is where the robot stands, `look` the floor point it
 * faces, `pose` the animation it holds there.
 */
const SCOUT_HOME = { at: [-4.7, 0.4], look: [-5.2, 2.9], pose: 'idle' };
const SCOUT_AT_REPORT = { at: [-3.4, -0.75], look: [-3.4, -1.7], pose: 'scan' };
const SCOUT_AT_RELAY = { at: [-1.25, 0.55], look: [0.3, 0.6], pose: 'idle' };
const RELAY_HOME = { at: [0.3, 0.6], look: [0.3, 4], pose: 'idle' };
const RELAY_RECEIVE = { at: [0.3, 0.6], look: [-1.25, 0.55], pose: 'receive' };
const RELAY_SEND = { at: [0.3, 0.6], look: [4.6, 1.8], pose: 'send' };
const COURIER_DOCKED = { at: [4.6, 1.8], look: [0.3, 0.6], pose: 'idle' };
// The request reaches Courier and the frame holds as it turns toward the gate. It has
// not left the dock: the scenario records the request, not what happened next.
const COURIER_TURNING = { at: [4.95, 1.8], look: [BOUNDARY_X + 2, 1.8], pose: 'roll' };

export const BLOCKING = [
  /* S00 arrival    */ { 'AG-SCOUT': SCOUT_HOME, 'AG-RELAY': RELAY_HOME, 'AG-COURIER': COURIER_DOCKED },
  /* S01 assignment */ { 'AG-SCOUT': { ...SCOUT_HOME, pose: 'listen' }, 'AG-RELAY': RELAY_HOME, 'AG-COURIER': COURIER_DOCKED },
  /* S02 execution  */ { 'AG-SCOUT': SCOUT_AT_REPORT, 'AG-RELAY': RELAY_HOME, 'AG-COURIER': COURIER_DOCKED },
  /* S03 delegation */ { 'AG-SCOUT': SCOUT_AT_RELAY, 'AG-RELAY': RELAY_RECEIVE, 'AG-COURIER': COURIER_DOCKED },
  /* S04 boundary   */ { 'AG-SCOUT': SCOUT_AT_RELAY, 'AG-RELAY': RELAY_SEND, 'AG-COURIER': COURIER_TURNING },
  /* S05 atlas      */ { 'AG-SCOUT': SCOUT_AT_RELAY, 'AG-RELAY': RELAY_SEND, 'AG-COURIER': COURIER_TURNING },
  /* S06 evidence   */ { 'AG-SCOUT': SCOUT_AT_RELAY, 'AG-RELAY': RELAY_SEND, 'AG-COURIER': COURIER_TURNING },
  /* S07 resolution */ { 'AG-SCOUT': SCOUT_AT_RELAY, 'AG-RELAY': RELAY_SEND, 'AG-COURIER': COURIER_TURNING },
];

/**
 * Handoff timing, in real seconds after a scene starts. The travelling token (Authority3D)
 * and each robot's reaction (Robots3D) both read these, so a give and its receive can never
 * drift apart. Scene 4 holds at 1.9 s (see HOLD_AT in World3D), so its receive lands before.
 *   1  the task card: Manager's desk to Scout
 *   3  the delegation token: Scout to Relay
 *   4  the export request: Relay to Courier
 */
export const CUES = {
  1: { give: 0.4, receive: 1.9 },
  3: { give: 1.6, receive: 2.9 },
  4: { give: 0.3, receive: 1.4 },
};

/** 0 -> 1 -> 0 over `width` seconds starting at the cue; 0 outside it and when there is no cue. */
export function cueBump(step, key, t, width = 0.6) {
  const c = CUES[step];
  if (!c) return 0;
  const d = t - c[key];
  if (d < 0 || d > width) return 0;
  return Math.sin((d / width) * Math.PI);
}

/** True once the cue at scene `at` has happened: later scenes always, its own scene after the time. */
export function cueState(step, at, key, t) {
  if (step > at) return true;
  return step === at && t >= CUES[at][key];
}

/** The scene at which the world's clock stops: the request is made and the frame holds. */
export const FREEZE_STEP = 4;

/**
 * Camera shots. `pos` and `target` are [x, y, z]. `follow` makes both relative to a
 * robot's live position, so the camera moves with it. The HUD offsets the projection
 * (see `inset` in World3D) so subjects stay clear of the panels; targets are the subject.
 */
export const SHOTS = [
  /* S00 establishing  */ { pos: [16.5, 20.5, 30], target: [2.6, -0.4, 0.6], fov: 30 },
  /* S01 desk + Scout  */ { pos: [3.2, 7.4, 12.8], target: [-4.3, 0.6, 0.7], fov: 30 },
  /* S02 follow Scout  */ { follow: 'AG-SCOUT', pos: [7.0, 5.8, 5.6], target: [-0.2, 0.6, -0.5], fov: 30 },
  /* S03 Scout + Relay */ { pos: [5.0, 6.4, 11.0], target: [-0.6, 0.8, 0.4], fov: 30 },
  /* S04 the boundary  */ { pos: [13.5, 13, 27], target: [6.5, 0.2, 1.0], fov: 32 },
  /* S05 atlas         */ { pos: [2.6, 36, 15], target: [2.4, 0, 0.4], fov: 30 },
  /* S06 evidence      */ { pos: [8.6, 12, 18], target: [4.2, 0.4, 0.8], fov: 30 },
  /* S07 pullback      */ { pos: [34, 44, 62], target: [0, -2, 0], fov: 30 },
];

/** Floor-view shot for the late scenes, which default to the Atlas. */
export const LATE_FLOOR_SHOT = SHOTS[4];

/** Shot used when the visitor switches to the Atlas view during a floor scene. */
export const ATLAS_SHOT = SHOTS[5];

/**
 * The shot for a scene in a view. Each scene has its own shot in its default view;
 * switching to the other view uses the Atlas overview, or the boundary floor shot.
 */
export function shotFor(step, view, defaultView) {
  if (view === defaultView) return SHOTS[step];
  return view === 'atlas' ? ATLAS_SHOT : LATE_FLOOR_SHOT;
}

/**
 * Authority envelopes: the region of the floor a grant or delegation covers. They are
 * drawn only when the grant or delegation is visible in the graph. Courier has none.
 * `center` and `size` are [x, z]; `height` is the envelope's height.
 */
export const ENVELOPES = {
  'GR-001': { holder: 'AG-SCOUT', center: [-3.4, -0.55], size: [3.6, 3.6], height: 2.2 },
  'DL-001': { holder: 'AG-RELAY', center: [0.3, 0.6], size: [1.9, 1.9], height: 1.7 },
};

export function insideEnvelope(id, [x, z]) {
  const e = ENVELOPES[id];
  return Math.abs(x - e.center[0]) <= e.size[0] / 2 && Math.abs(z - e.center[1]) <= e.size[1] / 2;
}

/** Where a node sits at a scene, on the floor. Robots move; stations do not. */
export function nodeAt(id, step) {
  const b = BLOCKING[Math.max(0, Math.min(BLOCKING.length - 1, step))];
  if (b[id]) return b[id].at;
  return STATIONS[id] ? STATIONS[id].at : null;
}

/** Other floors revealed by the final pullback. Unlabelled and purely decorative. */
export const ECOSYSTEM = [
  [-22, -6, 0.7], [-19, 12, 0.6], [-4, -20, 0.8], [14, -18, 0.6], [26, -4, 0.7],
  [20, 14, 0.55], [-8, 20, 0.65], [6, 26, 0.5], [-30, 4, 0.5], [32, 10, 0.5],
  [-14, -26, 0.45], [28, -22, 0.45],
];
