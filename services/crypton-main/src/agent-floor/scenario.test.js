import {
  ACTIONS,
  DELEGATIONS,
  EVENTS,
  EVIDENCE,
  GRANTS,
  OUTCOMES,
  PRINCIPALS,
  RESEARCH,
  RESOURCES,
  STEPS,
  TASK,
  TECHNICAL_STEP,
  getEntity,
  visibleAt,
} from './scenario';

const ids = [
  ...PRINCIPALS.map((p) => p.id),
  ...RESOURCES.map((r) => r.id),
  ...GRANTS.map((g) => g.id),
  ...DELEGATIONS.map((d) => d.id),
  ...ACTIONS.map((a) => a.id),
  TASK.id,
];

describe('scenario manifest integrity', () => {
  it('has unique, stable IDs', () => {
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toEqual(expect.arrayContaining(['PR-MGR', 'AG-SCOUT', 'AG-RELAY', 'AG-COURIER', 'GR-001', 'DL-001', 'ACT-001', 'ACT-002', 'ACT-003']));
  });

  it('resolves every reference', () => {
    const known = new Set(ids);
    GRANTS.forEach((g) => { expect(known.has(g.from)).toBe(true); expect(known.has(g.to)).toBe(true); g.allows.forEach((a) => expect(known.has(a.resource)).toBe(true)); });
    DELEGATIONS.forEach((d) => { expect(known.has(d.from)).toBe(true); expect(known.has(d.to)).toBe(true); expect(known.has(d.parentGrant)).toBe(true); });
    ACTIONS.forEach((a) => {
      expect(known.has(a.actor)).toBe(true);
      expect(known.has(a.resource)).toBe(true);
      if (a.target) expect(known.has(a.target)).toBe(true);
      if (a.destination) expect(known.has(a.destination)).toBe(true);
      if (a.coveredBy) expect(known.has(a.coveredBy)).toBe(true);
    });
    EVENTS.forEach((e) => expect(known.has(e.ref)).toBe(true));
    expect(getEntity('GR-001').type).toBe('grant');
  });

  it('keeps a delegation within the authority its parent grant holds', () => {
    DELEGATIONS.forEach((d) => {
      const parent = getEntity(d.parentGrant);
      d.carries.forEach((c) => {
        expect(parent.allows).toContainEqual(c);
      });
      // and only the delegator can have been the parent grant's holder
      expect(parent.to).toBe(d.from);
    });
  });

  it('models the boundary request as attempted only', () => {
    const a = getEntity('ACT-003');
    expect(a.status).toBe('attempted');
    expect(a.executed).toBeNull(); // not modeled: neither executed nor blocked
    expect(a.enforcementRecorded).toBe(false);
    expect(a.coveredBy).toBeNull();
    // no grant or delegation covers export of RS-RECORDS or any export
    [...GRANTS.flatMap((g) => g.allows), ...DELEGATIONS.flatMap((d) => d.carries)].forEach((c) => {
      expect(c.verb).not.toMatch(/export/);
      expect(c.resource).not.toBe('RS-RECORDS');
    });
  });

  it('marks the legitimate actions as executed and inside a grant', () => {
    ['ACT-001', 'ACT-002'].forEach((id) => {
      const a = getEntity(id);
      expect(a.status).toBe('executed');
      expect(a.coveredBy).toBe('GR-001');
    });
  });

  it('reveals facts monotonically as steps advance', () => {
    let prev = visibleAt(0);
    for (let s = 1; s < STEPS.length; s += 1) {
      const cur = visibleAt(s);
      ['events', 'grants', 'delegations', 'actions'].forEach((k) => {
        expect(cur[k].length).toBeGreaterThanOrEqual(prev[k].length);
        prev[k].forEach((x) => expect(cur[k].map((y) => y.id)).toContain(x.id));
      });
      prev = cur;
    }
    expect(visibleAt(0).actions).toHaveLength(0);
    expect(visibleAt(1).grants.map((g) => g.id)).toEqual(['GR-001']);
    expect(visibleAt(2).actions.map((a) => a.id)).toEqual(['ACT-001']);
    expect(visibleAt(3).delegations.map((d) => d.id)).toEqual(['DL-001']);
    expect(visibleAt(4).actions.map((a) => a.id)).toEqual(['ACT-001', 'ACT-002', 'ACT-003']);
    expect(visibleAt(TECHNICAL_STEP).evidence).toBe(true);
    expect(visibleAt(5).evidence).toBe(false);
  });
});

describe('honesty of the copy', () => {
  const copy = [
    ...STEPS.map((s) => s.narration),
    ...EVENTS.map((e) => e.text),
    ...ACTIONS.map((a) => a.plain),
    EVIDENCE.question.text,
    EVIDENCE.question.modeledAnswer,
    ...EVIDENCE.limitations,
    ...RESEARCH.statements,
    RESEARCH.status,
  ].join('\n');

  it('never states that the export was executed, blocked, prevented or stopped as fact', () => {
    // 'whether it ran or was stopped' is the honest uncertainty statement, so 'or was stopped' is allowed
    expect(copy).not.toMatch(/(?<!\bor )\b(was|were|is|got)\s+(blocked|prevented|stopped|denied|successful|successfully\s+(exported|blocked))\b/i);
    expect(copy).not.toMatch(/\bexported (the )?(records|data) (successfully|to the external)/i);
    expect(copy).toMatch(/attempted/i);
  });

  it('labels the scenario as illustrative and the supported outcome as not a finding', () => {
    expect(EVENTS.some((e) => /illustrative/i.test(e.text))).toBe(true);
    expect(EVIDENCE.limitations.join(' ')).toMatch(/fictional/i);
  });

  it('uses exactly the five methodology outcomes and a supported outcome that is one of them', () => {
    expect(OUTCOMES.map((o) => o.id)).toEqual(['ENFORCED', 'VIOLATED', 'UNVERIFIABLE', 'UNREPRESENTABLE', 'NOT_APPLICABLE']);
    expect(OUTCOMES.map((o) => o.id)).toContain(EVIDENCE.supportedOutcome.outcome);
    // the illustration lacks the evidence for ENFORCED or VIOLATED
    expect(['ENFORCED', 'VIOLATED']).not.toContain(EVIDENCE.supportedOutcome.outcome);
  });

  it('does not claim external customer validation', () => {
    expect(RESEARCH.status).toMatch(/not been validated against an external customer/i);
    expect(copy).not.toMatch(/customers? (use|trust|rely)|fully validated|production-ready|certified/i);
  });
});
