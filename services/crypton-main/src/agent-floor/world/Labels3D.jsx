import React from 'react';
import { Html } from '@react-three/drei';
import { ACTIONS, DELEGATIONS, GRANTS, PRINCIPALS, RESOURCES, getEntity } from '../scenario';
import { CHIP_TEXT, RESOURCE_GLYPH, inspectLabel } from '../stageLabels';
import { SLAB, STATIONS, nodeAt } from './choreography';
import { useWorld } from './worldContext';

/**
 * The world's inspectable things as real buttons, anchored in 3D. Same accessible names
 * as the 2D stage (stageLabels.js), so a screen-reader or keyboard visitor gets the same
 * floor whichever renderer is active. Robot tags ride on the robots (see World3D).
 */

const Z = [30, 0]; // keep labels under the HUD (which sits at z-index 40+)

/**
 * drei <Html> anchored into the page-owned tag layer. Without a fixed `portal`, Html's DOM
 * target switches once R3F connects its events, and the first tag's root gets recreated
 * and emptied (React 19). A stable target avoids that.
 */
function Anchor({ children, ...rest }) {
  const { portal } = useWorld();
  return <Html portal={portal} center zIndexRange={Z} {...rest}>{children}</Html>;
}

export function Tag({ id, selection, focus, onSelect, children, className = '', hidden = false, attend }) {
  const cls = `af-wtag ${className}${selection === id ? ' is-selected' : ''}${focus === id ? ' is-focus' : ''}`;
  const on = attend ? () => attend(true) : undefined;
  const off = attend ? () => attend(false) : undefined;
  return (
    <button
      type="button"
      className={cls}
      hidden={hidden}
      aria-pressed={selection === id}
      aria-label={inspectLabel(id)}
      onClick={() => onSelect(id)}
      onMouseEnter={on}
      onMouseLeave={off}
      onFocus={on}
      onBlur={off}
    >
      {children}
    </button>
  );
}

export function RobotTag({ id, height, ...rest }) {
  const p = PRINCIPALS.find((x) => x.id === id);
  const { hover } = useWorld();
  // hovering or focusing the name tag makes the robot acknowledge, same as hovering the robot
  const attend = (on) => { if (on) hover.add(id); else hover.delete(id); };
  return (
    <Anchor position={[0, height + 0.6, 0]}>
      <Tag id={id} className="af-wtag--agent" attend={attend} {...rest}>{p.name.toUpperCase()}</Tag>
    </Anchor>
  );
}

// Plain names for the default story; the technical IDs appear when details are open.
const PLAIN = { 'PR-MGR': 'YOU', 'RS-REPORT': 'REPORT', 'RS-RECORDS': 'RECORDS', 'RS-EXTERNAL': 'OUTSIDE' };

const STATION_LIFT = { 'PR-MGR': 1.5, 'RS-REPORT': 1.55, 'RS-RECORDS': 2.0, 'RS-EXTERNAL': 1.45 };

function chipKind(id) {
  if (id.startsWith('GR')) return 'grant';
  if (id.startsWith('DL')) return 'delegation';
  return getEntity(id).status === 'attempted' ? 'attempted' : 'executed';
}

/** Where a chip sits: between the two things it relates, at the scene's blocking. */
function chipAnchor(id, step) {
  const e = getEntity(id);
  const ends = e.type === 'grant' || e.type === 'delegation' ? [e.from, e.to] : [e.actor, e.target || e.resource];
  const [a, b] = ends.map((n) => nodeAt(n, step));
  const nudge = { 'ACT-002': [0, 0.55], 'DL-001': [0, -0.55], 'ACT-003': [0, 0.4] }[id] || [0, 0];
  // ACT-001 sits right over Scout's name tag at the usual height, so it rides higher
  const lift = id === 'ACT-001' ? 2.35 : 1.65;
  return [(a[0] + b[0]) / 2 + nudge[0], SLAB.height + lift, (a[1] + b[1]) / 2 + nudge[1]];
}

// Every chip stays mounted and is hidden until its scene: unmounting drei <Html> roots
// mid-render races React 19, and a hidden button is neither focusable nor announced.
const ALL_CHIPS = [...GRANTS, ...DELEGATIONS, ...ACTIONS].map((e) => e.id);

export default function Labels3D({ step, vis, selection, focus, onSelect, detail = false }) {
  const shown = new Set([...vis.grants, ...vis.delegations, ...vis.actions].map((e) => e.id));
  const hasDestination = vis.actions.some((a) => a.destination);
  const stations = [{ id: 'PR-MGR' }, ...RESOURCES];
  return (
    <group>
      {stations.map(({ id }) => {
        const [x, z] = STATIONS[id].at;
        const base = id === 'RS-EXTERNAL' ? 0 : SLAB.height;
        return (
          <Anchor key={id} position={[x, base + STATION_LIFT[id], z]}>
            <Tag id={id} selection={selection} focus={focus} onSelect={onSelect} className={`af-wtag--station${id === 'RS-EXTERNAL' ? ' is-external' : ''}`}>
              {!detail ? PLAIN[id] : id === 'PR-MGR' ? 'YOU · MANAGER' : `${RESOURCE_GLYPH[id]} · ${id}`}
            </Tag>
          </Anchor>
        );
      })}
      {ALL_CHIPS.map((id) => (
        <Anchor key={id} position={chipAnchor(id, step)}>
          <Tag id={id} hidden={!detail || !shown.has(id)} selection={selection} focus={focus} onSelect={onSelect} className={`af-wtag--chip af-wtag--${chipKind(id)}`}>
            {CHIP_TEXT[id][0]} <span className="af-wtag-word">{CHIP_TEXT[id][1]}</span>
          </Tag>
        </Anchor>
      ))}
      {/* non-interactive place names */}
      <Anchor position={[STATIONS.gate.at[0] + 0.3, SLAB.height + 2.25, STATIONS.gate.at[1]]}>
        <span className="af-wplace af-wplace--boundary" aria-hidden="true">{detail ? 'ORGANISATION BOUNDARY' : 'EDGE OF YOUR ORGANISATION'}</span>
      </Anchor>
      <Anchor position={[STATIONS['RS-EXTERNAL'].at[0], 2.3, STATIONS['RS-EXTERNAL'].at[1]]}>
        <span className="af-wplace af-wplace--unknown" aria-hidden="true" hidden={!detail || !hasDestination}>? NO RECORD OF ARRIVAL</span>
      </Anchor>
    </group>
  );
}
