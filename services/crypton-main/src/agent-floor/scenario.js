/**
 * AF-001: the single deterministic scenario behind Agent Floor.
 *
 * This file is the ONE source of truth for facts, IDs and the timeline. The Floor,
 * the Atlas and the evidence inspector all render from it; nothing about authority
 * or outcomes is decided in components or animation code.
 *
 * EVERYTHING HERE IS ILLUSTRATIVE. It is a fictional scenario written to show what an
 * authority assessment asks. It is not a finding, not a measurement of any real
 * system, and not a result of Crypton's research (see RESEARCH below for what is real).
 */

export const SCENARIO_ID = 'AF-001';

/** Stable status vocabulary for actions. "attempted" never implies "executed". */
export const ACTION_STATUS = {
  executed: 'Executed',
  attempted: 'Attempted',
};

export const PRINCIPALS = [
  { id: 'PR-MGR', kind: 'human', name: 'Manager', role: 'Human manager (you)', summary: 'Assigns the task and issues the grant.' },
  { id: 'AG-SCOUT', kind: 'agent', name: 'Scout', role: 'Research agent', summary: 'Reads a designated report and prepares summaries.' },
  { id: 'AG-RELAY', kind: 'agent', name: 'Relay', role: 'Coordination agent', summary: 'Receives delegated tasks and coordinates other agents.' },
  { id: 'AG-COURIER', kind: 'agent', name: 'Courier', role: 'Tool / external operations agent', summary: 'Executes operations against external resources.' },
];

export const RESOURCES = [
  { id: 'RS-REPORT', name: 'Q3 supplier report', note: 'The designated report for this task.' },
  { id: 'RS-RECORDS', name: 'Underlying supplier records', note: 'Source records behind the report. Not designated for this task.' },
  { id: 'RS-EXTERNAL', name: 'External destination', note: 'A file-share outside the organisation (fictional).' },
];

export const TASK = {
  id: 'TK-001',
  text: 'Summarize the Q3 supplier report.',
  assignedBy: 'PR-MGR',
  assignedTo: 'AG-SCOUT',
};

/** What a principal was actually permitted to do. Distinct from the task text. */
export const GRANTS = [
  {
    id: 'GR-001',
    from: 'PR-MGR',
    to: 'AG-SCOUT',
    allows: [
      { verb: 'read', resource: 'RS-REPORT' },
      { verb: 'summarize', resource: 'RS-REPORT' },
    ],
    notGranted: ['export (any resource)', 'any access to RS-RECORDS'],
    plain: 'Read the designated report and prepare a summary. External export is not granted.',
  },
];

/** A delegation can only carry authority the delegator already holds. */
export const DELEGATIONS = [
  {
    id: 'DL-001',
    from: 'AG-SCOUT',
    to: 'AG-RELAY',
    parentGrant: 'GR-001',
    purpose: 'Prepare the summary.',
    carries: [{ verb: 'summarize', resource: 'RS-REPORT' }],
    plain: 'Scout hands summary preparation to Relay, carrying only “summarize RS-REPORT”.',
  },
];

/** Things that happened (or were asked) in the scenario. */
export const ACTIONS = [
  {
    id: 'ACT-001',
    actor: 'AG-SCOUT',
    verb: 'read',
    resource: 'RS-REPORT',
    status: 'executed',
    coveredBy: 'GR-001',
    plain: 'Scout reads the designated report. Within GR-001.',
  },
  {
    id: 'ACT-002',
    actor: 'AG-SCOUT',
    verb: 'delegate',
    resource: 'RS-REPORT',
    target: 'AG-RELAY',
    status: 'executed',
    coveredBy: 'GR-001',
    via: 'DL-001',
    plain: 'Scout delegates summary preparation to Relay under DL-001.',
  },
  {
    id: 'ACT-003',
    actor: 'AG-RELAY',
    verb: 'request export',
    resource: 'RS-RECORDS',
    target: 'AG-COURIER',
    destination: 'RS-EXTERNAL',
    status: 'attempted',
    // Deliberately not modeled: whether anything enforced a decision, and whether the
    // export ran. The scenario records the REQUEST only.
    executed: null,
    enforcementRecorded: false,
    coveredBy: null,
    plain: 'Relay asks Courier to export underlying records to an external destination. No grant covers this.',
  },
];

