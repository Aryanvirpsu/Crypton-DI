import React, { useEffect, useMemo, useRef } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { C } from './palette';
import { FREEZE_STEP, SHOTS, shotFor } from './choreography';
import { STEPS } from '../scenario';
import { WorldContext, useWorld } from './worldContext';
import Floor3D from './Floor3D';
import Authority3D from './Authority3D';
import Labels3D, { RobotTag } from './Labels3D';
import { ROBOT_HEIGHT, Robot } from './Robots3D';
import { graphAt } from '../authorityGraph';

/**
 * The 3D Agent Floor. Renders the scene the reducer is on; it never changes the scene.
 *
 * Props (all from AgentFloor): step, view, selection, focus, onSelect, vis, reduced,
 * active (render loop on/off), progressRef (0..1 scroll position inside the scene),
 * clockRef (a DOM node the world writes its timecode into), inset (px the subject
 * should sit away from the centre, so HUD panels do not cover it).
 */

const damp = THREE.MathUtils.damp;
// Real seconds into the boundary scene at which the frame holds (after the request lands).
const HOLD_AT = 1.9;

/**
 * Render statistics for performance review. Inert unless the URL has ?perf=1, in which case
 * it publishes draw calls, triangles and frame rate to window.__afPerf once a second.
 */
function PerfProbe() {
  const gl = useThree((st) => st.gl);
  const on = typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('perf') === '1';
  const acc = useRef({ frames: 0, t0: 0, maxMs: 0 });
  useFrame((_, dt) => {
    if (!on) return;
    const a = acc.current;
    const now = performance.now();
    if (!a.t0) a.t0 = now;
    a.frames += 1;
    a.maxMs = Math.max(a.maxMs, dt * 1000);
    if (now - a.t0 >= 1000) {
      const i = gl.info;
      window.__afPerf = {
        fps: Math.round((a.frames * 1000) / (now - a.t0)),
        worstFrameMs: Math.round(a.maxMs),
        calls: i.render.calls,
        triangles: i.render.triangles,
        geometries: i.memory.geometries,
        textures: i.memory.textures,
      };
      a.frames = 0; a.t0 = now; a.maxMs = 0;
    }
  });
  return null;
}

/** Runs first each frame: the world clock, and holding the frame at the boundary. */
function Director({ step, clockRef }) {
  const world = useWorld();
  const last = useRef(step);
  useFrame((_, rawDt) => {
    const dt = Math.min(rawDt, 0.1);
    if (last.current !== step) { last.current = step; world.sceneTime.current = 0; }
    world.sceneTime.current += dt;
    world.live.current += dt;
    const held = step > FREEZE_STEP || (step === FREEZE_STEP && (world.reduced || world.sceneTime.current > HOLD_AT));
    let k = world.reduced ? (held ? 0 : 1) : damp(world.timeScale.current, held ? 0 : 1, held ? 6 : 3, dt);
    if (held && k < 0.02) k = 0; // a held frame is held, not slowly creeping
    world.timeScale.current = k;
    world.time.current += dt * world.timeScale.current;
    const el = clockRef && clockRef.current;
    if (el) {
      const t = world.time.current;
      const mm = String(Math.floor(t / 60)).padStart(2, '0');
      const ss = (t % 60).toFixed(2).padStart(5, '0');
      el.textContent = `T+${mm}:${ss}`;
    }
  });
  return null;
}

/** Key light dims and a coral spot finds the request when the frame is held. */
function Lights({ step }) {
  const key = useRef();
  const hemi = useRef();
  const spot = useRef();
  const world = useWorld();
  useEffect(() => {
    // the target is not in the scene graph, so place it and update its matrix by hand
    spot.current.target.position.set(3.4, 0.6, 1.4);
    spot.current.target.updateMatrixWorld();
  }, []);
  useFrame((_, dt) => {
    const held = 1 - world.timeScale.current; // 0 running, 1 held
    const k = world.reduced ? 1 : dt;
    key.current.intensity = damp(key.current.intensity, 3.1 - held * 1.7, 3, k);
    hemi.current.intensity = damp(hemi.current.intensity, 0.95 - held * 0.5, 3, k);
    spot.current.intensity = damp(spot.current.intensity, step >= FREEZE_STEP ? 9 * Math.max(held, 0.3) : 0, 3, k);
  });
  return (
    <>
      <hemisphereLight ref={hemi} args={[C.bone, C.graphite, 0.7]} />
      <directionalLight
        ref={key}
        position={[-7, 14, 9]}
        intensity={2.4}
        color={C.bone}
        castShadow
        shadow-mapSize={[2048, 2048]}
        shadow-bias={-0.0004}
        shadow-camera-left={-12}
        shadow-camera-right={14}
        shadow-camera-top={9}
        shadow-camera-bottom={-9}
        shadow-camera-near={1}
        shadow-camera-far={45}
      />
      <directionalLight position={[9, 5, -9]} intensity={0.55} color={C.mist} />
      <spotLight ref={spot} position={[3.2, 6.5, 3.6]} angle={0.42} penumbra={0.7} intensity={0} color={C.coral} distance={14} decay={1.4} />
    </>
  );
}

