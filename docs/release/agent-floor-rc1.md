# Agent Floor release candidate 1 (RC1)

Status: **release candidate, not deployed, not pushed, not merged.** Phase 1 (consolidation).
Prepared 2026-10-14. Every result below was produced by a command that was actually run; where something was not run, it says so.

## Identity

| | |
|---|---|
| Branch | `release/agent-floor-rc1` |
| Base | `origin/main` = `dev` = `d882223` (the branch is fast-forwardable from it) |
| Commits on top of base | `f5d048a` AF-001 vertical slice (pre-existing), `8e1f198` inquiries service, `aff0270` dependencies + dev proxy, `8b91b7a` Agent Floor frontend, plus one docs commit |
| Diff vs `origin/main` | 51 files, +7,131 / −4 (42 in `services/crypton-main`, 7 in `services/crypton-inquiries`, `package-lock.json`, `.gitignore`) |
| Not touched | `services/crypton-id`, `services/crypton-gateway`, `services/crypton-main/vercel.json`, `infra/`, `.github/` |

## What is included

- **`services/crypton-main/src/agent-floor/`**: the `/agent-floor` experience. Scroll-driven 3D floor (React Three Fiber), four-moment story, robots, scale reveal, held frame, assessment overlay, and the original 2D stage as the fallback.
- **`services/crypton-inquiries/`**: the private assessment inbox (Node 24+, no runtime dependencies, SQLite, AES-256-GCM at rest).
- **Dev tooling**: `src/setupProxy.js` (dev-only `/api/assessment` proxy), `scripts/start.cjs` (loopback dev server with a host allowlist).
- **Dependencies**: `three`, `@react-three/fiber`, `@react-three/drei` (runtime); `http-proxy-middleware` (dev).

Excluded on purpose: the unrelated uncommitted work in the `Crypton-DI` worktree (landing page GSAP/Lenis/shadcn, marketing and design notes, two local-tooling `.env.example` edits).

## Verification (all executed)

Environment: Windows 11, Node v24.19.0, npm 11.17.0. Browser checks used headless Edge with software rendering (SwiftShader).

| Check | Where | Result |
|---|---|---|
| Lockfile in sync, `npm ci` | fresh clone at a short path | pass, 1428 packages, ~80 s |
| Frontend unit tests (7 suites) | fresh clone | **98 / 98 pass** |
| Inquiry service tests | fresh clone | **20 / 20 pass** |
| Production build, `CI=true npm run build` | fresh clone | exit 0; JS bundle hashes identical to the working-tree build (deterministic) |
| Browser regression suite (36 checks) | fresh-clone build served by the inbox service | **36 / 36 pass**, no console errors |
| Form end to end, desktop and 390×844 phone | same | receipt shown; stored record read back through the private CLI; ciphertext only in the database and WAL; no visitor data in the service log |
| Double-click on submit | same | exactly one record |
| Service killed before submit | same | "We couldn't confirm receipt", details kept, no false success |
| Keyboard Tab order through the closing overlay | same | form, rail, page controls; 0 hidden world labels reached |
| Phone footer / scene-rail overlap | 390×844, 360×740 | fixed (see below) |
| `npm audit` (frontend workspace, production deps) | RC vs `origin/main` lockfile | **identical totals: 92** (5 low / 11 moderate / 73 high / 3 critical), all in the Create React App toolchain; no advisory on `three` or `@react-three/*` |
| Secret / artefact scan of everything committed | RC | none found |

Bundle (gzip): 259.5 kB lazy 3D chunk, 93.7 kB main, 16.5 kB, 8.7 kB CSS.

### Regression fixed in this phase

On a 390×844 phone the closing panel filled the screen and let the form scroll *under* the fixed scene rail: the submit button was clipped by about 12 px and the "How we use your details" link was completely hidden. The panel now ends above the rail (`bottom: 70px` inside the phone breakpoint), with slightly tighter top padding, gap and input padding so the submit button is fully visible at rest on an 844 px screen. On shorter phones the panel scrolls above the rail. Desktop is unchanged.

