import React from 'react';
import { ROBOT_ART } from './Robots';
import { PRINCIPALS, RESOURCES, getEntity } from './scenario';

/**
 * One stage, two views. The SAME agent / resource / chip elements exist in both views;
 * only their layout and the backdrop change, so an agent keeps its identity as the
 * physical floor turns into the logical Authority Atlas. All facts come from
 * scenario.js via `vis`; this file only draws them.
 */

// Positions are percentages of the stage (left, top). Presentation only.
const FLOOR = {
  'PR-MGR': [15, 80],
  'AG-SCOUT': [26, 50],
  'AG-RELAY': [50, 58],
  'AG-COURIER': [72, 68],
  'RS-REPORT': [29, 72],
  'RS-RECORDS': [62, 27],
  'RS-EXTERNAL': [91, 46],
  'GR-001': [11, 44],
  'ACT-001': [45, 77],
  'DL-001': [42, 13],
  'ACT-002': [42, 24],
  'ACT-003': [66, 77],
};
// At the boundary scene Courier rolls toward the edge of the floor.
const FLOOR_COURIER_AT_BOUNDARY = [76, 66];

const ATLAS = {
  'PR-MGR': [13, 16],
  'AG-SCOUT': [26, 46],
  'AG-RELAY': [50, 46],
  'AG-COURIER': [74, 46],
  'RS-REPORT': [26, 84],
  'RS-RECORDS': [50, 84],
  'RS-EXTERNAL': [86, 84],
  'GR-001': [16, 30],
  'ACT-001': [26, 65],
  'DL-001': [38, 46],
  'ACT-002': [38, 60],
  'ACT-003': [62, 46],
};

export function layoutFor(view, step) {
  if (view === 'atlas') return ATLAS;
  return step >= 4 ? { ...FLOOR, 'AG-COURIER': FLOOR_COURIER_AT_BOUNDARY } : FLOOR;
}

const pos = (layout, id) => ({ left: `${layout[id][0]}%`, top: `${layout[id][1]}%` });
const vb = (layout, id) => [layout[id][0] * 10, layout[id][1] * 6];

// ── floor backdrop (isometric, drawn with plain SVG geometry) ──────────────────

const lerp = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
const P = [[500, 60], [975, 320], [500, 580], [25, 320]]; // floor diamond

function Box({ cx, cy, w, h, className }) {
  const a = w / 2;
  const b = a / 2;
  const top = `${cx},${cy - h - b} ${cx + a},${cy - h} ${cx},${cy - h + b} ${cx - a},${cy - h}`;
  const left = `${cx - a},${cy - h} ${cx},${cy - h + b} ${cx},${cy + b} ${cx - a},${cy}`;
  const right = `${cx},${cy - h + b} ${cx + a},${cy - h} ${cx + a},${cy} ${cx},${cy + b}`;
  return (
    <g className={className}>
      <polygon points={left} className="af-box-l" />
      <polygon points={right} className="af-box-r" />
      <polygon points={top} className="af-box-t" />
    </g>
  );
}

function FloorBackdrop() {
  const lines = [];
  for (let i = 1; i < 8; i += 1) {
    const t = i / 8;
    const a1 = lerp(P[0], P[3], t);
    const a2 = lerp(P[1], P[2], t);
    const b1 = lerp(P[0], P[1], t);
    const b2 = lerp(P[3], P[2], t);
    lines.push(<line key={`a${i}`} x1={a1[0]} y1={a1[1]} x2={a2[0]} y2={a2[1]} className="af-grid" />);
    lines.push(<line key={`b${i}`} x1={b1[0]} y1={b1[1]} x2={b2[0]} y2={b2[1]} className="af-grid" />);
  }
  return (
    <svg viewBox="0 0 1000 600" className="af-backdrop" aria-hidden="true" focusable="false" preserveAspectRatio="xMidYMid meet">
      <polygon points={P.map((p) => p.join(',')).join(' ')} className="af-floor" />
      {lines}
      {/* workstations */}
      <Box cx={262} cy={332} w={170} h={26} className="af-desk" />
      <Box cx={500} cy={384} w={190} h={26} className="af-desk" />
      <Box cx={150} cy={492} w={150} h={22} className="af-desk af-desk--manager" />
      <Box cx={625} cy={192} w={74} h={70} className="af-cabinet" />
      <polygon points="720,392 800,432 720,472 640,432" className="af-pad" />
      {/* external boundary: everything beyond this line is outside the organisation */}
      <polyline points="820,215 880,252 800,318 745,282" className="af-boundary" fill="none" />
      <polygon points="880,252 975,320 905,360 800,318" className="af-outside" />
      <text x="840" y="362" className="af-boundary-label">OUTSIDE</text>
    </svg>
  );
}

// ── atlas backdrop ────────────────────────────────────────────────────────────

function edgeClass(kind, entity) {
  if (kind === 'grant') return 'af-edge af-edge--grant';
  if (kind === 'delegation') return 'af-edge af-edge--delegation';
  return entity.status === 'attempted' ? 'af-edge af-edge--attempted' : 'af-edge af-edge--executed';
}

