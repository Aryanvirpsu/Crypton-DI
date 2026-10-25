import React, { useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Line } from '@react-three/drei';
import * as THREE from 'three';
import { C, mat } from './palette';
import { CUES, ENVELOPES, SLAB, STATIONS } from './choreography';
import { ROBOT_HEIGHT } from './Robots3D';
import { useWorld } from './worldContext';

/**
 * Authority, drawn in space. Everything here is driven by the authority graph for the
 * current scene (`edges`, from authorityGraph.js): envelopes for grants and delegations,
 * the attempted request, and the Atlas arcs. Nothing is drawn that the graph lacks.
 */

const damp = THREE.MathUtils.damp;
const Y = SLAB.height;

const STATION_Y = { 'PR-MGR': 1.2, 'RS-REPORT': 1.1, 'RS-RECORDS': 1.7, 'RS-EXTERNAL': 1.0 };

/** Live anchor point for a node: robots move, stations do not. Writes into `out`. */
function anchor(world, id, out, lift = 0.62) {
  const r = world.robots.get(id);
  if (r) return out.set(r.position.x, Y + ROBOT_HEIGHT[id] * lift, r.position.z);
  const s = STATIONS[id];
  return out.set(s.at[0], (id === 'RS-EXTERNAL' ? 0 : Y) + (STATION_Y[id] || 0.8), s.at[1]);
}

// ── envelopes ───────────────────────────────────────────────────────────────────────

