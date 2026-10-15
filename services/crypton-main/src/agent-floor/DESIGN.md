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

## Two renderers, one floor
- **Immersive (default where WebGL works):** a 3D miniature floor (React Three Fiber + Drei) on a sticky stage inside a tall scroll track. The HUD (console, inspector, bar) floats over it.
- **Classic (fallback):** the original 2D SVG stage and side panels. Used when WebGL is unavailable, if the 3D world throws, under `?renderer=2d`, and in jsdom tests.
- Both render the same reducer state, the same panels, and the same accessible names (`stageLabels.js`). Neither decides anything: facts come from `scenario.js`, edges from `authorityGraph.js`, placement from `world/choreography.js`.

## Story first, evidence on request (UX pass, 2026-10-14)
Rule: one scene, one idea, one obvious next action.

| Before | After |
|---|---|
| Always-open console: scenario ID, badge, technical narration, event log, task/grant table, inspect buttons | **Hidden** in an optional details drawer |
| Inspector auto-opened on every scene | Opens only when the visitor asks (Inspect details, a "See the…" link, or a click in the world) |
| Technical narration ("exceeds the modeled grant…") | **Condensed** to one plain line plus one short sub-line (`story.js`, tested for length, jargon and honesty) |
| World chips (GR-001, ACT-003), RS-IDs, "? NO RECORD" always on | Plain place names (You, Report, Records, Outside); IDs appear with details |
| 8-control toolbar (Previous/Next/Reset/progress/label/Floor/Atlas/Skip) | Four moments, â€¹ â€º, Replay, Inspect details. Floor/Atlas moved to the drawer |
| Dense hero: eyebrow, four-line question, paragraph, four actions | "Meet your AI team." plus two actions |
| Outro → research → prototype inquiry after the world | The form **is** the final scene; research behind one disclosure |
| **Preserved:** every robot, movement, camera shot, handoff, envelope, the held frame, Atlas, pullback, deep links, keyboard, reduced motion, the 2D fallback | |

## The ending (CTA overlay, 2026-10-14)
The last scene is a focused overlay, not a panel in a layout. It has two beats, driven by one `cta` flag in `AgentFloor.jsx` (and `data-cta` on the root):
1. **Pullback.** The camera shows scale: other floors, several with the same coral boundary seam and request beacon as the main floor. These are decorative and unlabelled; they say "this shape repeats", not that any real system did anything. One caption: "Now picture every floor you run."
2. **Call to action.** The canvas gets `blur(2px) brightness(.85)` and a scrim fades in (the floor stays visible behind). On the left, an editorial heading and one line. On the right, an elevated card (glass, 20px radius, deep shadow) rises in, 0.35s after the heading. Camera inset is centred and eases there; the camera drifts slowly behind it.

In the CTA state the chrome steps back: the nav keeps only quiet links (the "Get an assessment" button is hidden, you are there); the bar keeps a faint 01-04 rail and a quiet Replay; arrows, Inspect details and the timecode are hidden; world tags are removed from the tab order. The "What's real" note starts a full screen below (24svh margin plus the sticky release), never in the same frame as the form. A compact privacy disclosure sits beneath the button; errors remain explicit and receipts confirm private-inbox storage.

## Robots (3D)
Stylized miniature industrial machines, not humanoids, readable by silhouette alone:
- **Scout:** compact bone-white body on treads, an oversized optical head, lime lens. Reads; scans with a lime cone.
- **Relay:** graphite-and-machine coordinator on a pedestal with two articulated arms. Receives and passes work.
- **Courier:** low wheeled cart with a cargo tray and a coral beacon. The one that would go outside.
Built from primitives in `world/Robots3D.jsx`. A Blender GLB can replace any of them through `ROBOT_MODELS` without touching the controller.

## Motion
- **Scroll is native.** No scroll-jacking and no snapping. Each scene owns an equal slice of the track. Scrolling into a slice selects the scene. Next/Previous/keys jump the page to the slice, so the two can't disagree (`experience.js`).
- **The camera is damped**, follows Scout in scene 02, eases slightly closer as you scroll through a scene, and has a small pointer parallax. The projection is offset so the subject stays clear of the HUD.
- **The held frame:** in scene 04 the request travels from Relay to Courier. Then the world clock eases to zero, the key light dims, a coral spot finds the request, and a letterbox and a "Frame held" slate appear. The frame holds *before* any outcome because the record does too. Courier turns toward the gate but never leaves the dock (a test enforces this).
- **The pullback (scene 07)** reveals other unlabelled floors, then the page continues into the outro, research and inquiry.
- **Reduced motion:** camera cuts instead of moves, no idle animation, no parallax, transitions removed, and the held frame is instant.
- Nothing that carries a security meaning happens only inside an animation. Every fact is also in the console, inspector and event log.