/** Damped camera that frames each scene, follows robots, and keeps the subject clear of the HUD. */
function CameraRig({ step, view, progressRef, inset }) {
  const { camera, size, pointer } = useThree();
  const world = useWorld();
  const look = useRef(null);
  const tmpPos = useMemo(() => new THREE.Vector3(), []);
  const tmpTarget = useMemo(() => new THREE.Vector3(), []);

  // The HUD offsets the projection so subjects stay clear of panels. The offset eases to
  // its target instead of jumping, so a layout change never snaps the world sideways.
  const target = useRef(inset);
  target.current = inset;
  const cur = useRef(null);
  useEffect(() => {
    cur.current = [target.current[0], target.current[1]];
    camera.setViewOffset(size.width, size.height, -cur.current[0], -cur.current[1], size.width, size.height);
    return () => camera.clearViewOffset();
  }, [camera, size.width, size.height]);

  useFrame((_, rawDt) => {
    const dt = Math.min(rawDt, 0.1);
    if (cur.current) {
      const k = world.reduced ? 1 : dt;
      const nx = damp(cur.current[0], target.current[0], 3, k);
      const ny = damp(cur.current[1], target.current[1], 3, k);
      if (Math.abs(nx - cur.current[0]) + Math.abs(ny - cur.current[1]) > 0.05 || world.reduced) {
        cur.current = [nx, ny];
        camera.setViewOffset(size.width, size.height, -nx, -ny, size.width, size.height);
      }
    }
    const shot = shotFor(step, view, STEPS[step].defaultView);
    tmpTarget.set(...shot.target);
    tmpPos.set(...shot.pos);
    if (shot.follow) {
      const r = world.robots.get(shot.follow);
      if (r) { tmpTarget.add(r.position); tmpPos.add(r.position); }
    }
    // scrolling through a scene eases the camera slightly closer; portrait screens
    // (phones) pull back so the same subjects still fit across the narrow width
    const p = progressRef ? progressRef.current : 0;
    const aspect = size.width / size.height;
    const portrait = aspect < 1 ? Math.min(1.7, 1 + (1 - aspect) * 1.1) : 1;
    tmpPos.sub(tmpTarget).multiplyScalar((1 - 0.07 * p) * portrait).add(tmpTarget);
    if (!world.reduced) {
      tmpPos.x += pointer.x * 0.35;
      tmpPos.y += pointer.y * 0.2;
      // behind the call to action the world drifts slowly, so it stays alive but calm
      if (step === 7 && view === STEPS[7].defaultView) {
        const a = Math.sin(world.live.current * 0.06) * 0.1;
        const dx = tmpPos.x - tmpTarget.x;
        const dz = tmpPos.z - tmpTarget.z;
        tmpPos.x = tmpTarget.x + dx * Math.cos(a) - dz * Math.sin(a);
        tmpPos.z = tmpTarget.z + dx * Math.sin(a) + dz * Math.cos(a);
      }
    }
    if (!look.current || world.reduced) {
      look.current = tmpTarget.clone();
      camera.position.copy(tmpPos);
    } else {
      const a = 1 - Math.exp(-2.1 * dt);
      camera.position.lerp(tmpPos, a);
      look.current.lerp(tmpTarget, a);
    }
    camera.lookAt(look.current);
    const fov = world.reduced ? shot.fov : damp(camera.fov, shot.fov, 2, dt);
    if (Math.abs(fov - camera.fov) > 1e-3) { camera.fov = fov; camera.updateProjectionMatrix(); }
  });
  return null;
}

function Scene(props) {
  const { step, view, selection, focus, onSelect, vis } = props;
  const edges = useMemo(() => graphAt(step).edges, [step]);
  const tagProps = { selection, focus, onSelect };
  return (
    <>
      <Director step={step} clockRef={props.clockRef} />
      <PerfProbe />
      <CameraRig step={step} view={view} progressRef={props.progressRef} inset={props.inset} />
      <color attach="background" args={[C.graphite]} />
      <fog attach="fog" args={[C.graphite, 45, 175]} />
      <Lights step={step} />
      <Floor3D step={step} showEcosystem={step === 7 && view === STEPS[7].defaultView} />
      {['AG-SCOUT', 'AG-RELAY', 'AG-COURIER'].map((id) => (
        <Robot key={id} id={id} step={step} selected={selection === id} onSelect={onSelect}>
          <RobotTag id={id} height={ROBOT_HEIGHT[id]} {...tagProps} />
        </Robot>
      ))}
      <Authority3D step={step} view={view} edges={edges} focus={focus} />
      <Labels3D step={step} vis={vis} detail={props.detail} {...tagProps} />
    </>
  );
}

export default function World3D(props) {
  const { step, active, reduced } = props;
  const ctx = useMemo(
    () => ({
      time: { current: 0 },
      live: { current: 0 },
      sceneTime: { current: 0 },
      timeScale: { current: 1 },
      robots: new Map(),
      hover: new Set(), // robot ids hovered or focused (pointer or the name tag)
      reduced,
      portal: props.tagLayerRef,
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  ctx.reduced = reduced;
  const first = SHOTS[step];

  return (
    <Canvas
      className="af-canvas"
      shadows="percentage"
      dpr={props.cta ? 1 : [1, 1.75]}
      frameloop={active ? 'always' : 'never'}
      camera={{ position: first.pos, fov: first.fov, near: 0.5, far: 220 }}
      gl={{ antialias: true, powerPreference: 'high-performance', toneMappingExposure: 1.25 }}
    >
      <WorldContext.Provider value={ctx}>
        <Scene {...props} />
      </WorldContext.Provider>
    </Canvas>
  );
}
