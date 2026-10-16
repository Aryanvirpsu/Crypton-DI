import { STEPS, TECHNICAL_STEP } from './scenario';
import { LAST_STEP, initialState, reducer } from './state';

const run = (actions, from = initialState) => actions.reduce(reducer, from);
const next = { type: 'NEXT' };
const prev = { type: 'PREV' };

describe('scenario state transitions', () => {
  it('starts at scene 00 on the floor with nothing selected', () => {
    expect(initialState).toMatchObject({ step: 0, view: 'floor', selection: null });
  });

  it('next advances one scene at a time and stops at the last', () => {
    for (let i = 1; i <= LAST_STEP; i += 1) expect(run(Array(i).fill(next)).step).toBe(i);
    expect(run(Array(LAST_STEP + 5).fill(next)).step).toBe(LAST_STEP);
  });

  it('previous is the inverse of next and stops at the first', () => {
    for (let i = 1; i <= LAST_STEP; i += 1) {
      const there = run(Array(i).fill(next));
      expect(run([prev], there)).toEqual(run(Array(i - 1).fill(next)));
    }
    expect(run([prev, prev])).toEqual(initialState);
  });

  it('reset returns exactly to the initial state from anywhere', () => {
    const messy = run([next, next, next, { type: 'SELECT', id: 'AG-RELAY' }, { type: 'SET_VIEW', view: 'atlas' }, next]);
    expect(run([{ type: 'RESET' }], messy)).toEqual(initialState);
  });

  it('is deterministic: walking and jumping to a scene give the same state', () => {
    for (let i = 0; i <= LAST_STEP; i += 1) {
      expect(run([{ type: 'GOTO', step: i }])).toEqual(run(Array(i).fill(next)));
    }
  });

  it('ignores invalid jumps', () => {
    [-1, LAST_STEP + 1, 1.5, 'x', undefined].forEach((step) => expect(run([{ type: 'GOTO', step }])).toEqual(initialState));
  });

  it('skip-to-technical lands on the evidence scene in the atlas, from anywhere', () => {
    [0, 3, LAST_STEP].forEach((from) => {
      const s = run([{ type: 'SKIP_TO_TECHNICAL' }], run([{ type: 'GOTO', step: from }]));
      expect(s.step).toBe(TECHNICAL_STEP);
      expect(s.view).toBe('atlas');
      expect(s.selection).toBe('AQ-001');
    });
  });

  it('uses each scene’s default view and focus, and lets the visitor switch views', () => {
    STEPS.forEach((st, i) => {
      const s = run([{ type: 'GOTO', step: i }]);
      expect(s.view).toBe(st.defaultView);
      expect(s.selection).toBe(st.focus);
    });
    const s = run([{ type: 'SET_VIEW', view: 'atlas' }]);
    expect(s.view).toBe('atlas');
    expect(run([{ type: 'SET_VIEW', view: 'bogus' }])).toEqual(initialState);
    // moving to another scene resets to that scene's default view
    expect(run([next], s).view).toBe(STEPS[1].defaultView);
  });

  it('selection changes do not move the scenario', () => {
    const s = run([next, next, { type: 'SELECT', id: 'AG-COURIER' }]);
    expect(s.step).toBe(2);
    expect(s.selection).toBe('AG-COURIER');
    expect(run([{ type: 'SELECT', id: null }], s).selection).toBeNull();
  });
});
