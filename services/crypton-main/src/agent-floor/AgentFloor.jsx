import React, { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
import './agentFloor.css';
import Stage from './Stage';
import Inquiry from './Inquiry';
import { EventLog, Inspector, ManagerDesk, ResearchPanel } from './Panels';
import { SCENARIO_ID, STEPS, visibleAt } from './scenario';
import { LAST_STEP, initialState, reducer } from './state';

/** True when the visitor asks the OS/browser to reduce motion. Safe where matchMedia is absent. */
export function useReducedMotion() {
  const query = '(prefers-reduced-motion: reduce)';
  const [reduced, setReduced] = useState(() => (typeof window !== 'undefined' && window.matchMedia ? window.matchMedia(query).matches : false));
  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return undefined;
    const mq = window.matchMedia(query);
    const on = (e) => setReduced(e.matches);
    if (mq.addEventListener) mq.addEventListener('change', on);
    else if (mq.addListener) mq.addListener(on);
    return () => {
      if (mq.removeEventListener) mq.removeEventListener('change', on);
      else if (mq.removeListener) mq.removeListener(on);
    };
  }, []);
  return reduced;
}

const isTypingTarget = (el) => el && (['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName) || el.isContentEditable);

export default function AgentFloor() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const reduced = useReducedMotion();
  const vis = useMemo(() => visibleAt(state.step), [state.step]);
  const step = STEPS[state.step];
  const focusInspector = useRef(false);
  const scenarioRef = useRef(null);

  useEffect(() => {
    const previous = document.title;
    document.title = 'Crypton / Agent Floor: who was actually behind the action?';
    return () => { document.title = previous; };
  }, []);

  const select = useCallback((id) => dispatch({ type: 'SELECT', id }), []);
  const next = useCallback(() => dispatch({ type: 'NEXT' }), []);
  const prev = useCallback(() => dispatch({ type: 'PREV' }), []);

  const scrollTo = (el) => {
    if (el && el.scrollIntoView) el.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'start' });
  };

  const takeCommand = () => {
    dispatch({ type: 'GOTO', step: 1 });
    scrollTo(scenarioRef.current);
  };

  const skipToTechnical = () => {
    focusInspector.current = true;
    dispatch({ type: 'SKIP_TO_TECHNICAL' });
    scrollTo(scenarioRef.current);
  };

  useEffect(() => {
    if (focusInspector.current) {
      focusInspector.current = false;
      const el = document.getElementById('af-inspector');
      if (el) el.focus({ preventScroll: true });
    }
  }, [state.step]);

  const onKeyDown = (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey || isTypingTarget(e.target)) return;
    if (e.key === 'ArrowRight') { e.preventDefault(); next(); }
    else if (e.key === 'ArrowLeft') { e.preventDefault(); prev(); }
  };

  return (
    <div className="af" data-motion={reduced ? 'reduced' : 'full'} data-view={state.view}>
      <a className="af-skip" href="#af-scenario">Skip to the scenario</a>

      <header className="af-nav">
        <a className="af-brand" href="/agent-floor" aria-label="Crypton Agent Floor home">
          <span className="af-brand-mark" aria-hidden="true" />
          CRYPTON <span className="af-brand-sub">/ Agent Floor</span>
        </a>
        <nav aria-label="Primary" className="af-nav-links">
          <a href="#af-scenario">Scenario</a>
          <a href="#af-research">Research</a>
          <a href="#af-inquiry">Inquiry</a>
          <a href="/" className="af-nav-back">Main site</a>
        </nav>
      </header>

      <main id="af-main">
        <section className="af-hero" aria-labelledby="af-hero-h">
          <p className="af-eyebrow">Agent authorization and delegation assessment</p>
          <h1 id="af-hero-h">You gave them a task. What did you give them permission to do?</h1>
          <p className="af-lead">
            Run a short, fictional operations floor with three robot agents, then see how an authority
            assessment would question what happened. Everything in the scenario is illustrative.
          </p>
          <div className="af-row">
            <button type="button" className="af-btn af-btn--primary" onClick={takeCommand}>Take command</button>
            <a className="af-btn" href="#af-research">Explore the research</a>
            <a className="af-btn" href="#af-inquiry">Discuss an assessment</a>
          </div>
          <p className="af-row">
            <button type="button" className="af-link" onClick={skipToTechnical}>Skip to the technical explanation</button>
          </p>
        </section>

        <section id="af-scenario" ref={scenarioRef} className="af-section af-scenario" aria-labelledby="af-scenario-h" onKeyDown={onKeyDown}>
          <div className="af-scenario-head">
            <h2 id="af-scenario-h">Scenario {SCENARIO_ID}</h2>
            <p className="af-badge af-badge--illustrative">Illustrative. Not a finding.</p>
          </div>

          <div className="af-toolbar" role="group" aria-label="Scenario controls">
            <button type="button" className="af-btn" onClick={prev} disabled={state.step === 0}>Previous</button>
            <button type="button" className="af-btn af-btn--primary" onClick={next} disabled={state.step === LAST_STEP}>Next</button>
            <button type="button" className="af-btn" onClick={() => dispatch({ type: 'RESET' })}>Reset</button>
            <span className="af-progress" aria-hidden="true"><span style={{ width: `${(state.step / LAST_STEP) * 100}%` }} /></span>
            <span className="af-step-label">Scene {String(state.step).padStart(2, '0')} of {String(LAST_STEP).padStart(2, '0')}: {step.title}</span>
            <div className="af-seg" role="group" aria-label="View">
              <button type="button" className="af-btn" aria-pressed={state.view === 'floor'} onClick={() => dispatch({ type: 'SET_VIEW', view: 'floor' })}>Floor</button>
              <button type="button" className="af-btn" aria-pressed={state.view === 'atlas'} onClick={() => dispatch({ type: 'SET_VIEW', view: 'atlas' })}>Atlas</button>
            </div>
            <button type="button" className="af-btn af-btn--ghost" onClick={skipToTechnical}>Skip to technical explanation</button>
          </div>
          <p className="af-hint">Tip: the left and right arrow keys also move between scenes.</p>

          <div className="af-grid-layout">
            <div className="af-stage-wrap">
              <div className="af-stage-scroll" tabIndex={0} role="region" aria-label="Scenario stage. On small screens it scrolls sideways.">
                <Stage
                  view={state.view}
                  step={state.step}
                  selection={state.selection}
                  focus={step.focus}
                  onSelect={select}
                  vis={vis}
                />
              </div>
              <p className="af-stage-caption">
                {state.view === 'floor'
                  ? 'Floor: a miniature operations floor. Select a robot, resource or tag to inspect it.'
                  : 'Atlas: the same agents, resources, grants and actions as a map of authority.'}
              </p>
            </div>
            <div className="af-side">
              <ManagerDesk step={state.step} vis={vis} selection={state.selection} onSelect={select} onNext={next} />
              <Inspector selection={state.selection} vis={vis} step={state.step} onSelect={select} />
            </div>
          </div>

          <EventLog step={state.step} vis={vis} />
        </section>

        <ResearchPanel />
        <Inquiry />
      </main>

      <footer className="af-footer">
        <p>
          Crypton is developing an agent authorization and delegation assessment. The methodology has
          been exercised internally; external customer validation is pending. The scenario on this page
          is fictional.
        </p>
      </footer>
    </div>
  );
}
