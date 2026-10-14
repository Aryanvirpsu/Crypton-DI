# Agent Floor — Design notes

Scope: the `/agent-floor` vertical slice only. Palette and direction were fixed by the brief.

## Atmosphere
A quiet, instrument-like operations floor: dark graphite surfaces, bone-coloured type, one lime signal colour for "the thing to look at", muted coral only for the boundary. Calm, precise, not playful. Robots are small, original, geometric.

## Palette (tokens live once, in the `.af` block of `agentFloor.css`)
| Token | Value | Use |
|---|---|---|
| graphite | `#111713` | page background |
| machine | `#27312A` | panels, floor planes, robot bodies |
| bone | `#F0EEE5` | primary text |
| signal | `#D5F27B` | focus, selection, primary action, executed-within-grant |
| coral | `#D78C76` | the boundary, the attempted request, errors |
| mist | `#A7B2A6` | secondary text, hairlines |

No hex values outside the token block. Colour is never the only carrier of meaning: each state also has a word or shape (`Executed`, `Attempted`, `REQUEST ONLY`, dashed "no record" stub).

## Type
Headings in DM Serif Display, body in Geist, IDs/labels/statuses in DM Mono (Google Fonts, with system fallbacks). Body 16px minimum, line-height ≥ 1.5.

## Components
- **Stage** — one set of persistent elements (agents, resources, grant/delegation/action chips). Floor and Atlas change the backdrop and the percent positions only, so identity is continuous across views.
- **Manager desk** — the visitor's control surface: assignment, per-agent inspect buttons, next step.
- **Inspector** — the single place facts are shown for any selected ID.
- **Event log** — `role="status"` narration plus an ordered list of events.
- **Evidence panel** — available evidence, missing information, limitations, supported outcome (always labelled illustrative).
- **Inquiry** — prototype form, local validation only, banner says nothing is sent.

## Motion
Short CSS transitions on position and opacity only. No scroll-jacking, no parallax, no WebGL, no auto-playing loops. Under `prefers-reduced-motion` (CSS media query and the `data-motion="reduced"` attribute from `useReducedMotion`) all transitions and animations are removed; scene changes become instant. Nothing that carries a security meaning happens inside an animation callback: facts come from `scenario.js`, state from `state.js`.

## Accessibility rules
- Every control is a real `button` or `a`; visible focus ring in signal lime.
- Controls and chips ≥ 44px tall (chips ≥ 32px, none under 24px).
- Toggles expose `aria-pressed`; scene narration is polite-live.
- A complete non-visual path exists: manager desk + inspector + event log carry every fact the stage shows.
- On narrow screens the stage keeps a 640px minimum width inside a focusable, sideways-scrolling region rather than shrinking touch targets.

## Don'ts
- Don't present outcomes as findings; the scenario is fictional.
- Don't show a request as blocked or executed unless the manifest records it.
- Don't add colours, fonts or motion outside these tokens and rules.
- Don't put copy that implies external customer validation.
- Don't make the inquiry form look like it sends anything.
