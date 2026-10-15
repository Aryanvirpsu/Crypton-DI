import React from 'react';
import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import AgentFloor from './AgentFloor';
import AssessmentForm from './AssessmentForm';
import { validateInquiry, INITIAL_VALUES } from './inquiryModel';
import { isConfigured, submitAssessmentRequest, toPayload } from './leadClient';
import { STEPS } from './scenario';

const stageIds = (container) =>
  [...container.querySelectorAll('.af-stage button')].map((b) => b.getAttribute('aria-label')).filter(Boolean).sort();

const clickNext = (n = 1) => { for (let i = 0; i < n; i += 1) userEvent.click(screen.getByRole('button', { name: 'Next' })); };
const stepLabel = () => document.querySelector('.af-step-label').textContent;
const toolbar = () => screen.getByRole('group', { name: 'Scenario controls' });

beforeAll(() => { if (!AbortSignal.timeout) AbortSignal.timeout = () => new AbortController().signal; });
const originalFetch = global.fetch;
beforeEach(() => { global.fetch = jest.fn().mockRejectedValue(new Error('unit test: backend unavailable')); });
afterEach(() => { delete window.matchMedia; global.fetch = originalFetch; });

describe('arrival', () => {
  it('opens with one plain statement and two obvious actions, without a loading gate', () => {
    render(<AgentFloor />);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Meet your AI team.');
    expect(screen.getByRole('button', { name: 'Watch them work' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Skip to the assessment' })).toHaveAttribute('href', '#af-inquiry');
    expect(screen.getByText('A fictional simulation')).toBeInTheDocument();
    expect(screen.getAllByText(/illustrative/i).length).toBeGreaterThan(0);
  });

  it('offers a way to the assessment from the navigation', () => {
    render(<AgentFloor />);
    const nav = screen.getByRole('navigation', { name: 'Primary' });
    expect(within(nav).getByRole('link', { name: 'Get an assessment' })).toHaveAttribute('href', '#af-inquiry');
  });

  it('has a persistent, labelled navigation and a skip link', () => {
    render(<AgentFloor />);
    expect(screen.getByRole('navigation', { name: 'Primary' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Skip to the scenario' })).toHaveAttribute('href', '#af-scenario');
  });
});

describe('scenario controls', () => {
  it('Take command assigns the task; Next, Previous and Reset move through scenes consistently', () => {
    render(<AgentFloor />);
    expect(stepLabel()).toMatch(/Scene 00 of 07: Arrival/);
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();

    userEvent.click(screen.getByRole('button', { name: 'Watch them work' }));
    expect(stepLabel()).toMatch(/Scene 01 of 07: Assignment/);
    expect(screen.getAllByText(/Summarize the Q3 supplier report/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/External export is not granted/).length).toBeGreaterThan(0);

    clickNext(3);
    expect(stepLabel()).toMatch(/Scene 04 of 07: Boundary/);
    userEvent.click(screen.getByRole('button', { name: 'Previous' }));
    expect(stepLabel()).toMatch(/Scene 03 of 07: Delegation/);

    userEvent.click(screen.getByRole('button', { name: 'Reset' }));
    expect(stepLabel()).toMatch(/Scene 00 of 07: Arrival/);
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
  });

  it('disables Next on the last scene', () => {
    render(<AgentFloor />);
    clickNext(7);
    expect(stepLabel()).toMatch(/Scene 07 of 07: Resolution/);
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('switches between Floor and Atlas and the scene change picks that scene’s default view', () => {
    const { container } = render(<AgentFloor />);
    const view = screen.getByRole('group', { name: 'View' });
    expect(within(view).getByRole('button', { name: 'Floor' })).toHaveAttribute('aria-pressed', 'true');
    userEvent.click(within(view).getByRole('button', { name: 'Atlas' }));
    expect(container.querySelector('.af')).toHaveAttribute('data-view', 'atlas');
    clickNext();
    expect(container.querySelector('.af')).toHaveAttribute('data-view', STEPS[1].defaultView);
    clickNext(4); // scene 05 is the Atlas
    expect(container.querySelector('.af')).toHaveAttribute('data-view', 'atlas');
  });

  it('skips to the technical explanation without playing through the scenario', () => {
    render(<AgentFloor />);
    userEvent.click(screen.getAllByRole('button', { name: /Skip to (the )?technical explanation/i })[0]);
    expect(stepLabel()).toMatch(/Scene 06 of 07: Evidence inspection/);
    expect(screen.getByRole('heading', { name: /Evidence inspector/ })).toBeInTheDocument();
    expect(screen.getByText(/Authority question/)).toBeInTheDocument();
    expect(screen.getByText('Illustrative scenario. Not a finding.')).toBeInTheDocument();
    expect(document.activeElement).toBe(document.getElementById('af-inspector'));
  });
});

describe('keyboard operation', () => {
  it('operates Next with the keyboard', () => {
    render(<AgentFloor />);
    const next = screen.getByRole('button', { name: 'Next' });
    next.focus();
    expect(next).toHaveFocus();
    userEvent.keyboard('{enter}');
    expect(stepLabel()).toMatch(/Scene 01/);
    userEvent.keyboard(' ');
    expect(stepLabel()).toMatch(/Scene 02/);
  });

  it('moves between scenes with the arrow keys inside the scenario, but not while typing', () => {
    render(<AgentFloor />);
    const region = document.getElementById('af-scenario');
    fireEvent.keyDown(within(region).getByRole('button', { name: 'Next' }), { key: 'ArrowRight' });
    expect(stepLabel()).toMatch(/Scene 01/);
    fireEvent.keyDown(within(region).getByRole('button', { name: 'Next' }), { key: 'ArrowLeft' });
    expect(stepLabel()).toMatch(/Scene 00/);
    // typing in the inquiry form must not change scenes (the form is outside the scenario region)
    fireEvent.keyDown(screen.getByLabelText('Name'), { key: 'ArrowRight' });
    expect(stepLabel()).toMatch(/Scene 00/);
  });

  it('reaches every control by tabbing and exposes pressed state on toggles', () => {
    render(<AgentFloor />);
    userEvent.tab(); // skip link
    expect(screen.getByRole('link', { name: 'Skip to the scenario' })).toHaveFocus();
    const pressed = screen.getAllByRole('button').filter((b) => b.hasAttribute('aria-pressed'));
    expect(pressed.length).toBeGreaterThan(3);
  });
});

describe('one source of truth across views', () => {
  it('lists the same actors, resources and grant/delegation/action tags on the Floor and in the Atlas', () => {
    const { container } = render(<AgentFloor />);
    clickNext(4); // boundary scene on the floor
    userEvent.click(screen.getByRole('button', { name: 'Floor' }));
    const floor = stageIds(container);
    userEvent.click(screen.getByRole('button', { name: 'Atlas' }));
    const atlas = stageIds(container);
    expect(atlas).toEqual(floor);
    ['Scout', 'Relay', 'Courier'].forEach((n) => expect(floor.some((l) => l.includes(n))).toBe(true));
    ['GR-001', 'DL-001', 'ACT-001', 'ACT-002', 'ACT-003', 'RS-REPORT', 'RS-RECORDS', 'RS-EXTERNAL'].forEach((id) =>
      expect(floor.some((l) => l.includes(id))).toBe(true));
  });

  it('shows the same facts for an entity whichever view selected it', () => {
    render(<AgentFloor />);
    clickNext(4);
    userEvent.click(screen.getByRole('button', { name: 'Inspect ACT-003' }));
    const fromFloor = document.getElementById('af-inspector').textContent;
    userEvent.click(screen.getByRole('button', { name: 'Atlas' }));
    userEvent.click(screen.getByRole('button', { name: 'Inspect ACT-001' }));
    userEvent.click(screen.getByRole('button', { name: 'Inspect ACT-003' }));
    expect(document.getElementById('af-inspector').textContent).toBe(fromFloor);
  });

  it('does not show facts before the scene that introduces them', () => {
    render(<AgentFloor />);
    expect(screen.queryByRole('button', { name: 'Inspect GR-001' })).not.toBeInTheDocument();
    clickNext();
    expect(screen.getByRole('button', { name: 'Inspect GR-001' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Inspect ACT-003' })).not.toBeInTheDocument();
    clickNext(3);
    expect(screen.getByRole('button', { name: 'Inspect ACT-003' })).toBeInTheDocument();
  });
});

describe('inspection', () => {
  it('inspects an agent from the manager desk', () => {
    render(<AgentFloor />);
    userEvent.click(screen.getByRole('button', { name: 'Relay' }));
    const insp = document.getElementById('af-inspector');
    expect(within(insp).getByRole('heading', { name: /Relay/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Relay' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('inspects the grant and the delegation and links the delegation to its parent grant', () => {
    render(<AgentFloor />);
    clickNext(3);
    userEvent.click(screen.getByRole('button', { name: 'DL-001' }));
    const insp = document.getElementById('af-inspector');
    expect(within(insp).getByText(/Delegation/)).toBeInTheDocument();
    expect(within(insp).getByText('summarize RS-REPORT')).toBeInTheDocument();
    userEvent.click(within(insp).getByRole('button', { name: 'GR-001' }));
    expect(within(document.getElementById('af-inspector')).getByText(/External export is not granted/)).toBeInTheDocument();
  });

  it('describes the boundary request as attempted, never executed, with execution and enforcement not modeled', () => {
    render(<AgentFloor />);
    clickNext(4);
    userEvent.click(screen.getByRole('button', { name: 'Inspect ACT-003' }));
    const insp = document.getElementById('af-inspector');
    expect(insp).toHaveTextContent(/Attempted/);
    expect(insp).toHaveTextContent(/not recorded/i);
    expect(insp).not.toHaveTextContent(/Executed/);
    expect(insp).toHaveTextContent(/Execution record\s*Not modeled/);
    expect(insp).toHaveTextContent(/Enforcement decision\s*Not modeled/);
    expect(insp).toHaveTextContent(/No grant or delegation in this scenario covers it/);
  });

  it('publishes scenario events in a polite live region', () => {
    render(<AgentFloor />);
    const live = screen.getByRole('status');
    expect(live).toHaveAttribute('aria-live', 'polite');
    clickNext(2);
    expect(live).toHaveTextContent(/Scene 02: Legitimate execution/);
    expect(screen.getByText(/Scout reads the designated report \(executed, within GR-001\)/)).toBeInTheDocument();
  });
});

describe('evidence and resolution', () => {
  it('separates what is available from what is missing and states the supported outcome as illustrative', () => {
    render(<AgentFloor />);
    clickNext(6);
    const insp = document.getElementById('af-inspector');
    expect(within(insp).getByText('Evidence available')).toBeInTheDocument();
    expect(within(insp).getByText('Missing')).toBeInTheDocument();
    expect(within(insp).getByText('Limitations')).toBeInTheDocument();
    expect(insp).toHaveTextContent(/UNVERIFIABLE/);
    expect(insp).toHaveTextContent(/Not a finding/);
    expect(within(insp).queryByText('Resolution')).not.toBeInTheDocument();
    clickNext();
    expect(within(document.getElementById('af-inspector')).getByText('Resolution')).toBeInTheDocument();
  });

  it('keeps the research section clearly separated from the illustration', () => {
    render(<AgentFloor />);
    const section = document.getElementById('af-research');
    expect(within(section).getByText('Illustrative')).toBeInTheDocument();
    expect(within(section).getByText('Crypton research today')).toBeInTheDocument();
    expect(section).toHaveTextContent(/has not been validated against an external customer/);
  });
});

describe('reduced motion', () => {
  it('flags the page as reduced-motion when the visitor asks for it', () => {
    window.matchMedia = jest.fn().mockImplementation((q) => ({ matches: q.includes('reduce'), media: q, addEventListener: jest.fn(), removeEventListener: jest.fn() }));
    const { container } = render(<AgentFloor />);
    expect(container.querySelector('.af')).toHaveAttribute('data-motion', 'reduced');
  });

  it('defaults to full motion and works without matchMedia', () => {
    const { container } = render(<AgentFloor />);
    expect(container.querySelector('.af')).toHaveAttribute('data-motion', 'full');
  });
});

describe('assessment request', () => {
  const fill = (over = {}) => {
    userEvent.clear(screen.getByLabelText('Name'));
    userEvent.type(screen.getByLabelText('Name'), over.name ?? 'Ada Lovelace');
    userEvent.clear(screen.getByLabelText('Work email'));
    userEvent.type(screen.getByLabelText('Work email'), over.email ?? 'ada@example.com');
    if (over.consent !== false) userEvent.click(screen.getByLabelText(/You may contact me/));
  };
  const flush = () => new Promise((r) => setTimeout(r, 0));

  it('ends the page with the request: three fields, an optional disclosure and one button', () => {
    render(<AgentFloor />);
    const section = document.getElementById('af-inquiry');
    expect(within(section).getByRole('heading', { name: 'Your agents have boundaries too.' })).toBeInTheDocument();
    expect(within(section).getByRole('heading', { name: 'Get an assessment' })).toBeInTheDocument();
    ['Name', 'Work email', /Company or project/].forEach((l) => expect(within(section).getByLabelText(l)).toBeInTheDocument());
    expect(within(section).getByText(/Tell us about your agent system/)).toBeInTheDocument();
    expect(within(section).getByRole('button', { name: /Request an assessment/ })).toBeInTheDocument();
  });

  it('says plainly when an endpoint is explicitly unavailable', () => {
    render(<AssessmentForm connected={false} />);
    expect(screen.getByText(/Not connected yet/)).toBeInTheDocument();
  });

  it('reports every missing required field with a link to it, and no success', async () => {
    render(<AgentFloor />);
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    await flush();
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent(/Enter your name/);
    expect(alert).toHaveTextContent(/Enter your work email/);
    expect(alert).toHaveTextContent(/contact you/);
    expect(screen.getByLabelText('Name')).toHaveAttribute('aria-invalid', 'true');
    expect(screen.queryByText(/Request received/)).not.toBeInTheDocument();
  });

  it('rejects a malformed email and requires consent', async () => {
    render(<AgentFloor />);
    fill({ email: 'not-an-email', consent: false });
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    await flush();
    expect(screen.getByRole('alert')).toHaveTextContent(/valid email/);
    expect(screen.getByRole('alert')).toHaveTextContent(/contact you/);
  });

  it('with no endpoint configured, says nothing was sent and keeps what was typed', async () => {
    render(<AssessmentForm connected={false} submit={async () => ({ status: 'not-configured' })} />);
    fill();
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    await flush();
    expect(await screen.findByText(/Nothing was sent\./)).toBeInTheDocument();
    expect(screen.queryByText(/Request received/)).not.toBeInTheDocument();
    expect(screen.getByLabelText('Name')).toHaveValue('Ada Lovelace');
  });

  it('shows success only when the endpoint accepted the request', async () => {
    const submit = jest.fn().mockResolvedValue({ status: 'sent', receipt: '11111111-1111-4111-8111-111111111111' });
    render(<AssessmentForm connected submit={submit} />);
    fill();
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    expect(await screen.findByText('Request received.')).toBeInTheDocument();
    expect(submit).toHaveBeenCalledWith(expect.objectContaining({ name: 'Ada Lovelace', email: 'ada@example.com', consent: true }));
    expect(screen.queryByText(/Not connected yet/)).not.toBeInTheDocument();
  });

  it('on a failed send, says so and keeps the details for another try', async () => {
    const submit = jest.fn().mockResolvedValue({ status: 'error', code: 500 });
    render(<AssessmentForm connected submit={submit} />);
    fill();
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    expect(await screen.findByText(/couldn’t confirm receipt/)).toBeInTheDocument();
    expect(screen.queryByText(/Request received/)).not.toBeInTheDocument();
    expect(screen.getByLabelText('Work email')).toHaveValue('ada@example.com');
  });
});

describe('lead client', () => {
  const values = { ...INITIAL_VALUES, name: ' Ada ', email: 'ada@example.com ', consent: true };

  it('only treats https, same-origin paths and local dev servers as configured', () => {
    expect(isConfigured('')).toBe(false);
    expect(isConfigured('http://example.com/lead')).toBe(false);
    expect(isConfigured('javascript:alert(1)')).toBe(false);
    expect(isConfigured('//evil.example/lead')).toBe(false);
    expect(isConfigured('https://api.example.com/assessment')).toBe(true);
    expect(isConfigured('/api/assessment')).toBe(true);
    expect(isConfigured('http://localhost:8090/api/assessment')).toBe(true);
  });

  it('sends nothing when no endpoint is configured', async () => {
    const fetchImpl = jest.fn();
    await expect(submitAssessmentRequest(values, { endpoint: '', fetchImpl })).resolves.toEqual({ status: 'not-configured' });
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('POSTs trimmed JSON in the body, never in the URL, and requires a committed receipt', async () => {
    const fetchImpl = jest.fn().mockResolvedValue({ ok: true, status: 201, headers: { get: () => 'application/json' }, json: async () => ({ status: 'received', receipt: '11111111-1111-4111-8111-111111111111' }) });
    await expect(submitAssessmentRequest(values, { endpoint: '/api/assessment', fetchImpl })).resolves.toEqual({ status: 'sent', receipt: '11111111-1111-4111-8111-111111111111' });
    const [url, init] = fetchImpl.mock.calls[0];
    expect(url).toBe('/api/assessment');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual(toPayload(values));
    expect(JSON.parse(init.body).name).toBe('Ada');
    expect(url).not.toMatch(/ada/i);
  });

  it('reports an error on a non-2xx response or a network failure', async () => {
    await expect(submitAssessmentRequest(values, { endpoint: '/x', fetchImpl: jest.fn().mockResolvedValue({ ok: false, status: 500 }) }))
      .resolves.toMatchObject({ status: 'error', code: 500 });
    await expect(submitAssessmentRequest(values, { endpoint: '/x', fetchImpl: jest.fn().mockRejectedValue(new Error('offline')) }))
      .resolves.toEqual({ status: 'error' });
  });
});

describe('validateInquiry', () => {
  const ok = { ...INITIAL_VALUES, name: 'Ada', email: 'ada@example.com', consent: true };

  it('accepts a minimal valid inquiry', () => {
    expect(validateInquiry(ok)).toEqual({});
  });

  it.each([
    [{ name: '' }, 'name'],
    [{ name: 'A' }, 'name'],
    [{ name: 'x'.repeat(81) }, 'name'],
    [{ email: '' }, 'email'],
    [{ email: 'a@b' }, 'email'],
    [{ email: 'a b@c.com' }, 'email'],
    [{ context: 'x'.repeat(1001) }, 'context'],
    [{ organization: 'x'.repeat(121) }, 'organization'],
    [{ consent: false }, 'consent'],
  ])('rejects %j on %s', (patch, field) => {
    expect(Object.keys(validateInquiry({ ...ok, ...patch }))).toContain(field);
  });

  it('keeps organization and context optional', () => {
    expect(validateInquiry({ ...ok, organization: '', context: '' })).toEqual({});
  });
});


describe('receipt integrity and failure recovery', () => {
  const values = { ...INITIAL_VALUES, name: 'Ada', email: 'ada@example.com', consent: true, token: 'signed' };
  it.each([
    { ok: true, status: 200, headers: { get: () => 'text/html' } },
    { ok: true, status: 202, headers: { get: () => 'application/json' }, json: async () => ({ status: 'received', receipt: '11111111-1111-4111-8111-111111111111' }) },
    { ok: true, status: 201, headers: { get: () => 'application/json' }, json: async () => ({ status: 'received' }) },
    { ok: true, status: 201, headers: { get: () => 'application/json' }, json: async () => ({ status: 'queued', receipt: '11111111-1111-4111-8111-111111111111' }) },
  ])('never claims delivery from an unverified 2xx response', async (response) => {
    expect((await submitAssessmentRequest(values, { fetchImpl: jest.fn().mockResolvedValue(response) })).status).toBe('error');
  });
  it('contains thrown submission errors and preserves fields', async () => {
    render(<AssessmentForm connected submit={async () => { throw new Error('offline'); }} />);
    userEvent.type(screen.getByLabelText('Name'), 'Ada'); userEvent.type(screen.getByLabelText('Work email'), 'ada@example.com');
    userEvent.click(screen.getByLabelText(/You may contact me/)); userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    expect(await screen.findByText(/couldn’t confirm receipt/)).toBeInTheDocument();
    expect(screen.getByLabelText('Work email')).toHaveValue('ada@example.com');
  });
  it('focuses the first invalid field', () => {
    render(<AssessmentForm connected={false} />);
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    expect(screen.getByLabelText('Name')).toHaveFocus();
  });
  it('explains scope and source-backed deliverables without promising findings', () => {
    render(<AgentFloor />);
    expect(screen.getByText(/discuss one workflow, agree on scope/)).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Assessment deliverables' })).toHaveTextContent('Authority mappingReproducible findingsRemediation guidance');
    expect(screen.getByRole('link', { name: /Vulnerable vs. fixed/ })).toHaveAttribute('href', 'https://github.com/Aryanvirpsu/Crypton-DI/tree/main/labs/validation-benchmark/v0.1');
  });
});


describe('slow connection delivery', () => {
  it('shares an in-flight challenge with submission so receipt retries keep the original token', async () => {
    let completeChallenge;
    const pending = new Promise(resolve => { completeChallenge = resolve; });
    global.fetch.mockReturnValueOnce(pending).mockResolvedValueOnce({ ok: true, status: 201, headers: { get: () => 'application/json' }, json: async () => ({ status: 'received', receipt: '11111111-1111-4111-8111-111111111111' }) });
    render(<AssessmentForm connected />);
    userEvent.type(screen.getByLabelText('Name'), 'Ada');
    userEvent.type(screen.getByLabelText('Work email'), 'ada@example.com');
    userEvent.click(screen.getByLabelText(/You may contact me/));
    userEvent.click(screen.getByRole('button', { name: /Request an assessment/ }));
    expect(global.fetch).toHaveBeenCalledTimes(1);
    completeChallenge({ ok: true, headers: { get: () => 'application/json' }, json: async () => ({ token: 'original-signed-token' }) });
    expect(await screen.findByText('Request received.')).toBeInTheDocument();
    expect(global.fetch).toHaveBeenCalledTimes(2);
    expect(JSON.parse(global.fetch.mock.calls[1][1].body).token).toBe('original-signed-token');
  });
});
