# Agent Floor (vertical slice v1)

Interactive, fully fictional walkthrough of scenario **AF-001**: a manager assigns a report-summary task to SCOUT; SCOUT delegates to RELAY; RELAY then asks COURIER to export records outside the building, which nothing granted. Served at `/agent-floor` inside `services/crypton-main`.

## Run
```bash
cd services/crypton-main
npm start            # open http://localhost:3000/agent-floor
```
Production build: `CI=true npm run build`. Tests: `CI=true npx react-scripts test --watchAll=false src/agent-floor`.

## The visitor flow (story mode)
One scene, one idea, one obvious next action. The eight technical scenes are grouped into four moments (`story.js`):

| Moment | Scenes | On screen |
|---|---|---|
| 01 Enter | 00 | "Meet your AI team." Two actions: *Watch them work*, *Skip to the assessment*. |
| 02 Follow | 01–03 | One line per scene: the job, the work, the handoff. |
| 03 Discover | 04–06 | The request, the held frame, "But was it allowed to do that?", "And did it actually happen?" |
| 04 Act | 07 | Two beats. **Pullback:** the camera reveals many similar floors, several with a coral boundary seam and a request beacon, under "Now picture every floor you run." **Call to action:** after about 3 s (at once if the visitor asked for the assessment or prefers reduced motion) the world dims behind a scrim and the heading and an elevated form card rise over it. |

The technical layer (inspector, evidence, event log, task vs grant, Floor/Atlas, legend, IDs on world tags) is one click away: **Inspect details** in the bar, any "See the…" link under a caption, or clicking anything in the world. Research status sits behind one disclosure after the floor. "Get an assessment" in the nav jumps straight to the form from anywhere.

## Assessment requests
`AssessmentForm.jsx` sends to the same-origin `/api/assessment` through `leadClient.js`.
The dependency-free private inbox lives in `services/crypton-inquiries/`; see its README
for operation, security controls, delivery semantics, and the uncompleted production gate.
Run that service locally and restart the frontend once to load `src/setupProxy.js`.
Success requires a committed storage receipt, never just a 2xx response. On ambiguous
network errors the form preserves fields and token so retries do not duplicate requests.
The form offers a compact privacy disclosure; requests are not marketing subscriptions.

The final overlay retains the approved visual treatment and adds one next-step sentence,
three offer-backed deliverables and a public benchmark link (HTTP 200 and benchmark title
verified 2026-10-14). Source: `pilot-pack/v0.1/02-founding-assessment.md` and the two
recorded vulnerable/fixed JSON pairs in `labs/validation-benchmark/v0.1/`.

## Architecture
Four layers, each with one job:

| Layer | Files | Concern |
|---|---|---|
| Scenario and authority model | `scenario.js`, `authorityGraph.js`, `state.js` | Facts, the authority graph at each scene, pure transitions. The only source of security meaning. Any future Atlas renders `graphAt(step)`. |
| Experience director | `experience.js`, `AgentFloor.jsx` | Renderer choice, scroll to scene binding, deep links (`?scene=N`, `?renderer=2d`), keyboard, HUD composition. |
| Robot world (3D) | `world/` (`World3D`, `Robots3D`, `Floor3D`, `Authority3D`, `Labels3D`, `choreography.js`, `palette.js`) | The miniature floor, robots, camera, envelopes, the held frame, the pullback. Presentation only; lazy-loaded chunk. |
| Classic stage (2D) | `Stage.jsx`, `Robots.jsx` | SVG floor and Atlas. Fallback renderer and test harness. |
| Story and forensic UI | `story.js`, `Panels.jsx`, `stageLabels.js`, `AssessmentForm.jsx`, `inquiryModel.js`, `leadClient.js` | Plain-language captions; inspector, evidence, event log, task vs grant (the details drawer); research; the assessment request and its single submission boundary. |

`App.js` mounts it only for the path `/agent-floor`; the existing landing page is unchanged.

## What is real vs illustrative
- Illustrative: every actor, grant, delegation, action, and the evidence in AF-001.
- ACT-003 is *attempted* only. The manifest records no execution and no enforcement decision, so the UI says so ("Not modeled").
- The outcome the evidence could support is **UNVERIFIABLE**, labelled "Illustrative scenario. Not a finding." ENFORCED and VIOLATED would need real evidence and, for VIOLATED, an affirmative trust-boundary check.
- Research section: methodology exercised internally; benchmark cases compare documented vulnerable vs fixed third-party versions; no external customer validation.

## Tests
Manifest integrity and copy honesty, authority-graph derivation (monotonic, typed, the request is attempted with an unrecorded destination leg, Courier never holds authority), world blocking agreeing with the scenario (envelopes match grants, records and destination outside every envelope, Courier never leaves the dock), reducer transitions (next/prev/reset/goto/skip determinism), component behaviour (controls, keyboard, same IDs across Floor/Atlas, inspector facts, evidence, reduced motion), inquiry validation and no-fake-success.

## Limitations
- The 3D world has no automated tests (jsdom has no WebGL). It was reviewed in headless Edge (SwiftShader) screenshots per scene at desktop and phone sizes. Real-GPU performance on low-end phones has not been measured.
- The robots are procedural primitives, not Blender models.
- No automated accessibility audit (axe, Lighthouse) was run; only structural checks and jest tests.
- Production inbox hosting, persistent storage, routing, backups and operator ownership still require approval and verification. Local delivery is not production delivery.
- Source design material (Google doc, Asana, a `crypton-agent-floor/index.html` prototype) was unavailable.

## Integration
Replace the `/agent-floor` path check in `App.js` with a router route when routing is introduced; approve and verify production inbox routing before launch; keep `scenario.js` as the sole fact source if more scenarios are added.
