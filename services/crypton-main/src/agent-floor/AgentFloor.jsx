import React, { Suspense, lazy, useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
import './agentFloor.css';
import './agentFloorWorld.css';
import Stage from './Stage';
import AssessmentForm from './AssessmentForm';
import { EventLog, Inspector, ManagerDesk, ResearchPanel } from './Panels';
import { SCENARIO_ID, STEPS, visibleAt } from './scenario';
import { LAST_STEP, initialState, reducer } from './state';
import { MOMENTS, STORY, momentOf } from './story';
import { initialMode, sceneFromURL, useOnScreen, useSceneScroll, useViewport } from './experience';

// The 3D world is its own chunk: it loads only where WebGL is available.
const World3D = lazy(() => import('./world/World3D'));

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

/** If the 3D world fails (lost context, driver trouble), fall back to the 2D stage. */
class WorldBoundary extends React.Component {
  constructor(props) { super(props); this.state = { failed: false }; }
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch() { this.props.onFail(); }
  render() { return this.state.failed ? null : this.props.children; }
}

const isTypingTarget = (el) => el && (['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName) || el.isContentEditable);
const pad = (n) => String(n).padStart(2, '0');
const FINAL = LAST_STEP;

function HeroCopy({ onWatch, onAssess }) {
  const s = STORY[0];
  return (
    <>
      <p className="af-eyebrow">A fictional simulation</p>
      <h1 id="af-hero-h">{s.line}</h1>
      <p className="af-lead">{s.sub}</p>
      <div className="af-row">
        <button type="button" className="af-btn af-btn--primary" onClick={onWatch}>Watch them work</button>
        <a className="af-btn" href="#af-inquiry" onClick={onAssess}>Skip to the assessment</a>
      </div>
    </>
  );
}

/** The classic (2D) scenario controls. The immersive layout uses the story bar instead. */
function Toolbar({ state, step, prev, next, dispatch, skipToTechnical }) {
  return (
    <div className="af-toolbar" role="group" aria-label="Scenario controls">
      <button type="button" className="af-btn" onClick={prev} disabled={state.step === 0}>Previous</button>
      <button type="button" className="af-btn af-btn--primary" onClick={next} disabled={state.step === LAST_STEP}>Next</button>
      <button type="button" className="af-btn" onClick={() => dispatch({ type: 'RESET' })}>Reset</button>
      <span className="af-progress" aria-hidden="true"><span style={{ width: `${(state.step / LAST_STEP) * 100}%` }} /></span>
      <span className="af-step-label">Scene {pad(state.step)} of {pad(LAST_STEP)}: {step.title}</span>
      <ViewSwitch view={state.view} dispatch={dispatch} />
      <button type="button" className="af-btn af-btn--ghost" onClick={skipToTechnical}>Skip to technical explanation</button>
    </div>
  );
}

function ViewSwitch({ view, dispatch }) {
  return (
    <div className="af-seg" role="group" aria-label="View">
      <button type="button" className="af-btn" aria-pressed={view === 'floor'} onClick={() => dispatch({ type: 'SET_VIEW', view: 'floor' })}>Floor</button>
      <button type="button" className="af-btn" aria-pressed={view === 'atlas'} onClick={() => dispatch({ type: 'SET_VIEW', view: 'atlas' })}>Atlas</button>
    </div>
  );
}

function AtlasLegend() {
  return (
    <ul className="af-legend" aria-label="Atlas legend">
      <li><span className="af-key af-key--grant" aria-hidden="true" />Grant</li>
      <li><span className="af-key af-key--delegation" aria-hidden="true" />Delegation</li>
      <li><span className="af-key af-key--executed" aria-hidden="true" />Executed (illustrative)</li>
      <li><span className="af-key af-key--attempted" aria-hidden="true" />Attempted, outcome not recorded</li>
    </ul>
  );
}

/** What is real and what is illustrated: kept, but out of the way of the story. */
function AssessmentContext() {
  return (
    <div className="af-assessment-context">
      <p className="af-next-step">We’ll discuss one workflow, agree on scope, and determine whether an assessment fits.</p>
      <ul className="af-deliverables" aria-label="Assessment deliverables">
        <li>Authority mapping</li><li>Reproducible findings</li><li>Remediation guidance</li>
      </ul>
      <a className="af-link af-benchmark-link" href="https://github.com/Aryanvirpsu/Crypton-DI/tree/main/labs/validation-benchmark/v0.1" target="_blank" rel="noreferrer">Vulnerable vs. fixed: benchmark evidence ↗</a>
    </div>
  );
}

function RealityNote() {
  return (
    <section id="af-real" className="af-section af-real" aria-label="What is real and what is illustrated">
      <details className="af-real-details">
        <summary>What’s real, and what’s illustrated</summary>
        <ResearchPanel />
      </details>
    </section>
  );
}

export default function AgentFloor() {
  const [state, dispatch] = useReducer(reducer, initialState, (s) => {
    const n = sceneFromURL(STEPS.length);
    return n === null ? s : reducer(s, { type: 'GOTO', step: n });
  });
  const [mode, setMode] = useState(initialMode);
  const immersive = mode === 'immersive';
  const reduced = useReducedMotion();
  const vis = useMemo(() => visibleAt(state.step), [state.step]);
  const step = STEPS[state.step];
  const story = STORY[state.step];
  const moment = momentOf(state.step);
  const focusInspector = useRef(false);
  const focusForm = useRef(false);
  const scenarioRef = useRef(null);
  const progressRef = useRef(0);
  const clockRef = useRef(null);
  const firstFieldRef = useRef(null);
  const typingRef = useRef(false);
  const tagLayerRef = useRef(null);
  const onScreen = useOnScreen(scenarioRef, immersive);
  const [vw, vh] = useViewport();
  // The technical layer (inspector, event log, task vs grant, Atlas legend) is optional.
  const [details, setDetails] = useState(false);
  // The last scene has two beats: the pullback, then the call to action rises over a dimmed world.
  const [cta, setCta] = useState(false);
  const skipIntro = useRef(false);

  useEffect(() => {
    const previous = document.title;
    document.title = 'Crypton / Agent Floor: who was actually behind the action?';
    return () => { document.title = previous; };
  }, []);

  const select = useCallback((id) => {
    dispatch({ type: 'SELECT', id });
    if (id) setDetails(true);
  }, []);
  const next = useCallback(() => dispatch({ type: 'NEXT' }), []);
  const prev = useCallback(() => dispatch({ type: 'PREV' }), []);
  const goScene = useCallback((n) => dispatch({ type: 'GOTO', step: n }), []);

  useSceneScroll({ enabled: immersive, trackRef: scenarioRef, step: state.step, count: STEPS.length, onScene: goScene, progressRef, pausedRef: typingRef });

  const scrollTo = (el) => {
    if (el && el.scrollIntoView) el.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'start' });
  };

  const watch = () => {
    dispatch({ type: 'GOTO', step: 1 });
    if (!immersive) scrollTo(scenarioRef.current);
  };

  /** "Get an assessment" from anywhere: the form is the last scene, so go there. */
  const goAssessment = (e) => {
    if (!immersive) return; // classic: the #af-inquiry anchor does the work
    if (e) e.preventDefault();
    if (state.step === FINAL && cta) { // already there: just take the visitor to the first field
      if (firstFieldRef.current) firstFieldRef.current.focus({ preventScroll: true });
      return;
    }
    focusForm.current = true;
    skipIntro.current = true;
    setDetails(false);
    dispatch({ type: 'GOTO', step: FINAL });
  };

  const skipToTechnical = () => {
    focusInspector.current = true;
    dispatch({ type: 'SKIP_TO_TECHNICAL' });
    if (!immersive) scrollTo(scenarioRef.current);
  };

  useEffect(() => {
    if (focusInspector.current) {
      focusInspector.current = false;
      const el = document.getElementById('af-inspector');
      if (el) el.focus({ preventScroll: true });
    }
  }, [state.step]);

  // Arriving at the last scene: pullback first, then the call to action. Anyone who asked for
  // the assessment (nav, skip, 04 Act) and anyone who prefers reduced motion gets it at once.
  useEffect(() => {
    if (!immersive) return undefined;
    if (state.step !== FINAL) { setCta(false); skipIntro.current = false; return undefined; }
    setDetails(false);
    const quick = skipIntro.current || reduced;
    const t = setTimeout(() => setCta(true), quick ? 150 : 2800);
    return () => clearTimeout(t);
  }, [state.step, immersive, reduced]);

  useEffect(() => {
    if (cta && focusForm.current) {
      focusForm.current = false;
      // the card is no longer inert on this commit; focus once it is painted
      requestAnimationFrame(() => firstFieldRef.current && firstFieldRef.current.focus({ preventScroll: true }));
    }
  }, [cta]);

  // Escape closes the details drawer wherever focus is.
  useEffect(() => {
    if (!details) return undefined;
    const onEsc = (e) => { if (e.key === 'Escape') setDetails(false); };
    window.addEventListener('keydown', onEsc);
    return () => window.removeEventListener('keydown', onEsc);
  }, [details]);

  const onKeyDown = (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey || isTypingTarget(e.target)) return;
    if (e.key === 'ArrowRight') { e.preventDefault(); next(); }
    else if (e.key === 'ArrowLeft') { e.preventDefault(); prev(); }
  };

  // Keep the camera's subject clear of the HUD: right of the hero / closing card, left of
  // the details drawer, above the caption; on phones, above the bottom sheet.
  const wide = vw >= 1024;
  const bookend = state.step === 0 || state.step === FINAL;
  const inset = useMemo(() => {
    if (!wide) return [0, -Math.round(vh * (state.step === 0 ? 0.2 : state.step === FINAL ? 0.2 : 0.12))];
    if (state.step === FINAL) return [0, 0]; // the world sits centred behind the call to action
    if (details) return [-210, -10];
    if (state.step === 0) return [270, 20];
    return [110, -40];
  }, [wide, vh, details, state.step]);

  useEffect(() => {
    if (!cta || wide) return undefined;
    const root = document.documentElement;
    const previous = root.style.overflowY;
    root.style.overflowY = 'hidden';
    return () => { root.style.overflowY = previous; };
  }, [cta, wide]);

  const nav = (
    <header className="af-nav">
      <a className="af-brand" href="/" aria-label="Crypton Agent Floor home">
        <span className="af-brand-mark" aria-hidden="true" />
        CRYPTON <span className="af-brand-sub">/ Agent Floor</span>
      </a>
      <nav aria-label="Primary" className="af-nav-links">
        <a href="#af-real">What’s real</a>
        <a href="#af-inquiry" className="af-nav-cta" onClick={goAssessment}>Get an assessment</a>
      </nav>
    </header>
  );

  if (!immersive) {
    const toolbar = <Toolbar state={state} step={step} prev={prev} next={next} dispatch={dispatch} skipToTechnical={skipToTechnical} />;
    return (
      <div className="af" data-motion={reduced ? 'reduced' : 'full'} data-view={state.view}>
        <a className="af-skip" href="#af-scenario">Skip to the scenario</a>
        {nav}
        <main id="af-main">
          <section className="af-hero" aria-labelledby="af-hero-h">
            <HeroCopy onWatch={watch} />
          </section>

          <section id="af-scenario" ref={scenarioRef} className="af-section af-scenario" aria-labelledby="af-scenario-h" onKeyDown={onKeyDown}>
            <div className="af-scenario-head">
              <h2 id="af-scenario-h">Scenario {SCENARIO_ID}</h2>
              <p className="af-badge af-badge--illustrative">Illustrative. Not a finding.</p>
            </div>
            {toolbar}
            <p className="af-hint">Tip: the left and right arrow keys also move between scenes.</p>
            <div className="af-grid-layout">
              <div className="af-stage-wrap">
                <div className="af-stage-scroll" tabIndex={0} role="region" aria-label="Scenario stage. On small screens it scrolls sideways.">
                  <Stage view={state.view} step={state.step} selection={state.selection} focus={step.focus} onSelect={select} vis={vis} />
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

          <section id="af-inquiry" className="af-section af-assess" aria-labelledby="af-assess-h">
            <p className="af-eyebrow">Simulation complete</p>
            <h2 id="af-assess-h">{STORY[FINAL].line}</h2>
            <p className="af-lead">{STORY[FINAL].sub}</p>
            <AssessmentContext />
            <AssessmentForm />
          </section>

          <RealityNote />
        </main>
        <Footer />
      </div>
    );
  }

  const trackTyping = {
    onFocus: (e) => { if (isTypingTarget(e.target)) typingRef.current = true; },
    onBlur: () => { typingRef.current = false; },
  };

  return (
    <div className="af af--immersive" data-motion={reduced ? 'reduced' : 'full'} data-view={state.view} data-cta={cta ? 'true' : 'false'}>
      <a className="af-skip" href="#af-scenario">Skip to the scenario</a>
      {nav}
      <main id="af-main">
        <section
          id="af-scenario"
          ref={scenarioRef}
          className="af-track"
          style={{ '--af-scenes': STEPS.length }}
          aria-labelledby="af-scenario-h"
          onKeyDown={onKeyDown}
        >
          <h2 id="af-scenario-h" className="af-sr">Agent Floor simulation, scenario {SCENARIO_ID}</h2>
          <div
            className="af-sticky"
            data-step={state.step}
            data-moment={moment.id}
            data-held={state.step >= 4 ? 'true' : 'false'}
            data-details={details ? 'open' : 'closed'}
            data-far={state.step === FINAL && state.view === STEPS[FINAL].defaultView ? 'true' : 'false'}
            data-view-atlas={state.view === 'atlas' ? 'true' : 'false'}
          >
            <div className="af-world">
              {/* world tags (real buttons) live here, stacked above the canvas. It comes first so its
                  ref exists before the canvas renders any tag (see Anchor in world/Labels3D.jsx). */}
              <div className="af-tag-layer" ref={tagLayerRef} />
              <WorldBoundary onFail={() => setMode('classic')}>
                <Suspense fallback={<p className="af-world-loading">Building the floor…</p>}>
                  <World3D
                    step={state.step}
                    view={state.view}
                    selection={state.selection}
                    focus={step.focus}
                    onSelect={select}
                    vis={vis}
                    reduced={reduced}
                    active={onScreen}
                    progressRef={progressRef}
                    clockRef={clockRef}
                    inset={inset}
                    detail={details}
                    cta={cta}
                    tagLayerRef={tagLayerRef}
                  />
                </Suspense>
              </WorldBoundary>
            </div>

            <div className="af-letterbox" aria-hidden="true"><span /><span /></div>
            <div className="af-scrim" aria-hidden="true" />

            <div className="af-hud">
              {/* 01 · the opening */}
              <div className="af-hud-hero" inert={state.step !== 0 ? true : undefined}>
                <HeroCopy onWatch={watch} onAssess={goAssessment} />
                <p className="af-scroll-cue" aria-hidden="true">Scroll to watch</p>
              </div>

              {/* 02–03 · one line per scene (re-keyed so each line animates in) */}
              <div className="af-hud-caption">
                <div key={state.step} className="af-caption-anim" aria-hidden="true">
                  <p className="af-caption-moment">{moment.n} · {moment.label}</p>
                  <p className="af-caption-line">{story.line}</p>
                  {story.sub && <p className="af-caption-sub">{story.sub}</p>}
                </div>
                {story.detail && !bookend && (
                  <button type="button" className="af-link af-caption-more" onClick={() => select(story.detail.id)}>
                    {story.detail.label}
                  </button>
                )}
              </div>
              <p className="af-sr" role="status" aria-live="polite">
                Scene {pad(state.step)} of {pad(LAST_STEP)}. {story.line} {story.sub}
              </p>

              {/* 04a · the pullback: scale, before the ask */}
              <p className="af-hud-pullback" aria-hidden="true" data-on={state.step === FINAL && !cta ? 'true' : 'false'}>
                {STORY[FINAL].pullback}
              </p>

              {/* 04b · the last scene is the assessment request, layered over the dimmed world */}
              <div className="af-hud-closing" id="af-inquiry" inert={!cta ? true : undefined} {...trackTyping}>
                <div className="af-closing-copy">
                  <p className="af-eyebrow">Simulation complete</p>
                  <h2 className="af-closing-h">{STORY[FINAL].line}</h2>
                  <p className="af-lead">{STORY[FINAL].sub}</p>
            <AssessmentContext />
                </div>
                <div className="af-closing-form">
                  <AssessmentForm firstFieldRef={firstFieldRef} />
                </div>
              </div>

              {/* optional: the technical evidence */}
              <aside id="af-details" className="af-details-drawer" aria-label="Technical details" inert={!details ? true : undefined}>
                <div className="af-drawer-head">
                  <p className="af-eyebrow">Technical details · Scene {pad(state.step)}: {step.title}</p>
                  <button type="button" className="af-btn af-btn--small" onClick={() => setDetails(false)}>Close</button>
                </div>
                <p className="af-badge af-badge--illustrative">Illustrative. Not a finding.</p>
                <ViewSwitch view={state.view} dispatch={dispatch} />
                {state.view === 'atlas' && <div className="af-atlas-key"><AtlasLegend /></div>}
                <Inspector selection={state.selection} vis={vis} step={state.step} onSelect={select} />
                <EventLog step={state.step} vis={vis} compact live={false} />
                <ManagerDesk step={state.step} vis={vis} selection={state.selection} onSelect={select} compact />
              </aside>

              <p className="af-held" aria-hidden="true"><span>Paused</span>at the moment of the request</p>

              <div className="af-hud-bar">
                <nav className="af-moments" aria-label="Story">
                  {MOMENTS.map((m) => (
                    <button
                      key={m.id}
                      type="button"
                      className="af-moment"
                      aria-current={moment.id === m.id ? 'step' : undefined}
                      onClick={() => (m.id === 'act' ? goAssessment() : goScene(m.first))}
                    >
                      <span className="af-moment-n">{m.n}</span> <span className="af-moment-label">{m.label}</span>
                    </button>
                  ))}
                </nav>
                <div className="af-bar-actions">
                  <button type="button" className="af-btn af-btn--icon" aria-label="Previous scene" onClick={prev} disabled={state.step === 0}>‹</button>
                  <button type="button" className="af-btn af-btn--icon" aria-label="Next scene" onClick={next} disabled={state.step === FINAL}>›</button>
                  <button type="button" className="af-btn af-btn--quiet af-bar-replay" onClick={() => dispatch({ type: 'RESET' })}>Replay</button>
                  <button type="button" className="af-btn af-btn--quiet" aria-expanded={details} aria-controls="af-details" onClick={() => setDetails((d) => !d)}>
                    {details ? 'Hide details' : 'Inspect details'}
                  </button>
                </div>
                <span className="af-timecode" aria-hidden="true">
                  <span ref={clockRef}>T+00:00.00</span>
                  <span className="af-timecode-state">{state.step >= 4 ? 'DEMO · HELD' : 'SIMULATION'}</span>
                </span>
              </div>
            </div>
          </div>
        </section>

        <RealityNote />
      </main>
      <Footer />
    </div>
  );
}

function Footer() {
  return (
    <footer className="af-footer">
      <p>
        The scenario on this page is fictional. Crypton’s assessment methodology has been exercised
        internally; external customer validation is pending.
      </p>
    </footer>
  );
}