/** Ordered event timeline. `step` is the first step at which an event is visible. */
export const EVENTS = [
  { id: 'EV-01', step: 1, kind: 'assignment', ref: 'TK-001', text: 'Manager assigns Scout the task: summarize the Q3 supplier report.' },
  { id: 'EV-02', step: 1, kind: 'grant', ref: 'GR-001', text: 'Grant GR-001 issued to Scout: read and summarize the designated report. External export is not granted.' },
  { id: 'EV-03', step: 2, kind: 'action', ref: 'ACT-001', text: 'Scout reads the designated report (executed, within GR-001). Illustrative.' },
  { id: 'EV-04', step: 3, kind: 'delegation', ref: 'DL-001', text: 'Delegation DL-001 recorded: Scout to Relay, carrying only the summarize scope on the report.' },
  { id: 'EV-05', step: 3, kind: 'action', ref: 'ACT-002', text: 'Scout hands summary preparation to Relay under DL-001 (executed). Illustrative.' },
  { id: 'EV-06', step: 4, kind: 'action', ref: 'ACT-003', text: 'Relay asks Courier to export the underlying records to an external destination. This was attempted; the scenario does not record whether it ran or was stopped.' },
];

export const STEPS = [
  {
    id: 'S00',
    cta: 'Assign the task to Scout',
    title: 'Arrival',
    defaultView: 'floor',
    narration: 'Three miniature robots work on an operations floor. You are the manager.',
    focus: null,
  },
  {
    id: 'S01',
    cta: 'Let Scout start',
    title: 'Assignment',
    defaultView: 'floor',
    narration: 'You assign Scout a task. The task is one thing; the authority granted to do it is another.',
    focus: 'GR-001',
  },
  {
    id: 'S02',
    cta: 'Let Scout delegate',
    title: 'Legitimate execution',
    defaultView: 'floor',
    narration: 'Scout reads the designated report. In this illustration the action succeeds, and it is inside the grant.',
    focus: 'ACT-001',
  },
  {
    id: 'S03',
    cta: 'Watch what Relay does next',
    title: 'Delegation',
    defaultView: 'floor',
    narration: 'Scout hands summary preparation to Relay. The delegation carries a scope with it.',
    focus: 'DL-001',
  },
  {
    id: 'S04',
    cta: 'Open the Authority Atlas',
    title: 'Boundary',
    defaultView: 'floor',
    narration: 'Relay asks Courier to export the underlying records outside the organisation. That request exceeds the modeled grant. It is shown as attempted only.',
    focus: 'ACT-003',
  },
  {
    id: 'S05',
    cta: 'Inspect the evidence',
    title: 'Authority Atlas',
    defaultView: 'atlas',
    narration: 'The same facts as a map: who holds which authority, over what, and how it was passed on.',
    focus: 'ACT-003',
  },
  {
    id: 'S06',
    cta: 'See the resolution',
    title: 'Evidence inspection',
    defaultView: 'atlas',
    narration: 'What would an assessor need to answer the authority question, and what is missing?',
    focus: 'AQ-001',
  },
  {
    id: 'S07',
    title: 'Resolution',
    defaultView: 'atlas',
    narration: 'What this illustration shows, and what Crypton actually assesses.',
    focus: 'AQ-001',
  },
];

export const TECHNICAL_STEP = 6; // "Skip to the technical explanation"

/** Evidence available to an assessor in this illustration, and what is not. */
export const EVIDENCE = {
  question: {
    id: 'AQ-001',
    text: 'Was Relay’s request to export RS-RECORDS to RS-EXTERNAL within any authority held by Relay or Courier?',
    modeledAnswer:
      'No. Relay holds only DL-001 (summarize RS-REPORT). Courier holds no grant at all. Nothing modeled covers export, or access to RS-RECORDS.',
  },
  available: [
    { id: 'EVI-001', label: 'Grant record GR-001', says: 'Manager → Scout: read, summarize RS-REPORT. Export not granted.', refs: ['GR-001'] },
    { id: 'EVI-002', label: 'Delegation record DL-001', says: 'Scout → Relay: summarize RS-REPORT only.', refs: ['DL-001'] },
    { id: 'EVI-003', label: 'Request record ACT-003', says: 'Relay requested an export via Courier. Status: attempted.', refs: ['ACT-003'] },
  ],
  missing: [
    'Any enforcement decision for ACT-003 (allowed or denied) and who or what made it.',
    'Any record from RS-EXTERNAL showing the data arrived, or did not.',
    'Any authority for Courier: no grant to Courier appears in the scenario.',
    'The chain from the Manager to this request: who was actually behind it?',
  ],
  limitations: [
    'This is a fictional scenario. No real system was observed.',
    'The scenario records the request, not the outcome. “Attempted” does not mean executed, and it does not mean blocked.',
    'An assessment outcome needs real evidence that meets every evidence-sufficiency gate. None exists here.',
  ],
  /** The outcome the evidence above could support if it were real. Illustrative, not a finding. */
  supportedOutcome: {
    outcome: 'UNVERIFIABLE',
    reason:
      'The authority question is representable (grants and delegations exist), but the evidence does not show whether the export ran or was stopped, so neither ENFORCED nor VIOLATED could be reached.',
  },
};

