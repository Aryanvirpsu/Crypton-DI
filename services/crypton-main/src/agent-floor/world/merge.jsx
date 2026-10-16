import React, { useMemo } from 'react';
import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';
import { RoundedBoxGeometry } from 'three/examples/jsm/geometries/RoundedBoxGeometry.js';
import { mat } from './palette';

/**
 * Static scenery, merged by material. A workstation is dozens of small boxes; drawn one by
 * one that is dozens of draw calls. Declared as parts and merged here, a whole station is one
 * draw call per material, with no change to how it looks.
 *
 * A part: { g: 'box' | 'rbox' | 'cyl' | 'sphere' | 'torus' | 'plane' | 'geo',
 *           a: constructor args, p: [x, y, z], r: [rx, ry, rz], s: [sx, sy, sz],
 *           m: [colour, 'matte' | 'metal' | 'gloss' | 'glow'] }
 * `base` places the whole group: { pos: [x, y, z], yaw }.
 */

const KEEP = ['position', 'normal', 'uv'];

function geometryFor(part) {
  const a = part.a;
  switch (part.g) {
    case 'box': return new THREE.BoxGeometry(...a);
    case 'rbox': return new RoundedBoxGeometry(a[0], a[1], a[2], 2, a[3] === undefined ? 0.04 : a[3]);
    case 'cyl': return new THREE.CylinderGeometry(...a);
    case 'sphere': return new THREE.SphereGeometry(...a);
    case 'torus': return new THREE.TorusGeometry(...a);
    case 'plane': return new THREE.PlaneGeometry(...a);
    case 'geo': return a[0].clone();
    default: throw new Error(`unknown part ${part.g}`);
  }
}

/** Merge several placed groups of parts into one mesh per material, across all of them. */
export function mergeGroups(placed) {
  const groups = new Map();
  placed.forEach(({ parts, base = {} }) => {
    const baseM = new THREE.Matrix4().compose(
      new THREE.Vector3(...(base.pos || [0, 0, 0])),
      new THREE.Quaternion().setFromEuler(new THREE.Euler(0, base.yaw || 0, 0)),
      new THREE.Vector3(1, 1, 1),
    );
    parts.forEach((part) => {
      let g = geometryFor(part);
      const m = new THREE.Matrix4().compose(
        new THREE.Vector3(...(part.p || [0, 0, 0])),
        new THREE.Quaternion().setFromEuler(new THREE.Euler(...(part.r || [0, 0, 0]))),
        new THREE.Vector3(...(part.s || [1, 1, 1])),
      );
      g.applyMatrix4(baseM.clone().multiply(m));
      if (g.index) g = g.toNonIndexed();
      Object.keys(g.attributes).forEach((k) => { if (!KEEP.includes(k)) g.deleteAttribute(k); });
      const key = `${part.m[0]}:${part.m[1]}`;
      if (!groups.has(key)) groups.set(key, { key, material: mat(part.m[0], part.m[1]), list: [] });
      groups.get(key).list.push(g);
    });
  });
  return [...groups.values()].map(({ key, material, list }) => ({ key, material, geometry: mergeGeometries(list, false) }));
}

export function mergeParts(parts, base = {}) {
  return mergeGroups([{ parts, base }]);
}

/**
 * Renders merged parts. Pass `groups` ([{ parts, base }]) to merge several placed groups
 * together, or `parts` + `base` for one. Memoised: declare the arguments at module level.
 */
export function Merged({ parts, base, groups, castShadow = true, receiveShadow = true }) {
  const meshes = useMemo(() => (groups ? mergeGroups(groups) : mergeParts(parts, base)), [parts, base, groups]);
  return (
    <>
      {meshes.map((m) => (
        <mesh key={m.key} geometry={m.geometry} material={m.material} castShadow={castShadow} receiveShadow={receiveShadow} />
      ))}
    </>
  );
}
