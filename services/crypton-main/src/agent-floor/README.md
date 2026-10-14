# Agent Floor (vertical slice v1)

Interactive, fully fictional walkthrough of scenario **AF-001**: a manager assigns a report-summary task to SCOUT; SCOUT delegates to RELAY; RELAY then asks COURIER to export records outside the building, which nothing granted. Served at `/agent-floor` inside `services/crypton-main`.

## Run
```bash
cd services/crypton-main
npm start            # open http://localhost:3000/agent-floor
```
Production build: `CI=true npm run build`. Tests: `CI=true npx react-scripts test --watchAll=false src/agent-floor`.

## Architecture
| File | Concern |
|---|---|
| `scenario.js` | Facts and timeline. One manifest with stable IDs (PR-/AG-/RS-/GR-/DL-/ACT-/EV-/AQ-/EVI-). The only source of security meaning. |
| `state.js` | Pure reducer: NEXT, PREV, RESET, GOTO, SKIP_TO_TECHNICAL, SELECT, SET_VIEW. |
| `Stage.jsx`, `Robots.jsx` | Floor and Atlas rendering (SVG backdrop + positioned buttons). Presentation only. |
| `Panels.jsx` | Manager desk, inspector, evidence panel, event log, research panel. |
| `inquiryModel.js`, `Inquiry.jsx` | Prototype inquiry: validation and form. |
| `AgentFloor.jsx`, `agentFloor.css` | Composition, keyboard handling, reduced-motion hook, styles. |

`App.js` mounts it only for the path `/agent-floor`; the existing landing page is unchanged.

## What is real vs illustrative
- Illustrative: every actor, grant, delegation, action, and the evidence in AF-001.
- ACT-003 is *attempted* only. The manifest records no execution and no enforcement decision, so the UI says so ("Not modeled").
- The outcome the evidence could support is **UNVERIFIABLE**, labelled "Illustrative scenario. Not a finding." ENFORCED and VIOLATED would need real evidence and, for VIOLATED, an affirmative trust-boundary check.
- Research section: methodology exercised internally; benchmark cases compare documented vulnerable vs fixed third-party versions; no external customer validation.

## Tests (55)
Manifest integrity and copy honesty, reducer transitions (next/prev/reset/goto/skip determinism), component behaviour (controls, keyboard, same IDs across Floor/Atlas, inspector facts, evidence, reduced motion), inquiry validation and no-fake-success.

## Limitations
- Real-browser screenshots were not captured (the browser pane was hidden in the build session); layout was checked with a DOM geometry probe at 1280/768/390/320 and not with a screenshot review.
- No automated accessibility audit (axe, Lighthouse) was run; only structural checks and jest tests.
- The inquiry form sends nothing and no public contact address is published.
- Source design material (Google doc, Asana, a `crypton-agent-floor/index.html` prototype) was unavailable.

## Integration
Replace the `/agent-floor` path check in `App.js` with a router route when routing is introduced; wire the inquiry to a real endpoint only after a contact channel and privacy notice exist; keep `scenario.js` as the sole fact source if more scenarios are added.