## Accessibility rules
- Every control is a real `button` or `a`; visible focus ring in signal lime.
- Controls and chips ≥ 44px tall (chips ≥ 32px, none under 24px).
- Toggles expose `aria-pressed`; scene narration is polite-live.
- A complete non-visual path exists: manager desk + inspector + event log carry every fact the stage shows.
- On narrow screens the classic stage keeps a 640px minimum width inside a focusable, sideways-scrolling region rather than shrinking touch targets. The immersive layout puts the console in a bottom sheet and opens the inspector as a sheet only when the visitor selects something.
- World tags are real buttons (≥ 24px) with the same names as the 2D stage; in the final pullback they shrink to named markers rather than disappearing.

## Don'ts
- Don't present outcomes as findings; the scenario is fictional.
- Don't show a request as blocked or executed unless the manifest records it.
- Don't add colours, fonts or motion outside these tokens and rules. The 3D world uses the same six colours (`world/palette.js`).
- Don't show Courier leaving the dock, the records moving, or anything arriving outside: none of it is recorded.
- Don't put copy that implies external customer validation.
- Don't make the inquiry form look like it sends anything.

## Production clarity pass (2026-10-14)
Approved robot world and choreography are unchanged. The final copy adds a short next-step explanation, three compact offering deliverables and a quiet verified benchmark link. Simulation timecodes say SIMULATION / DEMO · HELD. Mobile moment labels remain visible; while the final overlay is active it owns scrolling and the document is locked, restored on exit. The optional textarea grows with input instead of creating another scrolling region. Hero/pullback text hides before incoming copy can overlap. Private-inbox receipts replace arbitrary HTTP success.

## Art direction layer (2026-10-14)
Character and finish added on top of the approved experience: no layout, flow, camera or conversion change.

**Robots** (`world/Robots3D.jsx`). Same three recognisable designs, more physical detail, and a restrained role-specific personality.
- *Scout, curious:* treads with road wheels, head band and side sensors, an eyelid that blinks every ~6 s, glances while idle, iris that widens, antenna ball that flashes. Nods when handed the task card and when it hands work on.
- *Relay, coordinated:* shoulder guards, waist struts, a five-light signal bar (slow wave, chase when work changes hands, steady while holding), a double-blink antenna, grippers that open to take work, close to carry it, open to pass it on.
- *Courier, purposeful:* bumper, mudguards, lidar puck, headlamp. Heavier by design (slower accelerate/brake), settles on its springs, dips when the request lands, and its lamp and beacon light up once it has the request.
- *Feedback:* hover, keyboard focus on the name tag, and selection each give a small acknowledgement (nod, lamp, soft floor ring; selection adds one ping). Idle motion runs on world time, so it freezes with the held frame; acknowledgements run on real time, so a held frame still answers. Under reduced motion nothing loops or nods and feedback is a state change.

**Handoffs** (`choreography.CUES`, `Authority3D.jsx`). The token, its trail, the departure ring, the arrival ring and each robot's reaction all read one set of cue times, so a give and its receive cannot drift apart. The boundary receive lands before the held frame (tested).

**Environment** (`world/scenery.js`, `floorArt.js`, `Floor3D.jsx`). A painted floor (panel variation, worn walkways, station footprints, a dashed line inside the boundary) from one generated texture. Desk, terminal, cabinets, hub, dock, gate, bollards, pipe run, cables, crates, barrels, a tool cart. Stations wake with the story: the desk while the task is set, the terminal (scrolling text, reading lamp, light pool) while Scout reads, the hub ring faster while work changes hands, the dock once Courier has the request. Records-cabinet lamps take slow turns and mean nothing about access. All static scenery is merged by material (`merge.jsx`).

**Scale reveal** (`world/ecosystem.js`, `Ecosystem3D.jsx`). Twelve far floors in four work-area variants at different activity levels, running on the live clock so they carry on while the main floor is held. Four of twelve carry the illustrated signal: a request that travels to a boundary seam and waits there, with a beacon beyond it. It never crosses, nothing is labelled, and `ecosystem.test.js` fails if more than 40% of floors carry it.

**Performance** (same scene, software rendering; frame rate is only comparable on a quiet machine). Draw calls 196 / 109 / 174 / 209 / 299 became 158 / 114 / 141 / 170 / 170 for scenes 0 / 2 / 3 / 4 / 7; triangles at scene 7 went from 75.6k to 53.1k; geometries from 267 to 127; textures from 3 to 7 (small generated canvases). `?perf=1` publishes `window.__afPerf` (calls, triangles, fps) once a second; inert otherwise.

