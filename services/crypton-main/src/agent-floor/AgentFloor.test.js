import React from 'react';
import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import AgentFloor from './AgentFloor';
import { validateInquiry, INITIAL_VALUES } from './inquiryModel';
import { STEPS } from './scenario';

const stageIds = (container) =>
  [...container.querySelectorAll('.af-stage button')].map((b) => b.getAttribute('aria-label')).filter(Boolean).sort();

const clickNext = (n = 1) => { for (let i = 0; i < n; i += 1) userEvent.click(screen.getByRole('button', { name: 'Next' })); };
const stepLabel = () => document.querySelector('.af-step-label').textContent;
const toolbar = () => screen.getByRole('group', { name: 'Scenario controls' });

afterEach(() => { delete window.matchMedia; });

describe('arrival', () => {
  it('shows the headline and the three calls to action without a loading gate', () => {
    render(<AgentFloor />);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('You gave them a task. What did you give them permission to do?');
    expect(screen.getByRole('button', { name: 'Take command' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Explore the research' })).toHaveAttribute('href', '#af-research');
    expect(screen.getByRole('link', { name: 'Discuss an assessment' })).toHaveAttribute('href', '#af-inquiry');
    expect(screen.getAllByText(/illustrative/i).length).toBeGreaterThan(0);
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

    userEvent.click(screen.getByRole('button', { name: 'Take command' }));
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

describe('assessment inquiry (prototype)', () => {
  const fill = (over = {}) => {
    userEvent.clear(screen.getByLabelText('Name'));
    userEvent.type(screen.getByLabelText('Name'), over.name ?? 'Ada Lovelace');
    userEvent.clear(screen.getByLabelText('Email'));
    userEvent.type(screen.getByLabelText('Email'), over.email ?? 'ada@example.com');
    if (over.consent !== false) userEvent.click(screen.getByLabelText(/I agree to be contacted/));
  };

  it('is clearly a prototype and defaults to principal continuity', () => {
    render(<AgentFloor />);
    expect(screen.getByRole('heading', { name: 'Who was actually behind the action?' })).toBeInTheDocument();
    expect(screen.getByText(/Prototype, nothing is sent/)).toBeInTheDocument();
    expect(screen.getByLabelText('Assessment interest')).toHaveValue('principal_continuity');
  });

  it('reports every missing required field with a link to it, and no success', () => {
    render(<AgentFloor />);
    userEvent.click(screen.getByRole('button', { name: /Check and preview/ }));
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent(/Enter your name/);
    expect(alert).toHaveTextContent(/Enter your email/);
    expect(alert).toHaveTextContent(/agree to be contacted/);
    expect(screen.getByLabelText('Name')).toHaveAttribute('aria-invalid', 'true');
    expect(screen.queryByText(/Not sent\./)).not.toBeInTheDocument();
    expect(screen.queryByText(/thank you|submitted|we.ll be in touch/i)).not.toBeInTheDocument();
  });

  it('rejects a malformed email and requires consent', () => {
    render(<AgentFloor />);
    fill({ email: 'not-an-email', consent: false });
    userEvent.click(screen.getByRole('button', { name: /Check and preview/ }));
    expect(screen.getByRole('alert')).toHaveTextContent(/valid email/);
    expect(screen.getByRole('alert')).toHaveTextContent(/agree to be contacted/);
  });

  it('on valid input shows a local preview that says nothing was sent', () => {
    render(<AgentFloor />);
    fill();
    userEvent.click(screen.getByRole('button', { name: /Check and preview/ }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText(/Not sent\./)).toBeInTheDocument();
    expect(screen.getByText(/no way to deliver them/)).toBeInTheDocument();
    expect(screen.queryByText(/thank you|submitted successfully/i)).not.toBeInTheDocument();
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
    [{ interest: 'bogus' }, 'interest'],
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