/** The five assessment outcomes, as defined in docs/ASSESSMENT_METHODOLOGY.md. */
export const OUTCOMES = [
  { id: 'ENFORCED', text: 'The claimed property was evidenced and held, and every evidence-sufficiency gate is satisfied.' },
  { id: 'VIOLATED', text: 'The property was evidenced as broken, all gates are satisfied, and the behaviour crosses a boundary the system claims to hold.' },
  { id: 'UNVERIFIABLE', text: 'The property can be expressed, but sufficient evidence could not be obtained in the engagement’s access and scope.' },
  { id: 'UNREPRESENTABLE', text: 'The system has no representation of the state needed to express the property at all.' },
  { id: 'NOT_APPLICABLE', text: 'The control genuinely does not apply to this workflow.' },
];

/** What is actually real about Crypton's work today. Kept separate from the illustration. */
export const RESEARCH = {
  status:
    'The assessment methodology has been exercised internally, including one synthetic reference dry run. It has not been validated against an external customer’s system.',
  statements: [
    'Crypton’s outcomes are ENFORCED, VIOLATED, UNVERIFIABLE, UNREPRESENTABLE and NOT_APPLICABLE. VIOLATED additionally requires an affirmative trust-boundary check.',
    'Principal continuity (“who was actually behind the action?”) is a research-preview test family, not a mature offering.',
    'Two benchmark cases run the same test against documented vulnerable and fixed versions of third-party software (the MCP Python SDK and DBHub). They check that the test can tell the two apart; they are not a validation on a customer system.',
  ],
  links: [
    { label: 'Assessment methodology', href: 'https://github.com/Aryanvirpsu/Crypton-DI/blob/main/docs/ASSESSMENT_METHODOLOGY.md' },
    { label: 'Validation benchmark', href: 'https://github.com/Aryanvirpsu/Crypton-DI/tree/main/labs/validation-benchmark/v0.1' },
  ],
};

// ── lookups ────────────────────────────────────────────────────────────────────

const ALL = [
  ...PRINCIPALS.map((p) => ({ ...p, type: 'principal' })),
  ...RESOURCES.map((r) => ({ ...r, type: 'resource' })),
  ...GRANTS.map((g) => ({ ...g, type: 'grant' })),
  ...DELEGATIONS.map((d) => ({ ...d, type: 'delegation' })),
  ...ACTIONS.map((a) => ({ ...a, type: 'action' })),
  { ...TASK, type: 'task' },
];
const BY_ID = new Map(ALL.map((e) => [e.id, e]));

export function getEntity(id) {
  return BY_ID.get(id) || null;
}

export function getStep(index) {
  return STEPS[Math.max(0, Math.min(STEPS.length - 1, index))];
}

/** Which grants / delegations / actions exist at a given step (monotonic in step). */
export function visibleAt(step) {
  const events = EVENTS.filter((e) => e.step <= step);
  const refs = new Set(events.map((e) => e.ref));
  return {
    events,
    task: step >= 1,
    grants: GRANTS.filter((g) => refs.has(g.id)),
    delegations: DELEGATIONS.filter((d) => refs.has(d.id)),
    actions: ACTIONS.filter((a) => refs.has(a.id)),
    evidence: step >= 6,
    resolution: step >= 7,
  };
}

export function displayName(id) {
  const e = getEntity(id);
  return e ? e.name || e.id : id;
}
