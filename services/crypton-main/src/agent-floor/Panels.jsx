import React from 'react';
import {
  EVIDENCE,
  OUTCOMES,
  PRINCIPALS,
  RESEARCH,
  STEPS,
  TASK,
  displayName,
  getEntity,
} from './scenario';

const verbLabel = (v) => `${v.verb} ${v.resource}`;

/** Status wording. "Attempted" never implies executed or blocked. */
export function statusText(action) {
  if (action.status === 'executed') return 'Executed (illustrative)';
  return 'Attempted. Whether it ran or was stopped is not recorded.';
}

// ── manager desk ─────────────────────────────────────────────────────────────

export function ManagerDesk({ step, vis, selection, onSelect, onNext }) {
  const grant = vis.grants[0];
  const s = STEPS[step];
  return (
    <section className="af-panel-card" aria-labelledby="af-desk-h">
      <h3 id="af-desk-h" className="af-eyebrow">Manager desk</h3>
      {!vis.task ? (
        <p className="af-desk-empty">No task assigned yet. Assign Scout a task to begin.</p>
      ) : (
        <div className="af-taskgrant">
          <div className="af-taskgrant-col">
            <h4>Task</h4>
            <p>{TASK.text}</p>
            <p className="af-muted">What you asked for.</p>
          </div>
          <div className="af-taskgrant-col af-taskgrant-col--grant">
            <h4>Authority grant {grant && grant.id}</h4>
            <p>{grant && grant.plain}</p>
            <p className="af-muted">What Scout is actually permitted to do.</p>
          </div>
        </div>
      )}

      {step < STEPS.length - 1 && (
        <button type="button" className="af-btn af-btn--primary" onClick={onNext}>
          {s.cta}
        </button>
      )}

      <h4 className="af-eyebrow af-eyebrow--sub">Inspect agent</h4>
      <div className="af-row">
        {PRINCIPALS.filter((p) => p.kind === 'agent').map((p) => (
          <button
            key={p.id}
            type="button"
            className="af-btn"
            aria-pressed={selection === p.id}
            onClick={() => onSelect(p.id)}
          >
            {p.name}
          </button>
        ))}
      </div>

      <h4 className="af-eyebrow af-eyebrow--sub">Inspect grant / delegation</h4>
      {vis.grants.length + vis.delegations.length === 0 ? (
        <p className="af-muted">None issued yet.</p>
      ) : (
        <div className="af-row">
          {[...vis.grants, ...vis.delegations].map((g) => (
            <button key={g.id} type="button" className="af-btn" aria-pressed={selection === g.id} onClick={() => onSelect(g.id)}>
              {g.id}
            </button>
          ))}
        </div>
      )}
    </section>
  );
}

// ── inspector ────────────────────────────────────────────────────────────────

function Row({ label, children }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </>
  );
}

function IdButton({ id, onSelect }) {
  return (
    <button type="button" className="af-link" onClick={() => onSelect(id)}>
      {id}
    </button>
  );
}

