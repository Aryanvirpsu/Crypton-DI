import React, { useEffect, useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { RoundedBox } from '@react-three/drei';
import * as THREE from 'three';
import { C, mat } from './palette';
import { BLOCKING, SLAB, cueBump, cueState } from './choreography';
import { useWorld } from './worldContext';

/**
 * Scout, Relay and Courier: stylized miniature industrial robots built from primitives.
 * Each model faces +z and animates its own parts. They share one vocabulary:
 *   pose          what the scene asks of them (idle, scan, listen, receive, send, roll)
 *   speedRef      current ground speed, for treads and wheels
 *   attentionRef  0..1, rises while the robot is hovered, focused or selected
 *   step          the scene, so each robot can react to its handoff cue (choreography.CUES)
 *
 * Personality, kept restrained:
 *   Scout    curious: glances, blinks, widens its iris, nods when it is given or gives work
 *   Relay    coordinated: precise arms, a five-light signal bar, a double-blink antenna
 *   Courier  purposeful: heavy, settles on its suspension, lights up once it has the request
 *
 * Idle motion runs on world time, so it freezes with the held frame. Acknowledgements run on
 * real time, so a robot still answers a hover while the frame is held. Under reduced motion
 * nothing loops or nods; states simply change.
 *
 * To replace a model with a Blender GLB, swap its entry in ROBOT_MODELS: the controller only
 * needs a component that accepts { pose, speedRef, attentionRef, step }.
 */

const damp = THREE.MathUtils.damp;
const clamp = THREE.MathUtils.clamp;

function useLed(color) {
  // Each robot gets its own LED material so changing one does not change every LED.
  return useMemo(() => mat(color, 'glow').clone(), [color]);
}

/** A one-shot 0..1..0 bump when attention first rises. Real time, so it answers while frozen. */
function useAcknowledge(attentionRef, reduced) {
  const st = useRef({ prev: 0, t: 9 });
  return (dt) => {
    const s = st.current;
    const a = attentionRef.current;
    if (a > 0.5 && s.prev <= 0.5) s.t = 0;
    s.prev = a;
    s.t += dt;
    if (reduced || s.t > 0.6) return 0;
    return Math.sin((s.t / 0.6) * Math.PI);
  };
}

// ── Scout: compact research robot. Big optical head, tracked base. ──────────────────────

const SCOUT_WHEELS = [-0.2, 0, 0.2];

function ScoutModel({ pose, speedRef, attentionRef, step }) {
  const world = useWorld();
  const { time, reduced } = world;
  const head = useRef();
  const lid = useRef();
  const iris = useRef();
  const antenna = useRef();
  const cone = useRef();
  const wheels = useRef([]);
  const led = useLed(C.lime);
  const ball = useLed(C.lime);
  const ack = useAcknowledge(attentionRef, reduced);
  const bone = mat(C.bone, 'gloss');
  const machine = mat(C.machine);
  const graphite = mat(C.graphite, 'gloss');
  const mist = mat(C.mist, 'metal');

  useFrame((_, rawDt) => {
    const dt = Math.min(rawDt, 0.05);
    const t = time.current;
    const sceneT = world.sceneTime.current;
    const bump = (key, w) => (reduced ? 0 : cueBump(step, key, sceneT, w));
    const got = bump('receive', 0.6); // given work (the task card, scene 1)
    const gave = step === 3 ? bump('give', 0.6) : 0; // hands work on (scene 3)
    const att = attentionRef.current;
    const nod = ack(dt);
    const moving = speedRef.current > 0.05;

    // head: glances while idle, sweeps while scanning, still while listening
    let pitch = 0.03 * Math.sin(t * 0.5);
    let yaw = reduced ? 0 : Math.sin(t * 0.7) * 0.25 + Math.sin(t * 0.23 + 1) * 0.18;
    if (pose === 'scan') { pitch = 0.42; yaw = reduced ? 0 : Math.sin(t * 2.2) * 0.18; }
    if (pose === 'listen') { pitch = -0.2; yaw = 0; }
    if (moving) { pitch = 0.08; yaw = 0; }
    pitch += -0.16 * att + 0.2 * nod + 0.2 * got + 0.16 * gave;
    const h = head.current;
    h.rotation.x = damp(h.rotation.x, pitch, 6, dt);
    h.rotation.y = damp(h.rotation.y, yaw, 6, dt);
    h.position.y = 0.7 + (reduced ? 0 : Math.sin(t * 2.4) * 0.012) + 0.03 * att;

    // eye: iris widens on attention and on a handoff; blinks now and then; lens flickers while scanning
    const scanning = pose === 'scan' && !reduced;
    const k = 1 + 0.3 * att + 0.4 * Math.max(got, gave) + (scanning ? 0.12 + Math.sin(t * 9) * 0.1 : 0);
    iris.current.scale.set(k, 1, k);
    led.emissiveIntensity = 1.5 + 0.8 * att + 1.2 * Math.max(got, gave) + (scanning ? Math.sin(t * 9) * 0.5 : 0);
    const bp = t % 5.7;
    lid.current.scale.y = !reduced && !scanning && bp < 0.14 ? Math.max(0.0001, Math.sin((bp / 0.14) * Math.PI)) : 0.0001;

    // antenna: a slow sway; the ball flashes when work is handed over or received
    antenna.current.rotation.z = reduced ? 0 : Math.sin(t * 1.6) * 0.07;
    ball.emissiveIntensity = 1.2 + 2.4 * Math.max(got, gave) + 0.8 * att;

    // scan beam only while reading the designated report
    const c = cone.current;
    c.scale.setScalar(damp(c.scale.x, pose === 'scan' ? 1 : 0.0001, 7, dt));

    // treads turn with the ground speed
    const spin = speedRef.current * dt * world.timeScale.current * 8;
    wheels.current.forEach((w) => { if (w) w.rotation.x += spin; });
  });

  return (
    <group>
      {[-1, 1].map((side) => (
        <group key={side} position={[0.19 * side, 0, 0]}>
          <RoundedBox args={[0.17, 0.2, 0.58]} radius={0.07} position={[0, 0.1, 0]} material={machine} castShadow />
          {SCOUT_WHEELS.map((z, i) => (
            <mesh
              key={z}
              ref={(el) => { wheels.current[(side > 0 ? 3 : 0) + i] = el; }}
              position={[0.014 * side, 0.095, z]}
              rotation={[0, 0, Math.PI / 2]}
              material={mist}
              castShadow
            >
              <cylinderGeometry args={[0.08, 0.08, 0.19, 14]} />
            </mesh>
          ))}
        </group>
      ))}
      <RoundedBox args={[0.5, 0.3, 0.46]} radius={0.1} position={[0, 0.34, 0]} material={bone} castShadow />
      <mesh position={[0, 0.36, 0.235]} material={graphite}>
        <boxGeometry args={[0.24, 0.06, 0.01]} />
      </mesh>
      <mesh position={[0, 0.5, 0]} rotation={[Math.PI / 2, 0, 0]} material={graphite}>
        <torusGeometry args={[0.1, 0.022, 8, 20]} />
      </mesh>
      <mesh position={[0, 0.54, 0]} material={machine} castShadow>
        <cylinderGeometry args={[0.06, 0.08, 0.14, 12]} />
      </mesh>
      <group ref={head} position={[0, 0.7, 0]}>
        <mesh material={bone} castShadow>
          <sphereGeometry args={[0.25, 32, 24]} />
        </mesh>
        <mesh rotation={[Math.PI / 2, 0, 0]} material={machine}>
          <torusGeometry args={[0.248, 0.016, 8, 40]} />
        </mesh>
        {[-1, 1].map((side) => (
          <group key={side} position={[0.245 * side, 0, 0]} rotation={[0, 0, Math.PI / 2]}>
            <mesh material={mist}>
              <cylinderGeometry args={[0.05, 0.05, 0.05, 14]} />
            </mesh>
            <mesh position={[0, 0.03 * side, 0]} material={graphite}>
              <cylinderGeometry args={[0.025, 0.025, 0.012, 10]} />
            </mesh>
          </group>
        ))}
        <mesh position={[0, 0.02, 0.2]} rotation={[Math.PI / 2, 0, 0]} material={graphite}>
          <cylinderGeometry args={[0.145, 0.15, 0.1, 28]} />
        </mesh>
        <mesh ref={iris} position={[0, 0.02, 0.255]} rotation={[Math.PI / 2, 0, 0]} material={led}>
          <cylinderGeometry args={[0.065, 0.065, 0.02, 24]} />
        </mesh>
        {/* eyelid: a graphite plate that drops over the lens for a blink */}
        <group position={[0, 0.09, 0.262]}>
          <mesh ref={lid} position={[0, -0.07, 0]} scale={[1, 0.0001, 1]} material={graphite}>
            <boxGeometry args={[0.16, 0.14, 0.012]} />
          </mesh>
        </group>
        <group ref={antenna} position={[0.15, 0.22, -0.04]}>
          <mesh position={[0.02, 0.05, 0]} rotation={[0, 0, -0.35]} material={machine}>
            <cylinderGeometry args={[0.012, 0.012, 0.2, 6]} />
          </mesh>
          <mesh position={[0.06, 0.15, 0]} material={ball}>
            <sphereGeometry args={[0.03, 12, 8]} />
          </mesh>
        </group>
        {/* scan beam */}
        <mesh ref={cone} position={[0, 0.02, 0.6]} rotation={[-Math.PI / 2, 0, 0]} scale={0.0001}>
          <coneGeometry args={[0.3, 0.78, 28, 1, true]} />
          <meshBasicMaterial color={C.lime} transparent opacity={0.3} depthWrite={false} side={THREE.DoubleSide} blending={THREE.AdditiveBlending} />
        </mesh>
      </group>
    </group>
  );
}

// ── Relay: graphite coordinator with two articulated arms and a signal bar. ─────────────

function Arm({ side, armRef, foreRef, fingerA, fingerB, material, joint }) {
  return (
    <group ref={armRef} position={[0.4 * side, 0.82, 0]}>
      <mesh material={joint}>
        <sphereGeometry args={[0.075, 16, 12]} />
      </mesh>
      {/* shoulder guard */}
      <mesh position={[0.02 * side, 0.04, 0]} material={material} castShadow>
        <boxGeometry args={[0.1, 0.05, 0.13]} />
      </mesh>
      <mesh position={[0, -0.16, 0]} material={material} castShadow>
        <boxGeometry args={[0.085, 0.3, 0.085]} />
      </mesh>
      <group ref={foreRef} position={[0, -0.32, 0]}>
        <mesh material={joint}>
          <sphereGeometry args={[0.055, 12, 10]} />
        </mesh>
        <mesh position={[0, -0.14, 0]} material={material} castShadow>
          <boxGeometry args={[0.07, 0.26, 0.07]} />
        </mesh>
        <mesh position={[0, -0.27, 0]} material={joint}>
          <boxGeometry args={[0.1, 0.03, 0.06]} />
        </mesh>
        <group ref={fingerA} position={[0, -0.28, 0]}>
          <mesh position={[0.026, -0.045, 0]} material={joint}>
            <boxGeometry args={[0.022, 0.09, 0.05]} />
          </mesh>
        </group>
        <group ref={fingerB} position={[0, -0.28, 0]}>
          <mesh position={[-0.026, -0.045, 0]} material={joint}>
            <boxGeometry args={[0.022, 0.09, 0.05]} />
          </mesh>
        </group>
      </group>
    </group>
  );
}

// Shoulder pitch (x) and elbow pitch per pose, [left, right]. Negative pitch raises the arm forward.
const RELAY_ARMS = {
  idle: [[0.08, -0.2], [0.08, -0.2]],
  receive: [[-1.05, -0.55], [-1.05, -0.55]],
  hold: [[-0.6, -1.05], [-0.6, -1.05]],
  send: [[0.1, -0.15], [-1.45, -0.1]],
};

function RelayModel({ pose, attentionRef, step }) {
  const world = useWorld();
  const { time, reduced } = world;
  const armL = useRef();
  const armR = useRef();
  const foreL = useRef();
  const foreR = useRef();
  const fingers = useRef([]);
  const gripVal = useRef(0.2);
  const head = useRef();
  const bars = useMemo(() => Array.from({ length: 5 }, () => mat(C.lime, 'glow').clone()), []);
  const tip = useLed(C.lime);
  const visor = useLed(C.lime);
  const ack = useAcknowledge(attentionRef, reduced);
  const machine = mat(C.machine, 'metal');
  const graphite = mat(C.graphite, 'gloss');
  const joint = mat(C.mist, 'metal');

  useFrame((_, rawDt) => {
    const dt = Math.min(rawDt, 0.05);
    const t = time.current;
    const sceneT = world.sceneTime.current;
    const bump = (key, w) => (reduced ? 0 : cueBump(step, key, sceneT, w));
    const att = attentionRef.current;
    const nod = ack(dt);

    // what the arms are doing: open to take work, close and carry it, open again to pass it on
    let p = pose;
    let grip = 0.2;
    if (step === 3) {
      const got = cueState(3, 3, 'receive', sceneT);
      p = got ? 'hold' : 'receive';
      grip = got ? 0.03 : 0.55;
    } else if (step >= 4) {
      p = 'send';
      grip = cueState(step, 4, 'give', sceneT) ? 0.55 : 0.08;
    }
    const [[sl, el], [sr, er]] = RELAY_ARMS[p] || RELAY_ARMS.idle;
    const sway = reduced ? 0 : Math.sin(t * 1.3) * 0.05;
    armL.current.rotation.x = damp(armL.current.rotation.x, sl + sway - 0.12 * nod, 5, dt);
    armR.current.rotation.x = damp(armR.current.rotation.x, sr - sway - 0.12 * nod, 5, dt);
    foreL.current.rotation.x = damp(foreL.current.rotation.x, el, 5, dt);
    foreR.current.rotation.x = damp(foreR.current.rotation.x, er, 5, dt);
    const g = damp(gripVal.current, grip, 9, dt);
    gripVal.current = g;
    fingers.current.forEach((f, i) => { if (f) f.rotation.z = (i % 2 ? -1 : 1) * g; });

    // head: precise, slow scanning; settles on whoever it is working with
    head.current.rotation.y = damp(head.current.rotation.y, reduced ? 0 : Math.sin(t * 0.5) * 0.2, 4, dt);
    head.current.rotation.x = damp(head.current.rotation.x, 0.14 * nod - 0.05 * att, 6, dt);

    // signal bar: a slow wave while idle, a chase when work arrives or leaves, steady once holding
    const flash = Math.max(bump('receive', 0.5), step === 4 ? bump('give', 0.5) : 0);
    const chaseIdx = (t * 9) % 5;
    const holding = step === 3 && cueState(3, 3, 'receive', sceneT);
    bars.forEach((m, i) => {
      let v = reduced ? 0.7 : 0.45 + 0.3 * Math.sin(t * 1.2 - i * 0.8);
      if (step === 3 && !holding) v = reduced ? 0.7 : Math.abs(i - chaseIdx) < 1 ? 1.8 : 0.35;
      if (holding || step >= 5) v = 1.1;
      if (step === 4) v = Math.abs(i - ((sceneT * 10) % 5)) < 1 && sceneT < 1 ? 1.8 : 0.9;
      m.emissiveIntensity = v + 1.4 * flash + 0.5 * att;
    });
    visor.emissiveIntensity = 1.4 + 0.5 * att + 0.8 * flash;

    // antenna tip: a double blink every four seconds, quick flashes while signalling
    const bp = t % 4;
    const dbl = bp < 0.1 || (bp > 0.22 && bp < 0.32) ? 2.4 : 0.3;
    tip.emissiveIntensity = reduced ? 1.2 : Math.max(dbl, 2.6 * flash);
  });

  return (
    <group>
      <mesh position={[0, 0.07, 0]} material={graphite} castShadow receiveShadow>
        <cylinderGeometry args={[0.36, 0.4, 0.14, 32]} />
      </mesh>
      <mesh position={[0, 0.27, 0]} material={machine} castShadow>
        <cylinderGeometry args={[0.13, 0.17, 0.28, 16]} />
      </mesh>
      {/* hydraulic struts either side of the waist */}
      {[-1, 1].map((x) => (
        <mesh key={x} position={[0.22 * x, 0.3, -0.04]} material={joint}>
          <cylinderGeometry args={[0.022, 0.022, 0.3, 8]} />
        </mesh>
      ))}
      <RoundedBox args={[0.66, 0.52, 0.46]} radius={0.08} position={[0, 0.66, 0]} material={machine} castShadow />
      {/* signal bar: five lights */}
      {bars.map((m, i) => (
        <mesh key={i} position={[(i - 2) * 0.085, 0.8, 0.235]} material={m}>
          <boxGeometry args={[0.065, 0.04, 0.01]} />
        </mesh>
      ))}
      <mesh position={[0, 0.6, 0.232]} material={graphite}>
        <boxGeometry args={[0.32, 0.16, 0.01]} />
      </mesh>
      <group ref={head} position={[0, 1.04, 0]}>
        <RoundedBox args={[0.36, 0.24, 0.32]} radius={0.06} material={machine} castShadow />
        <mesh position={[0, 0.01, 0.162]} material={visor}>
          <boxGeometry args={[0.24, 0.06, 0.01]} />
        </mesh>
        <mesh position={[0.1, 0.17, -0.04]} material={joint}>
          <cylinderGeometry args={[0.01, 0.01, 0.12, 6]} />
        </mesh>
        <mesh position={[0.1, 0.24, -0.04]} material={tip}>
          <sphereGeometry args={[0.025, 10, 8]} />
        </mesh>
      </group>
      <Arm side={-1} armRef={armL} foreRef={foreL} fingerA={(el) => { fingers.current[0] = el; }} fingerB={(el) => { fingers.current[1] = el; }} material={machine} joint={joint} />
      <Arm side={1} armRef={armR} foreRef={foreR} fingerA={(el) => { fingers.current[2] = el; }} fingerB={(el) => { fingers.current[3] = el; }} material={machine} joint={joint} />
    </group>
  );
}

// ── Courier: low wheeled cart with a cargo tray and a beacon. Heavy, deliberate. ────────

const WHEELS = [[-0.34, 0.33], [0.34, 0.33], [-0.34, -0.33], [0.34, -0.33]];

function CourierModel({ pose, speedRef, attentionRef, step }) {
  const world = useWorld();
  const { time, reduced, timeScale } = world;
  const chassis = useRef();
  const wheels = useRef([]);
  const beacon = useRef();
  const lidar = useRef();
  const prevSpeed = useRef(0);
  const coral = useLed(C.coral);
  const lamp = useLed(C.lime);
  const ack = useAcknowledge(attentionRef, reduced);
  const machine = mat(C.machine, 'metal');
  const graphite = mat(C.graphite, 'gloss');
  const mist = mat(C.mist, 'metal');
  const tint = mat(C.lime, 'glow');

  useFrame((_, rawDt) => {
    const dt = Math.min(rawDt, 0.05);
    const t = time.current;
    const sceneT = world.sceneTime.current;
    const att = attentionRef.current;
    const nod = ack(dt);
    const got = step >= 4 && cueState(step, 4, 'receive', sceneT); // it has been handed the request
    const landing = reduced || step !== 4 ? 0 : cueBump(4, 'receive', sceneT, 0.5);

    // wheels roll without slipping; the chassis settles on its springs as it speeds up and slows
    const spin = speedRef.current * dt * timeScale.current * 7;
    wheels.current.forEach((w) => { if (w) w.rotation.x += spin; });
    const accel = (speedRef.current - prevSpeed.current) / Math.max(dt, 1e-3);
    prevSpeed.current = speedRef.current;
    const c = chassis.current;
    const pitch = reduced ? 0 : clamp(-accel * 0.01, -0.045, 0.045) + 0.035 * nod;
    c.rotation.x = damp(c.rotation.x, pitch, 3.2, dt);
    c.position.y = damp(c.position.y, reduced ? 0 : -0.03 * landing - 0.012 * att, 5, dt);

    // beacon and lidar: slow while waiting, quick and coral-bright once it has the request
    const spinRate = (got ? 6 : pose === 'roll' ? 3 : 1.1) * (1 + att);
    beacon.current.rotation.y = reduced ? 0 : t * spinRate;
    lidar.current.rotation.y = reduced ? 0 : t * 2.4;
    coral.emissiveIntensity = got ? (reduced ? 2.4 : 1.4 + Math.max(0, Math.sin(t * 8)) * 1.8) : 1 + 0.5 * att;
    // headlamp: dim at rest, lit once it is working
    lamp.emissiveIntensity = (got ? 2.2 : 0.5) + 0.8 * att;
  });

  return (
    <group>
      <group ref={chassis}>
        <RoundedBox args={[0.64, 0.26, 1.02]} radius={0.08} position={[0, 0.3, 0]} material={machine} castShadow />
        {[-0.325, 0.325].map((x) => (
          <mesh key={x} position={[x, 0.32, 0]} material={tint}>
            <boxGeometry args={[0.01, 0.035, 0.7]} />
          </mesh>
        ))}
        {/* tray with a rail either side */}
        <mesh position={[0, 0.465, 0.02]} material={mist} castShadow receiveShadow>
          <boxGeometry args={[0.58, 0.035, 0.84]} />
        </mesh>
        {[-0.29, 0.29].map((x) => (
          <mesh key={x} position={[x, 0.51, 0.02]} material={graphite}>
            <boxGeometry args={[0.03, 0.06, 0.84]} />
          </mesh>
        ))}
        {/* front: bumper, headlamp, lidar puck */}
        <mesh position={[0, 0.2, 0.53]} material={graphite} castShadow>
          <boxGeometry args={[0.56, 0.09, 0.05]} />
        </mesh>
        <mesh position={[0, 0.32, 0.515]} material={lamp}>
          <boxGeometry args={[0.36, 0.04, 0.01]} />
        </mesh>
        <group ref={lidar} position={[-0.2, 0.52, 0.34]}>
          <mesh material={graphite}>
            <cylinderGeometry args={[0.06, 0.07, 0.06, 14]} />
          </mesh>
          <mesh position={[0, 0.02, 0.055]} material={tint}>
            <boxGeometry args={[0.05, 0.012, 0.012]} />
          </mesh>
        </group>
        {/* mudguards */}
        {WHEELS.map(([x, z], i) => (
          <mesh key={i} position={[x * 1.05, 0.26, z]} material={machine} castShadow>
            <boxGeometry args={[0.14, 0.025, 0.3]} />
          </mesh>
        ))}
        <mesh position={[0.2, 0.62, -0.4]} material={graphite}>
          <cylinderGeometry args={[0.018, 0.018, 0.3, 8]} />
        </mesh>
        <group ref={beacon} position={[0.2, 0.8, -0.4]}>
          <mesh material={coral}>
            <cylinderGeometry args={[0.06, 0.07, 0.08, 16]} />
          </mesh>
          <mesh position={[0.05, 0, 0]} material={graphite}>
            <boxGeometry args={[0.02, 0.09, 0.09]} />
          </mesh>
        </group>
      </group>
      {WHEELS.map(([x, z], i) => (
        <group key={i} position={[x, 0.14, z]}>
          <mesh ref={(el) => { wheels.current[i] = el; }} rotation={[0, 0, Math.PI / 2]} material={graphite} castShadow>
            <cylinderGeometry args={[0.14, 0.14, 0.1, 20]} />
          </mesh>
          <mesh rotation={[0, 0, Math.PI / 2]} position={[x > 0 ? 0.052 : -0.052, 0, 0]} material={mist}>
            <cylinderGeometry args={[0.06, 0.06, 0.01, 12]} />
          </mesh>
        </group>
      ))}
    </group>
  );
}

export const ROBOT_MODELS = { 'AG-SCOUT': ScoutModel, 'AG-RELAY': RelayModel, 'AG-COURIER': CourierModel };
export const ROBOT_HEIGHT = { 'AG-SCOUT': 1.15, 'AG-RELAY': 1.35, 'AG-COURIER': 1.0 };

// ── controller: moves a robot to its blocking for the current scene ──────────────────

// Scout is quick and light; Relay is measured; Courier is heavy and slow to start and stop.
const TUNING = {
  'AG-SCOUT': { max: 1.8, accel: 4.5, brake: 8 },
  'AG-RELAY': { max: 1.6, accel: 4, brake: 7 },
  'AG-COURIER': { max: 1.35, accel: 2.4, brake: 4.2 },
};
const shortestAngle = (a) => Math.atan2(Math.sin(a), Math.cos(a));

/**
 * Drives one robot toward BLOCKING[step][id], tracks hover / focus / selection as an
 * `attention` value for the model, and draws the selection ring. Motion runs on world time,
 * so when the frame is held the robot stops exactly where it is.
 */
export function Robot({ id, step, selected = false, onSelect, children }) {
  const world = useWorld();
  const group = useRef();
  const speedRef = useRef(0);
  const attention = useRef(0);
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const ring = useRef();
  const ping = useRef();
  const pingT = useRef(9);
  const Model = ROBOT_MODELS[id];
  const tune = TUNING[id];
  const block = BLOCKING[step][id];

  useEffect(() => {
    const g = group.current;
    world.robots.set(id, g);
    return () => world.robots.delete(id);
  }, [id, world.robots]);

  // a ping when the visitor selects this robot
  useEffect(() => { if (selected && !world.reduced) pingT.current = 0; }, [selected, world.reduced]);

  // Start in place on first mount (no fly-in from the origin).
  const initial = useRef(block);
  const facing = (from, to) => Math.atan2(to[0] - from[0], to[1] - from[1]);

  useFrame((_, rawDt) => {
    const real = Math.min(rawDt, 0.05);
    const g = group.current;
    const dt = real * world.timeScale.current;
    const [tx, tz] = block.at;
    const dx = tx - g.position.x;
    const dz = tz - g.position.z;
    const dist = Math.hypot(dx, dz);
    let heading = facing(block.at, block.look);

    // attention follows hover, keyboard focus (via the tag) and selection, in real time
    const want = world.hover.has(id) || selectedRef.current ? 1 : 0;
    // under reduced motion the answer is a state change, not an animation
    const k = world.reduced ? 1 : real;
    attention.current = damp(attention.current, want, 8, k);

    // selection ring and ping
    const sel = selectedRef.current ? 1 : 0;
    const hov = world.hover.has(id) ? 1 : 0;
    if (ring.current) {
      ring.current.material.opacity = damp(ring.current.material.opacity, sel * 0.9 + (1 - sel) * hov * 0.35, 10, k);
      ring.current.visible = ring.current.material.opacity > 0.01;
    }
    pingT.current += real;
    if (ping.current) {
      const p = clamp(pingT.current / 0.9, 0, 1);
      ping.current.visible = p < 1;
      ping.current.scale.setScalar(0.7 + p * 1.1);
      ping.current.material.opacity = (1 - p) * 0.6;
    }

    if (world.reduced) {
      g.position.set(tx, SLAB.height, tz);
      g.rotation.y = heading;
      speedRef.current = 0;
      return;
    }
    if (dist > 0.02) {
      // ease in from rest, ease out on arrival
      const wantSpeed = Math.min(tune.max, dist * 2.2 + 0.15);
      speedRef.current = damp(speedRef.current, wantSpeed, tune.accel, dt || 1e-6);
      const stepLen = Math.min(dist, speedRef.current * dt);
      g.position.x += (dx / dist) * stepLen;
      g.position.z += (dz / dist) * stepLen;
      if (dist > 0.25) heading = Math.atan2(dx, dz);
    } else {
      speedRef.current = damp(speedRef.current, 0, tune.brake, dt || 1e-6);
    }
    g.rotation.y += shortestAngle(heading - g.rotation.y) * (1 - Math.exp(-6 * dt));
  });

  const [ix, iz] = initial.current.at;
  return (
    <group
      ref={group}
      position={[ix, SLAB.height, iz]}
      rotation={[0, facing(initial.current.at, initial.current.look), 0]}
      onClick={(e) => { e.stopPropagation(); onSelect(id); }}
      onPointerOver={(e) => { e.stopPropagation(); world.hover.add(id); document.body.style.cursor = 'pointer'; }}
      onPointerOut={() => { world.hover.delete(id); document.body.style.cursor = ''; }}
    >
      <Model pose={block.pose} speedRef={speedRef} attentionRef={attention} step={step} />
      {/* selection ring and ping, flat on the floor under the robot */}
      <mesh ref={ring} position={[0, 0.012, 0]} rotation={[-Math.PI / 2, 0, 0]} visible={false}>
        <ringGeometry args={[0.62, 0.68, 40]} />
        <meshBasicMaterial color={C.lime} transparent opacity={0} depthWrite={false} />
      </mesh>
      <mesh ref={ping} position={[0, 0.014, 0]} rotation={[-Math.PI / 2, 0, 0]} visible={false}>
        <ringGeometry args={[0.62, 0.66, 40]} />
        <meshBasicMaterial color={C.lime} transparent opacity={0} depthWrite={false} />
      </mesh>
      {children}
    </group>
  );
}

