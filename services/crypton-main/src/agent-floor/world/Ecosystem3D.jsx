import React, { useLayoutEffect, useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { C } from './palette';
import { SEAM_X, buildFloors, packetProgress, workerAt } from './ecosystem';
import { useWorld } from './worldContext';

/**
 * The far floors, as a handful of instanced meshes: slabs, furniture, workers, requests,
 * beacons and seams. Roughly six draw calls for all twelve floors.
 *
 * Activity runs on the LIVE clock, not the world clock: the main floor is held at the moment
 * of the request, but the rest of the system carries on. Workers circle at their own pace,
 * some floors nearly idle, and on a few floors an illustrated "attempted request" travels to
 * the boundary seam and waits there, with a beacon beyond it. Under reduced motion all of
 * it holds still.
 */

const dummy = new THREE.Object3D();
const color = new THREE.Color();

export default function Ecosystem3D({ visible }) {
  const world = useWorld();
  const group = useRef();
  const dots = useRef();
  const packets = useRef();
  const beacons = useRef();
  const floors = useMemo(buildFloors, []);
  const problem = useMemo(() => floors.filter((f) => f.problem), [floors]);
  const furniture = useMemo(() => floors.flatMap((f) => f.furniture.map((fu) => ({ f, ...fu }))), [floors]);
  const workerCount = useMemo(() => floors.reduce((n, f) => n + f.workers, 0), [floors]);
  const slabs = useRef();
  const furn = useRef();
  const seams = useRef();

  const place = (mesh, i, f, local, size) => {
    dummy.position.set(f.x + f.s * local[0], f.y + f.s * local[1], f.z + f.s * local[2]);
    dummy.scale.set(f.s * size[0], f.s * size[1], f.s * size[2]);
    dummy.rotation.set(0, 0, 0);
    dummy.updateMatrix();
    mesh.setMatrixAt(i, dummy.matrix);
  };

  // everything that never moves is written once
  useLayoutEffect(() => {
    floors.forEach((f, i) => {
      place(slabs.current, i, f, [0, 0, 0], [14, 0.6, 9]);
      slabs.current.setColorAt(i, color.set(C.machine).multiplyScalar(f.shade));
    });
    furniture.forEach((fu, i) => {
      place(furn.current, i, fu.f, fu.p, fu.size);
      furn.current.setColorAt(i, color.set(fu.color));
    });
    problem.forEach((f, i) => {
      place(seams.current, i, f, [SEAM_X, 0.33, 0], [0.12, 0.05, 8.4]);
      place(beacons.current, i, f, [SEAM_X + 2.7, 0.7, 0], [0.7, 0.7, 0.7]);
    });
    let n = 0;
    floors.forEach((f) => { for (let k = 0; k < f.workers; k += 1) { dots.current.setColorAt(n, color.set(C.lime)); n += 1; } });
    problem.forEach((_, i) => packets.current.setColorAt(i, color.set(C.coral)));
    // seams and beacons take their colour from the material; the rest carry per-instance colour
    [slabs, furn, seams, beacons, dots, packets].forEach((r) => {
      r.current.instanceMatrix.needsUpdate = true;
      if (r.current.instanceColor) r.current.instanceColor.needsUpdate = true;
    });
  }, [floors, furniture, problem]);

  useFrame((_, dt) => {
    const g = group.current;
    if (!g) return;
    const s = THREE.MathUtils.damp(g.scale.x, visible ? 1 : 0.001, 2.5, world.reduced ? 1 : Math.min(dt, 0.1));
    g.scale.setScalar(s);
    g.visible = s > 0.01;
    if (!g.visible) return;
    const t = world.reduced ? 0 : world.live.current;
    g.rotation.y = world.reduced ? 0 : t * 0.004;

    // workers, each floor at its own pace
    let n = 0;
    floors.forEach((f) => {
      for (let k = 0; k < f.workers; k += 1) {
        const [x, y, z] = workerAt(f, k, t);
        place(dots.current, n, f, [x, y, z], [0.8, 0.8, 0.8]);
        n += 1;
      }
    });
    dots.current.instanceMatrix.needsUpdate = true;

    // the illustrated request: travels toward the seam and waits there; the beacon beyond it breathes
    problem.forEach((f, i) => {
      const p = packetProgress(f, t);
      const x = THREE.MathUtils.lerp(0.5, SEAM_X - 0.35, p);
      const hold = p >= 1 ? 1 + 0.25 * Math.sin(t * 5 + i) : 1;
      place(packets.current, i, f, [x, 0.65, (i - 1.5) * 0.9], [0.4 * hold, 0.4 * hold, 0.4 * hold]);
      const b = 0.7 * (1 + 0.18 * Math.sin(t * 2.2 + i * 1.7));
      place(beacons.current, i, f, [SEAM_X + 2.7, 0.7, 0], [b, b, b]);
    });
    packets.current.instanceMatrix.needsUpdate = true;
    beacons.current.instanceMatrix.needsUpdate = true;
  });

  return (
    <group ref={group} scale={0.001}>
      <instancedMesh ref={slabs} args={[null, null, floors.length]} frustumCulled={false}>
        <boxGeometry args={[1, 1, 1]} />
        <meshStandardMaterial color="#ffffff" roughness={0.78} metalness={0.06} />
      </instancedMesh>
      <instancedMesh ref={furn} args={[null, null, furniture.length]} frustumCulled={false}>
        <boxGeometry args={[1, 1, 1]} />
        <meshStandardMaterial color="#ffffff" roughness={0.6} metalness={0.25} />
      </instancedMesh>
      <instancedMesh ref={dots} args={[null, null, workerCount]} frustumCulled={false}>
        <sphereGeometry args={[0.5, 10, 8]} />
        <meshBasicMaterial color="#ffffff" toneMapped={false} />
      </instancedMesh>
      <instancedMesh ref={seams} args={[null, null, problem.length]} frustumCulled={false}>
        <boxGeometry args={[1, 1, 1]} />
        <meshBasicMaterial color={C.coral} toneMapped={false} />
      </instancedMesh>
      <instancedMesh ref={packets} args={[null, null, problem.length]} frustumCulled={false}>
        <sphereGeometry args={[0.5, 10, 8]} />
        <meshBasicMaterial color="#ffffff" toneMapped={false} />
      </instancedMesh>
      <instancedMesh ref={beacons} args={[null, null, problem.length]} frustumCulled={false}>
        <sphereGeometry args={[0.5, 12, 10]} />
        <meshBasicMaterial color={C.coral} toneMapped={false} />
      </instancedMesh>
    </group>
  );
}