### Dependency hygiene fixed in this phase

`http-proxy-middleware` was declared as a runtime dependency at `^2.0.9`, which is inside the range of a router host-matching advisory. It is only used by the dev server and the vulnerable option is not used, but it is now a `devDependency` at `^2.0.10`. Lockfile change: 10 lines.

## Remaining blockers for production

These need a decision or an action that is outside this phase (no deploy, no Vercel changes, no database changes were made).

1. **There is no production route for `/api/assessment`.** The Vercel project serves a static single-page app and rewrites every path to `index.html`, so in production the form would reach HTML, show "We couldn't confirm receipt", and capture nothing. The inbox needs an always-on single-instance host. Either serve the build from the inbox process (`INQUIRY_BUILD_DIR`) behind HTTPS, or keep Vercel and add an approved same-origin reverse proxy ahead of the SPA rewrite.
2. **Secrets and storage are not provisioned.** Production needs `INQUIRY_KEY` (secret manager, backed up separately from the database), `INQUIRY_DB` on a durable private volume (not serverless or ephemeral), and an exact HTTPS `INQUIRY_ORIGIN`. A backup and restore drill with the same key has not been done.
3. **Nobody is notified when someone submits.** Delivery means a committed record, not an email. The form tells the visitor "We'll reply", so an operator must be assigned to review the inbox on a schedule, or a notification path added.
4. **Privacy notice.** The form states how details are stored and links "How we use your details"; the wording has not had legal review. Retention is 90 days in the active inbox; independent backup retention is not configured.
5. **No security headers on the inbox-served HTML** (no CSP, `frame-ancestors`, or HSTS). Set them at the ingress.
6. **Vercel settings are unverified for the new build.** The project's root directory, install command and Node version were not inspected. The tested sequence is below.
7. **Test coverage limits.** Browser checks ran in software-rendered headless Edge only: no real GPU, no Safari or Firefox, no physical phone, no automated accessibility audit, no performance measurement on low-end hardware. Frame rate readings were not reliable (the machine was shared with other processes).
8. **Known pre-existing:** the Create React App toolchain carries the 92 advisories above on `origin/main` too. The released browser bundle does not ship those tools, but the toolchain should be replaced when practical.

## Clean deployment build

From a fresh clone of this branch (needs Node 24+ for the inbox; the frontend was built with Node 24.19):

```bash
npm ci --workspace=services/crypton-main
CI=true npm run build --workspace=services/crypton-main     # output: services/crypton-main/build
node --test services/crypton-inquiries/inbox.test.mjs        # backend tests
```

Frontend tests: `cd services/crypton-main && CI=true npx react-scripts test --watchAll=false --runInBand`.

Running the inbox (single-origin option, production values come from the secret manager):

```bash
NODE_ENV=production INQUIRY_KEY=<64 hex> INQUIRY_DB=/private/volume/inquiries.sqlite \
INQUIRY_ORIGIN=https://cryptonid.tech INQUIRY_BUILD_DIR=services/crypton-main/build \
HOST=0.0.0.0 PORT=8091 node services/crypton-inquiries/server.mjs
```

## Git integration

1. Review `release/agent-floor-rc1` as a pull request into `dev` (not directly into `main`). It is a clean fast-forward of `origin/main`.
2. Merge with a merge commit, not a squash, so the inbox service can be reverted independently of the frontend.
3. Promote `dev` to `main` by a second pull request once the production blockers above are closed.
4. Do not merge locally into the stale local `main` (it is 28 commits behind `origin/main`).
5. The landing-page work in the `Crypton-DI` worktree should be committed on its own branch from `dev`. Expect conflicts in `services/crypton-main/package.json` and `package-lock.json`; resolve by taking both sets of dependencies and re-running `npm install`.
6. After the RC is accepted, retire the `Crypton-DI-floor` worktree's uncommitted copy (its content is identical to the RC apart from the two fixes above).
