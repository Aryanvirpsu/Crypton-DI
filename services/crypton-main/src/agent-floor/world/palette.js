import * as THREE from 'three';

/**
 * The Agent Floor tokens (see the `.af` block in agentFloor.css), mirrored for WebGL.
 * The 3D world uses these six colours and nothing else; light and shading do the rest.
 */
import { C } from './colors';
export { C };

const cache = new Map();

/** Shared materials, created once per (kind, colour). */
export function mat(color, kind = 'matte') {
  const key = `${kind}:${color}`;
  if (cache.has(key)) return cache.get(key);
  let m;
  if (kind === 'glow') {
    m = new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 1.6, roughness: 0.4, toneMapped: false });
  } else if (kind === 'metal') {
    m = new THREE.MeshStandardMaterial({ color, roughness: 0.38, metalness: 0.55 });
  } else if (kind === 'gloss') {
    m = new THREE.MeshStandardMaterial({ color, roughness: 0.28, metalness: 0.08 });
  } else {
    m = new THREE.MeshStandardMaterial({ color, roughness: 0.72, metalness: 0.06 });
  }
  cache.set(key, m);
  return m;
}