function AtlasBackdrop({ vis, layout }) {
  const edges = [
    ...vis.grants.map((g) => ({ id: g.id, kind: 'grant', from: g.from, to: g.to, e: g })),
    ...vis.delegations.map((d) => ({ id: d.id, kind: 'delegation', from: d.from, to: d.to, e: d })),
    ...vis.actions
      .filter((a) => a.id !== 'ACT-002') // same hop as DL-001; shown as its own chip
      .map((a) => ({ id: a.id, kind: 'action', from: a.actor, to: a.target || a.resource, e: a })),
  ];
  const external = vis.actions.some((a) => a.id === 'ACT-003');
  return (
    <svg viewBox="0 0 1000 600" className="af-backdrop" aria-hidden="true" focusable="false" preserveAspectRatio="xMidYMid meet">
      <defs>
        <marker id="af-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 z" className="af-arrow" />
        </marker>
      </defs>
      <rect x="0" y="0" width="1000" height="600" className="af-atlas-bg" />
      <line x1="40" y1="438" x2="960" y2="438" className="af-lane" />
      <line x1="40" y1="170" x2="960" y2="170" className="af-lane" />
      <text x="48" y="160" className="af-lane-label">PRINCIPALS</text>
      <text x="48" y="428" className="af-lane-label">RESOURCES</text>
      {edges.map((ed) => {
        const [x1, y1] = vb(layout, ed.from);
        const [x2, y2] = vb(layout, ed.to);
        return <line key={ed.id} x1={x1} y1={y1} x2={x2} y2={y2} className={edgeClass(ed.kind, ed.e)} markerEnd="url(#af-arrow)" />;
      })}
      {external && (
        <g>
          {/* the request is drawn up to the boundary only: no execution is modeled */}
          <line x1={vb(layout, 'AG-COURIER')[0]} y1={vb(layout, 'AG-COURIER')[1] + 40} x2={vb(layout, 'RS-EXTERNAL')[0]} y2={vb(layout, 'RS-EXTERNAL')[1] - 40} className="af-edge af-edge--unknown" />
          <text x="835" y="365" className="af-boundary-label">no record</text>
        </g>
      )}
    </svg>
  );
}

// ── interactive layer ─────────────────────────────────────────────────────────

const RESOURCE_GLYPH = {
  'RS-REPORT': 'RPT',
  'RS-RECORDS': 'REC',
  'RS-EXTERNAL': 'EXT',
};

// [id, short word]. The word is dropped in the Atlas, where space is tight and the legend carries it.
const CHIP_TEXT = {
  'GR-001': ['GR-001', 'grant'],
  'DL-001': ['DL-001', 'delegation'],
  'ACT-001': ['ACT-001', 'read'],
  'ACT-002': ['ACT-002', 'delegate'],
  'ACT-003': ['ACT-003', 'requested'],
};

function chipKind(id) {
  if (id.startsWith('GR')) return 'grant';
  if (id.startsWith('DL')) return 'delegation';
  return getEntity(id).status === 'attempted' ? 'attempted' : 'executed';
}

export default function Stage({ view, step, selection, focus, onSelect, vis }) {
  const layout = layoutFor(view, step);
  const chips = [...vis.grants, ...vis.delegations, ...vis.actions];
  const cls = (id) => `${selection === id ? ' is-selected' : ''}${focus === id ? ' is-focus' : ''}`;

  return (
    <div className="af-stage" data-view={view}>
      <div className={`af-layer af-layer--floor${view === 'floor' ? '' : ' is-off'}`}>
        <FloorBackdrop />
      </div>
      <div className={`af-layer af-layer--atlas${view === 'atlas' ? '' : ' is-off'}`}>
        <AtlasBackdrop vis={vis} layout={ATLAS} />
      </div>

      {/* manager */}
      <button
        type="button"
        className={`af-node af-node--manager${cls('PR-MGR')}`}
        style={pos(layout, 'PR-MGR')}
        aria-pressed={selection === 'PR-MGR'}
        aria-label="Inspect Manager (you, the human manager)"
        onClick={() => onSelect('PR-MGR')}
      >
        <span className="af-glyph" aria-hidden="true">YOU</span>
        <span className="af-tag">MANAGER</span>
      </button>

      {/* resources */}
      {RESOURCES.map((r) => (
        <button
          key={r.id}
          type="button"
          className={`af-node af-node--resource${cls(r.id)}${r.id === 'RS-EXTERNAL' ? ' is-external' : ''}`}
          style={pos(layout, r.id)}
          aria-pressed={selection === r.id}
          aria-label={`Inspect resource ${r.id}: ${r.name}`}
          onClick={() => onSelect(r.id)}
        >
          <span className="af-glyph" aria-hidden="true">{RESOURCE_GLYPH[r.id]}</span>
          <span className="af-tag">{r.id}</span>
        </button>
      ))}

      {/* agents: the same elements in both views */}
      {PRINCIPALS.filter((p) => p.kind === 'agent').map((p) => {
        const Art = ROBOT_ART[p.id];
        return (
          <button
            key={p.id}
            type="button"
            className={`af-agent af-agent--${p.name.toLowerCase()}${cls(p.id)}`}
            style={pos(layout, p.id)}
            aria-pressed={selection === p.id}
            aria-label={`Inspect ${p.name}, ${p.role.toLowerCase()}`}
            onClick={() => onSelect(p.id)}
          >
            <Art />
            <span className="af-tag">{p.name.toUpperCase()}</span>
          </button>
        );
      })}

      {/* grants, delegations and actions: the same chips in both views */}
      {chips.map((c) => (
        <button
          key={c.id}
          type="button"
          className={`af-chip af-chip--${chipKind(c.id)}${cls(c.id)}`}
          style={pos(layout, c.id)}
          aria-pressed={selection === c.id}
          aria-label={`Inspect ${c.id}`}
          onClick={() => onSelect(c.id)}
        >
          <span>{CHIP_TEXT[c.id][0]}</span>
          <span className="af-chip-word"> {CHIP_TEXT[c.id][1]}</span>
        </button>
      ))}

      {/* state markers that are never the only carrier of meaning (text duplicates them) */}
      {vis.actions.some((a) => a.id === 'ACT-003') && view === 'floor' && (
        <span className="af-stamp" style={{ left: '84%', top: '36%' }} aria-hidden="true">
          REQUEST ONLY
        </span>
      )}
    </div>
  );
}