function EntityDetails({ id, vis, onSelect }) {
  const e = getEntity(id);
  if (!e) return null;

  if (e.type === 'principal') {
    const holds = [...vis.grants.filter((g) => g.to === id), ...vis.delegations.filter((d) => d.to === id)];
    const acts = vis.actions.filter((a) => a.actor === id);
    return (
      <>
        <h3>{e.name} <span className="af-id">{e.id}</span></h3>
        <p>{e.role}. {e.summary}</p>
        <dl>
          <Row label="Authority held">
            {holds.length === 0 ? 'None yet' : holds.map((h) => <span key={h.id}><IdButton id={h.id} onSelect={onSelect} />{' '}</span>)}
          </Row>
          <Row label="Actions">
            {acts.length === 0 ? 'None yet' : acts.map((a) => (
              <div key={a.id}><IdButton id={a.id} onSelect={onSelect} /> {a.verb} {a.resource}: {statusText(a)}</div>
            ))}
          </Row>
        </dl>
      </>
    );
  }
  if (e.type === 'resource') {
    const acts = vis.actions.filter((a) => a.resource === id || a.destination === id);
    return (
      <>
        <h3>{e.name} <span className="af-id">{e.id}</span></h3>
        <p>{e.note}</p>
        <dl>
          <Row label="Touched by">
            {acts.length === 0 ? 'Nothing yet' : acts.map((a) => (
              <div key={a.id}><IdButton id={a.id} onSelect={onSelect} /> {displayName(a.actor)}: {a.verb}</div>
            ))}
          </Row>
        </dl>
      </>
    );
  }
  if (e.type === 'grant') {
    return (
      <>
        <h3>Grant <span className="af-id">{e.id}</span></h3>
        <p>{e.plain}</p>
        <dl>
          <Row label="From → to">{displayName(e.from)} → {displayName(e.to)}</Row>
          <Row label="Allows">{e.allows.map(verbLabel).join('; ')}</Row>
          <Row label="Not granted">{e.notGranted.join('; ')}</Row>
        </dl>
      </>
    );
  }
  if (e.type === 'delegation') {
    return (
      <>
        <h3>Delegation <span className="af-id">{e.id}</span></h3>
        <p>{e.plain}</p>
        <dl>
          <Row label="From → to">{displayName(e.from)} → {displayName(e.to)}</Row>
          <Row label="Derived from"><IdButton id={e.parentGrant} onSelect={onSelect} /></Row>
          <Row label="Purpose">{e.purpose}</Row>
          <Row label="Carries">{e.carries.map(verbLabel).join('; ')}</Row>
          <Row label="Does not carry">Anything the parent grant did not give Scout, and nothing beyond the summarize scope.</Row>
        </dl>
      </>
    );
  }
  if (e.type === 'action') {
    return (
      <>
        <h3>Action <span className="af-id">{e.id}</span></h3>
        <p>{e.plain}</p>
        <dl>
          <Row label="Actor">{displayName(e.actor)}</Row>
          <Row label="Does">{e.verb}{e.target ? ` via ${displayName(e.target)}` : ''}</Row>
          <Row label="Resource">{e.resource}{e.destination ? ` → ${e.destination}` : ''}</Row>
          <Row label="Status">{statusText(e)}</Row>
          {e.id === 'ACT-003' && (
            <>
              <Row label="Execution record">Not modeled</Row>
              <Row label="Enforcement decision">Not modeled</Row>
            </>
          )}
          <Row label="Covered by">
            {e.coveredBy ? <IdButton id={e.coveredBy} onSelect={onSelect} /> : 'No grant or delegation in this scenario covers it.'}
          </Row>
        </dl>
      </>
    );
  }
  if (e.type === 'task') {
    return (
      <>
        <h3>Task <span className="af-id">{e.id}</span></h3>
        <p>{e.text}</p>
        <p className="af-muted">A task says what you want. It is not an authority grant.</p>
      </>
    );
  }
  return null;
}

export function EvidencePanel({ step, onSelect, resolved }) {
  return (
    <div className="af-evidence">
      <p className="af-badge af-badge--illustrative">Illustrative scenario. Not a finding.</p>
      <h3>Authority question <span className="af-id">{EVIDENCE.question.id}</span></h3>
      <p className="af-question">{EVIDENCE.question.text}</p>
      <p><strong>What the modeled facts say:</strong> {EVIDENCE.question.modeledAnswer}</p>

      <h4 className="af-eyebrow af-eyebrow--sub">Evidence available</h4>
      <ul className="af-list">
        {EVIDENCE.available.map((ev) => (
          <li key={ev.id}>
            <strong>{ev.label}.</strong> {ev.says}{' '}
            {ev.refs.map((r) => <IdButton key={r} id={r} onSelect={onSelect} />)}
          </li>
        ))}
      </ul>

      <h4 className="af-eyebrow af-eyebrow--sub">Missing</h4>
      <ul className="af-list af-list--missing">
        {EVIDENCE.missing.map((m) => <li key={m}>{m}</li>)}
      </ul>

      <h4 className="af-eyebrow af-eyebrow--sub">Limitations</h4>
      <ul className="af-list">
        {EVIDENCE.limitations.map((m) => <li key={m}>{m}</li>)}
      </ul>

      <h4 className="af-eyebrow af-eyebrow--sub">Outcome this evidence could support</h4>
      <p>
        <span className="af-outcome">{EVIDENCE.supportedOutcome.outcome}</span>{' '}
        {EVIDENCE.supportedOutcome.reason}
      </p>
      <details className="af-details">
        <summary>All five assessment outcomes</summary>
        <dl>
          {OUTCOMES.map((o) => (
            <React.Fragment key={o.id}>
              <dt>{o.id}</dt>
              <dd>{o.text}</dd>
            </React.Fragment>
          ))}
        </dl>
      </details>

      {resolved && (
        <div className="af-resolution">
          <h4 className="af-eyebrow af-eyebrow--sub">Resolution</h4>
          <p>
            In this illustration the export request went beyond every grant and delegation in the
            scenario, and the evidence does not show whether it ran. A real assessment answers this
            with real evidence: who held what authority, how it was passed on, and who was actually
            behind the action.
          </p>
          <p className="af-row">
            <a className="af-btn af-btn--primary" href="#af-inquiry">Discuss an assessment</a>
            <a className="af-btn" href="#af-research">Read the research status</a>
          </p>
        </div>
      )}
    </div>
  );
}

