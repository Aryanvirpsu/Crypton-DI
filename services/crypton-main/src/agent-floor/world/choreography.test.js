import { ATLAS_SHOT, BLOCKING, CUES, cueBump, cueState, BOUNDARY_X, ENVELOPES, FREEZE_STEP, SHOTS, STATIONS, insideEnvelope, nodeAt, shotFor } from './choreography';
import { DELEGATIONS, GRANTS, PRINCIPALS, RESOURCES, STEPS } from '../scenario';

const agents = PRINCIPALS.filter((p) => p.kind === 'agent').map((p) => p.id);

describe('world blocking agrees with the scenario', () => {
  it('has a shot and blocking for every scene, for every agent', () => {
    expect(SHOTS).toHaveLength(STEPS.length);
    expect(BLOCKING).toHaveLength(STEPS.length);
    BLOCKING.forEach((b) => agents.forEach((id) => expect(b[id]).toBeDefined()));
  });

  it('places every principal and resource somewhere on (or off) the floor', () => {
    [...PRINCIPALS, ...RESOURCES].forEach((e) => expect(nodeAt(e.id, 0)).not.toBeNull());
  });

  it('draws one envelope per grant and delegation, held by its recipient, and none for Courier', () => {
    expect(Object.keys(ENVELOPES).sort()).toEqual([...GRANTS, ...DELEGATIONS].map((g) => g.id).sort());
    [...GRANTS, ...DELEGATIONS].forEach((g) => expect(ENVELOPES[g.id].holder).toBe(g.to));
    expect(Object.values(ENVELOPES).some((e) => e.holder === 'AG-COURIER')).toBe(false);
  });

  it('puts the designated report inside the grant envelope and the records and destination outside every envelope', () => {
    expect(insideEnvelope('GR-001', STATIONS['RS-REPORT'].at)).toBe(true);
    Object.keys(ENVELOPES).forEach((id) => {
      expect(insideEnvelope(id, STATIONS['RS-RECORDS'].at)).toBe(false);
      expect(insideEnvelope(id, STATIONS['RS-EXTERNAL'].at)).toBe(false);
    });
  });

  it('keeps the external destination beyond the organisation boundary and every robot inside it', () => {
    expect(STATIONS['RS-EXTERNAL'].at[0]).toBeGreaterThan(BOUNDARY_X);
    BLOCKING.forEach((b) => agents.forEach((id) => expect(b[id].at[0]).toBeLessThan(BOUNDARY_X)));
  });

  it('never moves Courier off the dock: the request is shown, not an export', () => {
    BLOCKING.forEach((b) => expect(Math.abs(b['AG-COURIER'].at[0] - STATIONS.dock.at[0])).toBeLessThan(0.5));
  });

  it('holds the frame at the boundary scene', () => {
    expect(STEPS[FREEZE_STEP].title).toBe('Boundary');
  });

  it('has a shot for every scene in either view, and each scene keeps its own shot in its default view', () => {
    STEPS.forEach((st, i) => {
      expect(shotFor(i, st.defaultView, st.defaultView)).toBe(SHOTS[i]);
      ['floor', 'atlas'].forEach((v) => expect(shotFor(i, v, st.defaultView)).toBeDefined());
    });
    expect(shotFor(1, 'atlas', 'floor')).toBe(ATLAS_SHOT);
  });

  it('keeps every handoff receive after its give, and the boundary receive before the held frame', () => {
    Object.values(CUES).forEach((c) => expect(c.receive).toBeGreaterThan(c.give));
    expect(CUES[FREEZE_STEP].receive + 0.4).toBeLessThanOrEqual(1.9);
  });

  it('cue bumps rise and fall inside their window and are zero elsewhere', () => {
    expect(cueBump(3, 'receive', 0)).toBe(0);
    expect(cueBump(3, 'receive', CUES[3].receive + 0.3, 0.6)).toBeCloseTo(1, 5);
    expect(cueBump(3, 'receive', CUES[3].receive + 0.7, 0.6)).toBe(0);
    expect(cueBump(0, 'receive', 1)).toBe(0); // scenes without a handoff
  });

  it('cue state is monotonic across scenes', () => {
    expect(cueState(3, 3, 'receive', CUES[3].receive - 0.01)).toBe(false);
    expect(cueState(3, 3, 'receive', CUES[3].receive)).toBe(true);
    expect(cueState(2, 3, 'receive', 99)).toBe(false);
    expect(cueState(5, 3, 'receive', 0)).toBe(true);
  });
});
