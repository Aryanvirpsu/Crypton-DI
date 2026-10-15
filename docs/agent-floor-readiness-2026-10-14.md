# Agent Floor readiness pass — 14 October 2026

Branch: `feat/agent-floor-slice`. Scope: operational intake and conversion clarity,
with the existing robots, art, camera choreography, scroll story, scale reveal and
final overlay preserved. Existing uncommitted visual work was retained. No push,
deployment, paid provider, production credential or production system change.

## Operational outcome

The existing assessment form had no backend. Its old client accepted any HTTP
2xx as success, including a possible SPA fallback. It now posts to a same-origin
private-inbox service, validates a committed receipt, and never describes an
ambiguous network error as proof that nothing was saved.

The new service is `services/crypton-inquiries/` (Node 24+, no runtime dependencies).
It keeps encrypted records in an ignored SQLite inbox. Public HTTP routes only
issue form challenges and accept requests; there is no public inbox read route.
Operators review requests locally with `npm run inbox` in that service directory.
See its README for key custody, storage, retention, backups and production routing.

Controls: server/client validation, required consent, 8 KiB body limit, signed
hour-long challenges, a honeypot, a short token age check, IP/email request limits,
origin restrictions, encryption at rest, fail-closed storage/key errors, private
CLI readback, 90-day active-inbox retention, and idempotent same-token retries.
The client rejects redirects, HTML responses, queued responses and missing receipts.
A shared in-flight form challenge prevents a slow-load/submission race.

## Offering and evidence

Added one concise next-step sentence and three compact deliverables: authority
mapping, reproducible findings, and remediation guidance. They match
`pilot-pack/v0.1/02-founding-assessment.md` (authority map; report with reproduction
steps/evidence; remediation direction and walkthrough). No guarantee of finding
vulnerabilities or certification was added.

The quiet benchmark link points to:
https://github.com/Aryanvirpsu/Crypton-DI/tree/main/labs/validation-benchmark/v0.1

Its public page returned HTTP 200 and contained the benchmark title on this date.
The local record was checked: MCP 1.27.1 FAIL / 1.27.2 PASS; DBHub 0.22.5 FAIL /
0.22.6 PASS, with negative controls recorded in each JSON evidence pair. These
are reproductions of previously disclosed/fixed issues, not Crypton discoveries,
customer validation or testimonials. The benchmark itself was not rerun here.

## Real-browser measurements

Checked the running page in the Codex Chromium browser at 1440 × 900 and
390 × 844. Saved desktop overlay and mobile receipt screenshots. Browser
attachment/CDP operations were intermittent; do not treat this as an exhaustive
frame-by-frame audit or a physical-phone performance/keyboard test.

- Desktop empty submit: three required-field errors; focus moved to `af-f-name`.
- Desktop transition: outgoing hero `visibility: hidden`, incoming caption
  `visibility: visible`; timecode read `SIMULATION`.
- Mobile: all four meaningful moment labels visible; no horizontal overflow.
- Long optional mobile description: one scrollable container (`af-hud-closing`),
  document overflow locked while the overlay was active, textarea grew to about
  660 px rather than creating a nested scrolling field.
- Leaving the mobile overlay restored document overflow to `visible` and scene 0.
- Desktop and mobile submissions returned receipts that were independently read
  back from private encrypted storage. The desktop record survived an actual
  service restart; the mobile submission succeeded after that restart.

| Test request | Receipt | Private readback |
|---|---|---|
| Desktop synthetic verification | `5c6fc79c-ecbe-4f5a-b510-2149405e36a2` | Matched expected test address; survived restart |
| Mobile synthetic verification | `94ded4eb-2599-4c59-87e7-4513f3428905` | Matched expected test address; 899-character context |

These two records use clearly marked names and `.invalid` addresses and remain
in the local inbox as verification evidence. No email was sent. These are
functional checks, not acquired customer leads or evidence of conversion lift.

## Release boundary

Local delivery is operational. Public production delivery remains unverified:
approve hosting/routing, configure a durable private volume and secret key,
backups/restoration and retention, assign an inbox operator, then verify a real
submission through the public HTTPS origin. The current Vercel SPA rewrite does
not supply an API route; it must be preceded by an approved same-origin backend
proxy, or the frontend build can be served by the tested inbox service.

The service is intentionally single-instance; multi-instance operation needs
shared storage/rate limiting. Basic spam controls do not establish bot-proofing.
No low-end GPU benchmark, physical-device keyboard test, exhaustive accessibility
audit, public deployment check or conversion uplift experiment was performed.

## Regression results

- Frontend: **98/98 tests, 7/7 suites passed**. Final command:
  `CI=true npm --prefix services/crypton-main test -- --watchAll=false --runInBand --testTimeout=30000`.
  Reported wall time: 220.451 s under heavy workstation load; the component suite
  itself reported 12.737 s. An earlier concurrent run hit three timer deadlines;
  the serial rerun passed without changing assertions or source test deadlines.
- Backend: **20/20 tests passed**, 2.265 s in the final standalone run. Covers
  real HTTP delivery/readback, restart persistence, encrypted-at-rest data,
  idempotent retries, type/length/consent validation, forged/expired/early tokens,
  honeypots, persisted email/IP throttles, retention, origin/content-type/body
  limits, storage failure, wrong-key rejection, and public/private route isolation.
- `git diff --check`: passed (Windows line-ending notices only).
- Final production build: **passed, exit 0, compiled successfully** with
  `CI=true GENERATE_SOURCEMAP=false npm --prefix services/crypton-main run build`.
  Source maps were disabled for this local verification build to reduce memory
  pressure. No production configuration was changed. Main JS: 93.63 kB gzip;
  lazy 3D chunk: 259.46 kB gzip; CSS: 8.67 kB gzip. Saved build output:
  `services/crypton-inquiries/.verification/production-build.log` (ignored).
  An earlier source-map-enabled build also passed with the existing missing
  MediaPipe map warning; the browser compatibility database reports it is stale.
  No vendor dependency was replaced to hide those notices.
- After the interruption: the local private-inbox service was restored using the
  existing key. `/api/assessment/challenge` through `localhost:3000` returned
  HTTP 200 with a signed challenge; the service health route returned HTTP 200;
  both recorded browser receipts remained in private storage.
