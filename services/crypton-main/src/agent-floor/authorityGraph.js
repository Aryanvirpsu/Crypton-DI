import { ACTIONS, PRINCIPALS, RESOURCES, visibleAt } from './scenario';

/**
 * The authority graph at a scene: nodes and typed edges derived from scenario.js alone.
 *
 * Every renderer (the 2D Atlas, the 3D world, a future 3D Authority Atlas) draws THIS,
 * so they can never disagree about who holds what or what was asked. No renderer is
 * allowed to invent an edge.
 *
 * Edge kinds:
 *   grant      a principal granted authority to another (GR-*)
 *   delegation an agent passed part of its authority on (DL-*)
 *   executed   an action that ran, within a grant (illustrative)
 *   attempted  an action that was asked for; execution and enforcement are not recorded
 */

export const EDGE_KINDS = ['grant', 'delegation', 'executed', 'attempted'];

export const NODES = [
  ...PRINCIPALS.map((p) => ({ id: p.id, kind: p.kind === 'human' ? 'human' : 'agent' })),
  ...RESOURCES.map((r) => ({ id: r.id, kind: 'resource' })),
];

export function graphAt(step) {
  const vis = visibleAt(step);
  const edges = [
    ...vis.grants.map((g) => ({ id: g.id, kind: 'grant', from: g.from, to: g.to })),
    ...vis.delegations.map((d) => ({ id: d.id, kind: 'delegation', from: d.from, to: d.to })),
    ...vis.actions
      // ACT-002 is the same hop as DL-001; the delegation edge already draws it.
      .filter((a) => !a.via)
      .map((a) => ({
        id: a.id,
        kind: a.status === 'executed' ? 'executed' : 'attempted',
        from: a.actor,
        to: a.target || a.resource,
      })),
  ];
  // An attempted request with a destination gets a second, explicitly unresolved leg:
  // the request points there, but nothing records that anything arrived.
  vis.actions
    .filter((a) => a.status === 'attempted' && a.destination)
    .forEach((a) => edges.push({ id: `${a.id}:destination`, kind: 'unrecorded', from: a.target, to: a.destination, of: a.id }));
  return { nodes: NODES, edges };
}

/** Authority a principal holds at a scene (grants and delegations addressed to it). */
export function authorityOf(id, step) {
  const vis = visibleAt(step);
  return [...vis.grants.filter((g) => g.to === id), ...vis.delegations.filter((d) => d.to === id)];
}

/** True if any visible grant or delegation covers this action. Read from the manifest, never inferred. */
export function isCovered(actionId) {
  const a = ACTIONS.find((x) => x.id === actionId);
  return Boolean(a && a.coveredBy);
}
