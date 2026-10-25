import { BLOCKING, SLAB, STATIONS, shotFor } from './choreography';

/**
 * Scene-aware camera framing for small screens.
 *
 * On a phone the desktop shots crop the subject, so the camera is fitted instead of scaled:
 * for each scene we list the world points that must be visible (the cast, their name tags and
 * the places they relate to), then find the closest camera distance along the shot's own
 * direction at which every one of them projects inside the safe rectangle, and the screen
 * shift that centres them in it. Pure data and maths, so it is testable without a browser.
 * Desktop never calls this.
 */

// Heights of the name tags above the floor; kept in step with world/Labels3D.jsx.
const STATION_LIFT = { 'PR-MGR': 1.5, 'RS-REPORT': 1.55, 'RS-RECORDS': 2.0, 'RS-EXTERNAL': 1.45 };
const AGENT_HEIGHT = { 'AG-SCOUT': 1.15, 'AG-RELAY': 1.35, 'AG-COURIER': 1.0 };

const stationTag = (id) => {
  const [x, z] = STATIONS[id].at;
  return [x, (id === 'RS-EXTERNAL' ? 0 : SLAB.height) + STATION_LIFT[id], z];
};
const stationFoot = (id) => {
  const [x, z] = STATIONS[id].at;
  return [x, id === 'RS-EXTERNAL' ? 0 : SLAB.height, z];
};
const agentTag = (id, step) => {
  const [x, z] = BLOCKING[step][id].at;
  return [x, SLAB.height + AGENT_HEIGHT[id] + 0.6, z];
};
const agentFoot = (id, step) => {
  const [x, z] = BLOCKING[step][id].at;
  return [x, SLAB.height, z];
};
const withFoot = (tag, foot) => [tag, foot];
const station = (id) => withFoot(stationTag(id), stationFoot(id));
const agent = (id, step) => withFoot(agentTag(id, step), agentFoot(id, step));

/** Everything the opening and the Atlas show: the whole cast, every place and the gate. */
const EVERYTHING = (step) => [
  ...station('PR-MGR'), ...station('RS-REPORT'), ...station('RS-RECORDS'), ...station('RS-EXTERNAL'),
  ...agent('AG-SCOUT', step), ...agent('AG-RELAY', step), ...agent('AG-COURIER', step),
];

/** Points that must be on screen for each scene of the default floor view. */
export const FRAMING = [
  /* S00 establishing  */ () => EVERYTHING(0),
  /* S01 desk + Scout  */ () => [...station('PR-MGR'), ...agent('AG-SCOUT', 1), ...agent('AG-RELAY', 1)],
  /* S02 follow Scout  */ () => [...station('RS-REPORT'), ...agent('AG-SCOUT', 2), ...station('RS-RECORDS')],
  /* S03 Scout + Relay */ () => [...agent('AG-SCOUT', 3), ...agent('AG-RELAY', 3)],
  /* S04 the boundary  */ () => [...agent('AG-SCOUT', 4), ...agent('AG-RELAY', 4), ...agent('AG-COURIER', 4), ...station('RS-EXTERNAL'), [STATIONS.gate.at[0], SLAB.height + 1.4, STATIONS.gate.at[1]]],
  /* S05 atlas         */ () => EVERYTHING(5),
  /* S06 evidence      */ () => [...agent('AG-SCOUT', 6), ...agent('AG-RELAY', 6), ...agent('AG-COURIER', 6), ...station('RS-EXTERNAL'), [STATIONS.gate.at[0], SLAB.height + 1.4, STATIONS.gate.at[1]]],
  /* S07 pullback      */ null,
];

/**
 * The points to fit for a scene, or null when the scene keeps its own composition (the final
 * pullback). A scene shown in its non-default view uses the Atlas overview.
 */
export function framingPoints(step, view, defaultView) {
  if (view !== defaultView) return EVERYTHING(Math.min(step, BLOCKING.length - 1));
  const f = FRAMING[step];
  return f ? f() : null;
}

/** Where a follow-shot's anchor robot stands once the scene has settled. */
function followBase(shot, step) {
  if (!shot.follow) return [0, 0, 0];
  const [x, z] = BLOCKING[step][shot.follow].at;
  return [x, SLAB.height, z];
}

// Small vector helpers: this module is plain maths so it runs (and is tested) without WebGL or three.
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const norm = (a) => { const l = Math.hypot(a[0], a[1], a[2]) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };

/**
 * Screen-space bounds, in px, of the points seen by a camera `s` times the shot's distance from
 * its target, looking at the target with y up (the same projection three.js uses: vertical fov).
 */
function bounds(points, target, dir, s, fov, width, height) {
  const eye = [target[0] + dir[0] * s, target[1] + dir[1] * s, target[2] + dir[2] * s];
  const f = norm(sub(target, eye));
  const r = norm(cross(f, [0, 1, 0]));
  const u = cross(r, f);
  const tanHalf = Math.tan((fov * Math.PI) / 360);
  const aspect = width / height;
  let minX = Infinity; let maxX = -Infinity; let minY = Infinity; let maxY = -Infinity;
  let behind = false;
  for (const p of points) {
    const d = sub(p, eye);
    const z = dot(d, f);
    if (z <= 0.01) { behind = true; continue; }
    const nx = dot(d, r) / (z * tanHalf * aspect);
    const ny = dot(d, u) / (z * tanHalf);
    const x = (nx * 0.5 + 0.5) * width;
    const y = (1 - (ny * 0.5 + 0.5)) * height;
    if (x < minX) minX = x;
    if (x > maxX) maxX = x;
    if (y < minY) minY = y;
    if (y > maxY) maxY = y;
  }
  return { minX, maxX, minY, maxY, behind };
}

/**
 * The closest distance scale `scale` (1 = the shot's own distance) at which every point sits
 * inside the safe rectangle, and the screen `shift` in pixels that centres them in it.
 *
 * width/height: the canvas in px. safe: { top, bottom, left?, right? } in px, the part of the
 * stage the HUD leaves free (left/right default to the full width). padX: horizontal margin in
 * px (half a name tag plus breathing room).
 */
export function fitFrame({ points, step, view, defaultView, width, height, safe, padX = 44, padY = 18, min = 0.9, max = 3.4 }) {
  const base = shotFor(step, view, defaultView);
  const follow = base.follow ? followBase(base, step) : [0, 0, 0];
  const target = [base.target[0] + follow[0], base.target[1] + follow[1], base.target[2] + follow[2]];
  const dir = sub([base.pos[0] + follow[0], base.pos[1] + follow[1], base.pos[2] + follow[2]], target);

  const left = safe.left || 0;
  const right = safe.right || width;
  const availW = Math.max(40, right - left - 2 * padX);
  const availH = Math.max(40, safe.bottom - safe.top - 2 * padY);
  const fits = (s) => {
    const b = bounds(points, target, dir, s, base.fov, width, height);
    return !b.behind && b.maxX - b.minX <= availW && b.maxY - b.minY <= availH;
  };

  let lo = min;
  let hi = max;
  let scale = hi;
  if (fits(lo)) scale = lo;
  else if (fits(hi)) {
    for (let i = 0; i < 28; i += 1) {
      const mid = (lo + hi) / 2;
      if (fits(mid)) hi = mid; else lo = mid;
    }
    scale = hi;
  }
  const b = bounds(points, target, dir, scale, base.fov, width, height);
  return {
    scale,
    shift: [(left + right) / 2 - (b.minX + b.maxX) / 2, (safe.top + safe.bottom) / 2 - (b.minY + b.maxY) / 2],
    bbox: b,
  };
}
