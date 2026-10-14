import { STEPS, TECHNICAL_STEP, getStep } from './scenario';

/**
 * Pure state transitions for the scenario. No timers, no DOM, no animation: what is
 * true at a step is derived from `scenario.js` by step index alone, so Next / Previous
 * / Reset / Skip are trivially consistent and replayable.
 */

export const LAST_STEP = STEPS.length - 1;

function atStep(step, view) {
  const s = getStep(step);
  return { step: STEPS.indexOf(s), view: view || s.defaultView, selection: s.focus };
}

export const initialState = atStep(0);

export function reducer(state, action) {
  switch (action.type) {
    case 'NEXT':
      return state.step >= LAST_STEP ? state : atStep(state.step + 1);
    case 'PREV':
      return state.step <= 0 ? state : atStep(state.step - 1);
    case 'RESET':
      return initialState;
    case 'GOTO': {
      const n = Number(action.step);
      if (!Number.isInteger(n) || n < 0 || n > LAST_STEP) return state;
      return atStep(n);
    }
    case 'SKIP_TO_TECHNICAL':
      return atStep(TECHNICAL_STEP);
    case 'SELECT':
      return { ...state, selection: action.id || null };
    case 'SET_VIEW':
      return action.view === 'floor' || action.view === 'atlas' ? { ...state, view: action.view } : state;
    default:
      return state;
  }
}