export function Inspector({ selection, vis, step, onSelect }) {
  const showEvidence = selection === EVIDENCE.question.id && step >= 6;
  return (
    <section className="af-panel-card af-inspector" aria-labelledby="af-inspect-h" tabIndex={-1} id="af-inspector">
      <h3 id="af-inspect-h" className="af-eyebrow">{showEvidence ? 'Evidence inspector' : 'Inspector'}</h3>
      {step >= 6 && !showEvidence && (
        <button type="button" className="af-btn af-btn--small" onClick={() => onSelect(EVIDENCE.question.id)}>
          Back to the authority question
        </button>
      )}
      {showEvidence ? (
        <EvidencePanel step={step} onSelect={onSelect} resolved={step >= 7} />
      ) : selection ? (
        <EntityDetails id={selection} vis={vis} onSelect={onSelect} />
      ) : (
        <p className="af-muted">Select a robot, resource, grant or action to see what is known about it.</p>
      )}
    </section>
  );
}

// ── event log ────────────────────────────────────────────────────────────────

export function EventLog({ step, vis }) {
  const s = STEPS[step];
  return (
    <section className="af-log" aria-labelledby="af-log-h">
      <h3 id="af-log-h" className="af-eyebrow">Scenario events</h3>
      <p className="af-narration" role="status" aria-live="polite">
        <strong>Scene {String(step).padStart(2, '0')}: {s.title}.</strong> {s.narration}
      </p>
      {vis.events.length === 0 ? (
        <p className="af-muted">No events yet.</p>
      ) : (
        <ol className="af-events">
          {vis.events.map((ev) => (
            <li key={ev.id}><span className="af-id">{ev.id}</span> {ev.text}</li>
          ))}
        </ol>
      )}
    </section>
  );
}

// ── research status (what is real) ───────────────────────────────────────────

export function ResearchPanel() {
  return (
    <section id="af-research" className="af-section" aria-labelledby="af-research-h">
      <h2 id="af-research-h">What is real, and what is illustrated</h2>
      <div className="af-two">
        <div className="af-panel-card">
          <p className="af-badge af-badge--illustrative">Illustrative</p>
          <p>
            Scenario AF-001, its robots, grants, delegation and the export request are fictional. They
            exist to show the kind of question an authority assessment asks. They are not findings and
            do not describe any real system or customer.
          </p>
        </div>
        <div className="af-panel-card">
          <p className="af-badge af-badge--real">Crypton research today</p>
          <p>{RESEARCH.status}</p>
          <ul className="af-list">
            {RESEARCH.statements.map((st) => <li key={st}>{st}</li>)}
          </ul>
          <p className="af-row">
            {RESEARCH.links.map((l) => (
              <a key={l.href} className="af-btn" href={l.href} target="_blank" rel="noreferrer">{l.label}</a>
            ))}
          </p>
        </div>
      </div>
    </section>
  );
}

