/**
 * The visitor-facing story: four moments over the eight scenes, one plain-language line
 * per scene. Presentation copy only. Facts still come from scenario.js; this file must
 * never say more than the manifest records (story.test.js enforces the boundary request
 * is never described as executed, blocked or as a confirmed violation).
 */

export const MOMENTS = [
  { id: 'enter', n: '01', label: 'Enter', first: 0 },
  { id: 'follow', n: '02', label: 'Follow', first: 1 },
  { id: 'discover', n: '03', label: 'Discover', first: 4 },
  { id: 'act', n: '04', label: 'Act', first: 7 },
];

/**
 * One entry per scene. `line` is the one statement on screen, `sub` an optional short
 * follow-up, `detail` an optional "look closer" link into the technical inspector.
 */
export const STORY = [
  {
    moment: 'enter',
    line: 'Meet your AI team.',
    sub: 'Scout reads. Relay coordinates. Courier handles jobs outside. Scroll to watch them share a task.',
  },
  {
    moment: 'follow',
    line: 'You give Scout one job.',
    sub: 'Summarize a report. The glowing area is everything Scout is allowed to touch.',
    detail: { id: 'GR-001', label: 'See the exact permission' },
  },
  {
    moment: 'follow',
    line: 'Scout gets to work.',
    sub: 'It reads the report, inside what it was allowed to do.',
  },
  {
    moment: 'follow',
    line: 'One agent gives another a task.',
    sub: 'Scout hands the summary to Relay, with a smaller slice of its permission.',
    detail: { id: 'DL-001', label: 'See what was handed over' },
  },
  {
    moment: 'discover',
    line: 'Then Relay asks for something else.',
    sub: 'It asks Courier to send the underlying records outside your organisation.',
    detail: { id: 'ACT-003', label: 'Inspect the request' },
  },
  {
    moment: 'discover',
    line: 'But was it allowed to do that?',
    sub: 'Green lines are permissions you gave. The red line is a request none of them cover.',
  },
  {
    moment: 'discover',
    line: 'And did it actually happen?',
    sub: 'The record shows the request, not whether it ran or was stopped. Answering that takes evidence.',
    detail: { id: 'AQ-001', label: 'See the evidence' },
  },
  {
    moment: 'act',
    line: 'Your agents have boundaries too.',
    sub: 'Find out whether their actions stay within them.',
    // shown while the camera pulls back, before the call to action rises
    pullback: 'Now picture every floor you run.',
  },
];

export function momentOf(step) {
  return MOMENTS.find((m) => m.id === STORY[step].moment);
}
