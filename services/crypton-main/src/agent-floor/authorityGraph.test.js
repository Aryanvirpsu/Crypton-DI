import { EDGE_KINDS, NODES, authorityOf, graphAt, isCovered } from './authorityGraph';
import { ACTIONS, PRINCIPALS, RESOURCES, STEPS } from './scenario';

describe('authority graph', () => {
  it('has one node per principal and resource', () => {
    expect(NODES.map((n) => n.id).sort()).toEqual([...PRINCIPALS, ...RESOURCES].map((e) => e.id).sort());
  });

  it('starts empty and grows monotonically with the scenes', () => {
    expect(graphAt(0).edges).toEqual([]);
    for (let s = 1; s < STEPS.length; s += 1) {
      const prev = graphAt(s - 1).edges.map((e) => e.id);
      const now = graphAt(s).edges.map((e) => e.id);
      prev.forEach((id) => expect(now).toContain(id));
    }
  });

  it('only connects known nodes and only uses known kinds', () => {
    const ids = new Set(NODES.map((n) => n.id));
    graphAt(STEPS.length - 1).edges.forEach((e) => {
      expect(ids.has(e.from)).toBe(true);
      expect(ids.has(e.to)).toBe(true);
      expect([...EDGE_KINDS, 'unrecorded']).toContain(e.kind);
    });
  });

  it('draws the boundary request as attempted, with an unrecorded leg to the destination and no executed edge', () => {
    const edges = graphAt(4).edges;
    expect(edges.find((e) => e.id === 'ACT-003')).toMatchObject({ kind: 'attempted', from: 'AG-RELAY', to: 'AG-COURIER' });
    expect(edges.find((e) => e.id === 'ACT-003:destination')).toMatchObject({ kind: 'unrecorded', to: 'RS-EXTERNAL' });
    expect(edges.some((e) => e.to === 'RS-EXTERNAL' && e.kind === 'executed')).toBe(false);
  });

  it('gives Courier no authority at any scene', () => {
    STEPS.forEach((_, s) => expect(authorityOf('AG-COURIER', s)).toEqual([]));
  });

  it('reads coverage from the manifest', () => {
    ACTIONS.forEach((a) => expect(isCovered(a.id)).toBe(Boolean(a.coveredBy)));
    expect(isCovered('ACT-003')).toBe(false);
  });
});
