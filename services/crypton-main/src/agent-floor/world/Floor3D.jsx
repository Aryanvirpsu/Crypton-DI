import React, { useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { RoundedBox } from '@react-three/drei';
import * as THREE from 'three';
import { C, mat } from './palette';
import { BOUNDARY_X, SLAB, STATIONS, cueState } from './choreography';
import { Merged } from './merge';
import { SCENERY_GROUPS, CABINET_BASE, DESK_BASE, TERMINAL_BASE } from './scenery';
import { makeDeskScreen, makeFloorTexture, makePoolTexture, makeTerminalScreen } from './floorArt';
import Ecosystem3D from './Ecosystem3D';
import { useWorld } from './worldContext';

/**
 * The miniature operations floor: slab, painted floor, workstations and props, the boundary
 * of the organisation, and the external destination across the gap. It carries no scenario
 * facts. Stations come from STATIONS so labels and robots agree.
 *
 * Stations wake up as the story reaches them (the `act` levels): the manager's desk while the
 * task is set, the report terminal while Scout reads, the hub while work changes hands, the
 * dock once Courier has the request. Activation is state, so it changes with the scene and
 * holds with the frame; under reduced motion it still changes, it just does not animate.
 */

const TOP = SLAB.height;
const damp = THREE.MathUtils.damp;
const stationPos = (id) => STATIONS[id].at;

/** Station activation, eased: desk, terminal, hub, dock. Written by Floor3D, read by the parts. */
function useActivation(step) {
  const world = useWorld();
  const act = useRef({ desk: 1, terminal: 0.3, hub: 0.35, dock: 0.3 });
  const stepRef = useRef(step);
  stepRef.current = step;
  useFrame((_, rawDt) => {
    const dt = Math.min(rawDt, 0.1);
    const s = stepRef.current;
    const sceneT = world.sceneTime.current;
    const want = {
      desk: s <= 1 ? 1 : 0.5,
      terminal: s === 2 ? 1 : s === 1 ? 0.45 : 0.3,
      hub: s === 3 ? 1 : s >= 4 ? 0.8 : 0.35,
      dock: s >= 4 && cueState(s, 4, 'receive', sceneT) ? 1 : 0.3,
    };
    const k = world.reduced ? 1 : dt;
    Object.keys(want).forEach((key) => { act.current[key] = damp(act.current[key], want[key], 4, k); });
  });
  return act;
}

function Slab() {
  const floor = useMemo(() => {
    const t = makeFloorTexture();
    // the painted plate sits a hair inside the slab's rounded edge
    t.repeat.set(13.76 / SLAB.width, 8.76 / SLAB.depth);
    t.offset.set(0.12 / SLAB.width, 0.12 / SLAB.depth);
    return t;
  }, []);
  return (
    <group>
      <RoundedBox args={[SLAB.width, SLAB.height, SLAB.depth]} radius={0.12} position={[0, TOP / 2, 0]} material={mat(C.machine)} receiveShadow />
      {/* a thin graphite skirt gives the slab a machined edge */}
      <mesh position={[0, 0.08, 0]} material={mat(C.graphite)}>
        <boxGeometry args={[SLAB.width + 0.08, 0.16, SLAB.depth + 0.08]} />
      </mesh>
      <mesh position={[0, TOP + 0.003, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <planeGeometry args={[13.76, 8.76]} />
        <meshStandardMaterial map={floor} roughness={0.72} metalness={0.08} />
      </mesh>
    </group>
  );
}

/** Soft additive pools of light on the floor under each station. Level follows activation. */
function Pools({ act }) {
  const tex = useMemo(makePoolTexture, []);
  const refs = useRef({});
  const specs = useMemo(() => [
    { key: 'desk', at: stationPos('PR-MGR'), size: 3.4, color: C.bone, max: 0.2 },
    { key: 'terminal', at: [-3.4, -1.0], size: 4.2, color: C.lime, max: 0.26 },
    { key: 'hub', at: stationPos('hub'), size: 3.6, color: C.lime, max: 0.2 },
    { key: 'dock', at: [STATIONS.dock.at[0] + 0.3, STATIONS.dock.at[1]], size: 3.8, color: C.lime, max: 0.22 },
  ], []);
  const coral = useMemo(() => new THREE.Color(C.coral), []);
  const lime = useMemo(() => new THREE.Color(C.lime), []);
  const world = useWorld();
  useFrame((_, dt) => {
    specs.forEach((s) => {
      const m = refs.current[s.key];
      if (!m) return;
      m.opacity = s.max * act.current[s.key];
      if (s.key === 'dock') {
        // once Courier has the request, the bay is washed in the same coral as the request
        m.color.lerp(act.current.dock > 0.9 ? coral : lime, world.reduced ? 1 : Math.min(1, dt * 4));
      }
    });
  });
  return (
    <group>
      {specs.map((s) => (
        <mesh key={s.key} position={[s.at[0], TOP + 0.012, s.at[1]]} rotation={[-Math.PI / 2, 0, 0]} renderOrder={1}>
          <planeGeometry args={[s.size, s.size]} />
          <meshBasicMaterial ref={(m) => { refs.current[s.key] = m; }} map={tex} color={s.color} transparent opacity={0} depthWrite={false} blending={THREE.AdditiveBlending} toneMapped={false} />
        </mesh>
      ))}
    </group>
  );
}

function DeskLive({ step, act }) {
  const world = useWorld();
  const screen = useMemo(makeDeskScreen, []);
  const card = useRef();
  const screenMat = useRef();
  useFrame(() => {
    // the task card waits on the desk until it is handed to Scout
    if (card.current) card.current.visible = step === 0 || (step === 1 && !cueState(1, 1, 'give', world.sceneTime.current) && !world.reduced);
    if (screenMat.current) screenMat.current.color.setScalar(0.45 + 0.55 * act.current.desk);
  });
  return (
    <group position={[DESK_BASE.pos[0], DESK_BASE.pos[1], DESK_BASE.pos[2]]} rotation={[0, DESK_BASE.yaw, 0]}>
      <mesh position={[0, 0.94, -0.197]} rotation={[-0.12, 0, 0]}>
        <planeGeometry args={[0.62, 0.34]} />
        <meshBasicMaterial ref={screenMat} map={screen} toneMapped={false} />
      </mesh>
      <mesh ref={card} position={[0.45, 0.67, 0.12]} rotation={[-Math.PI / 2, 0, 0.2]} material={mat(C.lime, 'glow')}>
        <planeGeometry args={[0.28, 0.18]} />
      </mesh>
    </group>
  );
}

function TerminalLive({ act }) {
  const { reduced, timeScale } = useWorld();
  const screen = useMemo(makeTerminalScreen, []);
  const screenMat = useRef();
  const lamp = useMemo(() => mat(C.lime, 'glow').clone(), []);
  const light = useRef();
  useFrame((_, rawDt) => {
    const level = act.current.terminal;
    // text scrolls while Scout reads, and the screen brightens with its activity
    if (!reduced) screen.offset.y = (screen.offset.y - Math.min(rawDt, 0.1) * timeScale.current * 0.35 * level) % 1;
    if (screenMat.current) screenMat.current.color.setScalar(0.3 + 0.7 * level);
    lamp.emissiveIntensity = 0.3 + 1.7 * level;
    if (light.current) light.current.intensity = 0.4 + 1.6 * level;
  });
  return (
    <group position={[TERMINAL_BASE.pos[0], TERMINAL_BASE.pos[1], TERMINAL_BASE.pos[2]]}>
      <mesh position={[0, 0.96, 0.079]} rotation={[-0.55, 0, 0]}>
        <planeGeometry args={[0.74, 0.38]} />
        <meshBasicMaterial ref={screenMat} map={screen} toneMapped={false} />
      </mesh>
      {/* reading lamp on top of the screen */}
      <mesh position={[0, 1.2, 0.02]} material={lamp}>
        <boxGeometry args={[0.5, 0.025, 0.04]} />
      </mesh>
      <pointLight ref={light} position={[0, 1.2, 0.6]} color={C.lime} intensity={1.4} distance={3} decay={2} />
    </group>
  );
}

/** Three drawer lamps on the records cabinets, slowly taking turns. Neutral: they say nothing about access. */
function CabinetLive() {
  const { time, reduced } = useWorld();
  const lamps = useMemo(() => [0, 1, 2].map(() => mat(C.mist, 'glow').clone()), []);
  useFrame(() => {
    const on = reduced ? -1 : Math.floor(time.current * 0.5) % 3;
    lamps.forEach((m, i) => { m.emissiveIntensity = i === on ? 1.1 : 0.18; });
  });
  return (
    <group position={[CABINET_BASE.pos[0], CABINET_BASE.pos[1], CABINET_BASE.pos[2]]}>
      {[-0.55, 0, 0.55].map((x, i) => (
        <mesh key={x} position={[x, 1.5, 0.362]} material={lamps[i]}>
          <boxGeometry args={[0.05, 0.03, 0.01]} />
        </mesh>
      ))}
    </group>
  );
}

/** The ring of tiles around Relay's pad: turns slowly, faster while work changes hands. */
const HUB_TILES = Array.from({ length: 16 }, (_, i) => {
  const a = (i / 16) * Math.PI * 2;
  return { g: 'box', a: [0.07, 0.02, 0.07], p: [Math.cos(a) * 0.88, 0.05, Math.sin(a) * 0.88], r: [0, -a, 0], m: i % 4 === 0 ? [C.lime, 'glow'] : [C.mist, 'matte'] };
});
const HUB_RING_BASE = {};
function HubLive({ act }) {
  const ring = useRef();
  const { reduced, timeScale } = useWorld();
  const angle = useRef(0);
  useFrame((_, rawDt) => {
    if (reduced || !ring.current) return;
    angle.current += Math.min(rawDt, 0.1) * timeScale.current * (0.3 + 1.1 * act.current.hub);
    ring.current.rotation.y = angle.current;
  });
  return (
    <group position={[STATIONS.hub.at[0], TOP, STATIONS.hub.at[1]]}>
      <group ref={ring}>
        <Merged parts={HUB_TILES} base={HUB_RING_BASE} castShadow={false} />
      </group>
    </group>
  );
}

/** Bay lights: a slow lime chase at rest, a quick coral chase once Courier has the request. */
function DockLive({ act }) {
  const { time, reduced } = useWorld();
  const [dx, dz] = STATIONS.dock.at;
  const lamps = useMemo(() => [0, 1, 2, 3].map(() => mat(C.lime, 'glow').clone()), []);
  const coral = useMemo(() => new THREE.Color(C.coral), []);
  const lime = useMemo(() => new THREE.Color(C.lime), []);
  useFrame((_, dt) => {
    const live = act.current.dock > 0.9;
    const speed = live ? 7 : 1.4;
    const idx = (time.current * speed) % 4;
    lamps.forEach((m, i) => {
      const near = reduced ? (live ? 1 : 0.5) : Math.max(0, 1 - Math.min(Math.abs(i - idx), 4 - Math.abs(i - idx)));
      m.emissiveIntensity = 0.2 + (live ? 2.2 : 1) * near;
      m.emissive.lerp(live ? coral : lime, Math.min(1, dt * 5));
    });
  });
  return (
    <group>
      {lamps.map((m, i) => (
        <mesh key={i} position={[dx - 0.55 + i * 0.37, TOP + 0.045, dz - 0.62]} material={m}>
          <boxGeometry args={[0.12, 0.025, 0.04]} />
        </mesh>
      ))}
    </group>
  );
}

/** The organisation's edge: a coral seam along the slab, with a lamp over the gate. */
function Boundary() {
  const [, gz] = STATIONS.gate.at;
  return (
    <group>
      <mesh position={[BOUNDARY_X - 0.04, TOP + 0.01, 0]} material={mat(C.coral, 'glow')}>
        <boxGeometry args={[0.05, 0.02, SLAB.depth - 0.1]} />
      </mesh>
      <pointLight position={[BOUNDARY_X + 0.4, 1.6, gz]} color={C.coral} intensity={2.2} distance={5} decay={2} />
    </group>
  );
}

/** The external destination: a small platform across the gap, outside the organisation. */
function ExternalPod() {
  const g = useRef();
  const ring = useRef();
  const { time, reduced } = useWorld();
  const [x, z] = STATIONS['RS-EXTERNAL'].at;
  useFrame(() => {
    if (reduced) return;
    if (g.current) g.current.position.y = -0.25 + Math.sin(time.current * 0.8) * 0.06;
    if (ring.current) ring.current.rotation.z = time.current * 0.5;
  });
  return (
    <group ref={g} position={[x, -0.25, z]}>
      <mesh material={mat(C.graphite, 'gloss')} castShadow receiveShadow>
        <cylinderGeometry args={[1.25, 1.1, 0.32, 6]} />
      </mesh>
      <RoundedBox args={[0.8, 0.9, 0.8]} radius={0.06} position={[0, 0.61, 0]} material={mat(C.machine, 'metal')} castShadow />
      {[0.35, 0.6, 0.85].map((y) => (
        <mesh key={y} position={[0, y, 0.405]} material={mat(C.coral, 'glow')}>
          <boxGeometry args={[0.5, 0.03, 0.01]} />
        </mesh>
      ))}
      {/* a slow, tilted ring: something out there, unobserved */}
      <mesh ref={ring} position={[0, 1.35, 0]} rotation={[Math.PI / 2.4, 0, 0]} material={mat(C.mist, 'matte')}>
        <torusGeometry args={[0.55, 0.012, 6, 40]} />
      </mesh>
    </group>
  );
}

export default function Floor3D({ step = 0, showEcosystem }) {
  const act = useActivation(step);
  return (
    <group>
      <Slab />
      <Pools act={act} />
      <Merged groups={SCENERY_GROUPS} />
      <DeskLive step={step} act={act} />
      <TerminalLive act={act} />
      <CabinetLive />
      <HubLive act={act} />
      <DockLive act={act} />
      <Boundary />
      <ExternalPod />
      <Ecosystem3D visible={showEcosystem} />
    </group>
  );
}