function Envelope({ id, shown, emphasis }) {
  const e = ENVELOPES[id];
  const g = useRef();
  const face = useRef();
  const { reduced, time } = useWorld();
  const edges = useMemo(() => new THREE.EdgesGeometry(new THREE.BoxGeometry(e.size[0], e.height, e.size[1])), [e]);
  useFrame((_, dt) => {
    const k = reduced ? 1 : dt;
    const s = damp(g.current.scale.y, shown ? 1 : 0.001, 3.2, k);
    g.current.scale.set(1, s, 1);
    g.current.position.y = Y + (e.height * s) / 2;
    g.current.visible = s > 0.01;
    const pulse = reduced ? 0 : Math.sin(time.current * 1.6) * 0.015;
    face.current.material.opacity = damp(face.current.material.opacity, (emphasis ? 0.11 : 0.05) + pulse, 4, k);
  });
  return (
    <group ref={g} position={[e.center[0], Y, e.center[1]]} scale={[1, 0.001, 1]}>
      <mesh ref={face} renderOrder={2}>
        <boxGeometry args={[e.size[0], e.height, e.size[1]]} />
        <meshBasicMaterial color={C.lime} transparent opacity={0.05} depthWrite={false} side={THREE.DoubleSide} />
      </mesh>
      <lineSegments geometry={edges} renderOrder={3}>
        <lineBasicMaterial color={C.lime} transparent opacity={0.75} />
      </lineSegments>
      {/* floor footprint */}
      <mesh position={[0, -e.height / 2 + 0.012, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <planeGeometry args={[e.size[0], e.size[1]]} />
        <meshBasicMaterial color={C.lime} transparent opacity={0.07} depthWrite={false} />
      </mesh>
    </group>
  );
}

// ── live lines between nodes ────────────────────────────────────────────────────────

const STYLE = {
  grant: { color: C.lime, width: 2.2, dashed: false, opacity: 0.95 },
  delegation: { color: C.lime, width: 2.2, dashed: true, dash: 0.22, gap: 0.12, opacity: 0.95 },
  executed: { color: C.bone, width: 1.8, dashed: false, opacity: 0.85 },
  attempted: { color: C.coral, width: 2.6, dashed: true, dash: 0.2, gap: 0.14, opacity: 1 },
  unrecorded: { color: C.coral, width: 1.6, dashed: true, dash: 0.06, gap: 0.16, opacity: 0.6 },
};

const SEGMENTS = 24;

/**
 * A line from node A to node B that follows the robots. `arc` lifts the middle (Atlas).
 * Dashes crawl along the line while the world runs and stop when the frame is held.
 */
function LiveLine({ from, to, kind, arc = 0, lift }) {
  const world = useWorld();
  const ref = useRef();
  const st = STYLE[kind];
  const a = useMemo(() => new THREE.Vector3(), []);
  const b = useMemo(() => new THREE.Vector3(), []);
  const m = useMemo(() => new THREE.Vector3(), []);
  const curve = useMemo(() => new THREE.QuadraticBezierCurve3(new THREE.Vector3(), new THREE.Vector3(), new THREE.Vector3()), []);
  const initial = useMemo(() => Array.from({ length: SEGMENTS + 1 }, (_, i) => [i, 0, 0]), []);
  const flat = useMemo(() => new Float32Array((SEGMENTS + 1) * 3), []);

  useFrame((_, dt) => {
    const line = ref.current;
    if (!line) return;
    anchor(world, from, a, lift);
    anchor(world, to, b, lift);
    m.addVectors(a, b).multiplyScalar(0.5);
    m.y += arc * Math.min(4, a.distanceTo(b) * 0.35);
    curve.v0.copy(a); curve.v1.copy(m); curve.v2.copy(b);
    for (let i = 0; i <= SEGMENTS; i += 1) {
      const p = curve.getPoint(i / SEGMENTS);
      flat[i * 3] = p.x; flat[i * 3 + 1] = p.y; flat[i * 3 + 2] = p.z;
    }
    line.geometry.setPositions(flat);
    if (st.dashed) {
      line.computeLineDistances();
      if (!world.reduced) line.material.dashOffset -= dt * 0.5 * world.timeScale.current;
    }
  });

  return (
    <Line
      ref={ref}
      points={initial}
      color={st.color}
      lineWidth={st.width}
      dashed={st.dashed}
      dashSize={st.dash}
      gapSize={st.gap}
      transparent
      opacity={st.opacity}
      depthWrite={false}
      renderOrder={4}
    />
  );
}

// ── transfers: a thing carried from A to B during a scene ───────────────────────────

const TRAIL = 4;
const ease = (p) => (p < 0.5 ? 2 * p * p : 1 - (-2 * p + 2) ** 2 / 2);

/**
 * A small object that travels from `from` to `to` during scene `at`, timed by the shared cues
 * (choreography.CUES), then rests above the receiver (or disappears, if `rest` is false).
 * To make the handoff easy to read it leaves a short fading trail, and a ring opens on the
 * floor where it leaves and again where it lands. Progress runs on real scene time so it
 * completes before the frame is held. Under reduced motion it simply sits at its destination.
 */
function Transfer({ step, at, from, to, rest = true, color = C.lime, children }) {
  const world = useWorld();
  const g = useRef();
  const trail = useRef([]);
  const rings = useRef([]);
  const a = useMemo(() => new THREE.Vector3(), []);
  const b = useMemo(() => new THREE.Vector3(), []);
  const { give, receive } = CUES[at];
  const duration = receive - give;

  const place = (node, p, v) => {
    node.position.lerpVectors(a, b, p);
    node.position.y += Math.sin(p * Math.PI) * 0.9;
    if (v !== undefined) node.visible = v;
  };

  useFrame(() => {
    const node = g.current;
    const hideAll = () => {
      node.visible = false;
      trail.current.forEach((m) => { if (m) m.visible = false; });
      rings.current.forEach((m) => { if (m) m.visible = false; });
    };
    if (step < at || (!rest && step > at)) { hideAll(); return; }
    const t = world.sceneTime.current;
    const live = step === at && !world.reduced;
    const raw = step > at || world.reduced ? 1 : THREE.MathUtils.clamp((t - give) / duration, 0, 1);
    // carried things travel and rest just above the robots' heads
    anchor(world, from, a, 1.1);
    anchor(world, to, b, 1.1);
    const p = ease(raw);
    place(node, p, rest || raw < 1);
    node.rotation.y = p * Math.PI * 2;

    // fading trail while it travels
    const travelling = live && raw > 0 && raw < 1;
    trail.current.forEach((m, i) => {
      if (!m) return;
      const q = ease(THREE.MathUtils.clamp(raw - (i + 1) * 0.07, 0, 1));
      place(m, q, travelling && q > 0.01);
      m.scale.setScalar(1 - (i + 1) * 0.18);
      m.material.opacity = 0.5 - i * 0.11;
    });

    // rings on the floor: one where it leaves (at the give cue), one where it lands (at the receive cue)
    [[from, give], [to, receive]].forEach(([id, cue], i) => {
      const ring = rings.current[i];
      if (!ring) return;
      const d = (t - cue) / 0.75;
      const on = live && d >= 0 && d < 1;
      ring.visible = on;
      if (!on) return;
      anchor(world, id, ring.position, 0);
      ring.position.y = SLAB.height + 0.03;
      ring.scale.setScalar(0.5 + d * 1.3);
      ring.material.opacity = (1 - d) * 0.7;
    });
  });

  return (
    <>
      <group ref={g} visible={false}>{children}</group>
      {Array.from({ length: TRAIL }, (_, i) => (
        <mesh key={i} ref={(el) => { trail.current[i] = el; }} visible={false}>
          <sphereGeometry args={[0.07, 8, 6]} />
          <meshBasicMaterial color={color} transparent opacity={0.4} depthWrite={false} />
        </mesh>
      ))}
      {[0, 1].map((i) => (
        <mesh key={i} ref={(el) => { rings.current[i] = el; }} rotation={[-Math.PI / 2, 0, 0]} visible={false}>
          <ringGeometry args={[0.5, 0.56, 36]} />
          <meshBasicMaterial color={color} transparent opacity={0.6} depthWrite={false} />
        </mesh>
      ))}
    </>
  );
}

// ── composition ─────────────────────────────────────────────────────────────────────

export default function Authority3D({ step, view, edges, focus }) {
  const atlas = view === 'atlas';
  const has = (id) => edges.some((e) => e.id === id);
  const request = edges.find((e) => e.kind === 'attempted');
  const unrecorded = edges.find((e) => e.kind === 'unrecorded');

  return (
    <group>
      {Object.keys(ENVELOPES).map((id) => (
        <Envelope key={id} id={id} shown={has(id) && !atlas} emphasis={focus === id} />
      ))}

      {/* the delegation carries "summarize RS-REPORT": a tether from Relay's envelope to the report */}
      {has('DL-001') && !atlas && <LiveLine from="AG-RELAY" to="RS-REPORT" kind="delegation" arc={0.25} lift={0.3} />}

      {/* the request that no grant covers, and the destination it points at, unrecorded */}
      {request && !atlas && <LiveLine from={request.from} to={request.to} kind="attempted" arc={0.35} />}
      {unrecorded && !atlas && <LiveLine from={unrecorded.from} to={unrecorded.to} kind="unrecorded" arc={0.5} lift={0.5} />}

      {/* the Atlas: every edge in the graph as an arc over the physical floor */}
      {atlas && edges.map((e) => <LiveLine key={e.id} from={e.from} to={e.to} kind={e.kind} arc={1} />)}

      {/* S01: the task card leaves the desk for Scout */}
      <Transfer step={step} at={1} from="PR-MGR" to="AG-SCOUT" rest={false}>
        <mesh rotation={[-0.4, 0, 0]} material={mat(C.lime, 'glow')}>
          <boxGeometry args={[0.32, 0.02, 0.22]} />
        </mesh>
      </Transfer>
      {/* S03: the delegation token, carrying only the summarize scope, goes to Relay */}
      <Transfer step={step} at={3} from="AG-SCOUT" to="AG-RELAY">
        <mesh material={mat(C.lime, 'glow')}>
          <cylinderGeometry args={[0.11, 0.11, 0.05, 24]} />
        </mesh>
      </Transfer>
      {/* S04: the export request travels from Relay to Courier. The frame holds after it lands. */}
      <Transfer step={step} at={4} from="AG-RELAY" to="AG-COURIER" color={C.coral}>
        <mesh material={mat(C.coral, 'glow')}>
          <octahedronGeometry args={[0.13, 0]} />
        </mesh>
      </Transfer>
    </group>
  );
}
