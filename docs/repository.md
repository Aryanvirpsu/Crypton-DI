# Repository structure and workflow

## Layout

```text
services/crypton-id        Rust identity service (WebAuthn, EdDSA JWT, devices, recovery, audit). Authoritative issuer.
services/crypton-gateway   Rust gateway: the stack's single external entry point (see deployment-topology.md)
services/crypton-console   Product frontend (React/CRA): register, login, devices, recovery, admin. Uses @crypton/sdk.
services/crypton-main      Public site (React/CRA): Agent Floor experience + assessment-intake form, with a
                           serverless intake API in api/ (Vercel Functions + Supabase Postgres). No product
                           functionality; does not call crypton-id.
services/crypton-inquiries Local / single-host SQLite twin of the intake API (same public contract). Not the production path.
supabase/                  Migration for the assessment_inquiries schema
scripts/                   rust-audit.sh and rust-audit-ignore.txt (the RustSec CI gate)
libs/crypton-sdk           TypeScript client for crypton-id via the gateway
risk-engine                Rust library: deterministic risk scoring. Linked into the gateway.
policy-engine              Rust library: standalone policy evaluators. NOT linked into anything deployed (unique work, kept).
labs/agent-lab             Python agent identity/delegation/authorization/approval/assessment prototype
labs/exp-*, labs/experiments, labs/validation-benchmark   Reproducible experiments and benchmark cases (evidence, do not delete)
assessment-kit, assessment-engagement, docs/methodology   Assessment operating manual, reference engagement, methodology
pilot-pack, crypton-bigidea-2026   Pilot material and point-in-time verification evidence (preserved as captured)
infra/                     docker-compose.yml (dev Postgres/Redis, loopback-bound) and docker-compose.stack.yml (full stack, one entry point)
tests/runtime/             Real-stack verification suite (run.sh)
docs/                      Documentation; see docs/README.md for the map
```

Not restructured on purpose: moving `services/` or `labs/` would break Cargo/Docker/Vercel/Railway
configuration, experiment imports and paths cited in the Decision Log, for no functional gain.

## The two frontends are different products, not duplicates

| | `crypton-console` | `crypton-main` |
|---|---|---|
| Purpose | The product app | Public marketing/landing site |
| Backend | `@crypton/sdk` -> gateway -> `crypton-id` (register, login, devices, recovery, admin, audit) | none of Crypton's identity stack; its own `api/` serves the assessment intake (Supabase Postgres) |
| Routes / UI | 9 screens (Login, Register, Dashboard, Devices, Recovery, Actions, Admin x3) | the Agent Floor experience at `/` (`src/agent-floor/`), plus the earlier `Landing.js` (~88 KB) |
| Dependencies | CRA 5, React 19, `@crypton/sdk` | CRA 5, React 19, 3D libraries for the Agent Floor |
| Deploy | Vercel (own `vercel.json`) | Vercel (own `vercel.json`); delivery through the public production origin was still unverified in `docs/agent-floor-readiness-2026-10-14.md` |
| Activity | consolidated from the former demo+admin apps (2026-08) | Agent Floor and intake work merged to `main` through PRs #11, #12 and #14 |

Decision: keep both. They have different audiences, release cadences and (for `crypton-main`) an
active uncommitted workstream. The overlap is three small, byte-identical boilerplate files
(`Buttons.js`, `hooks.js`, a favicon/`vercel.json` pair), which is cheaper to carry than a shared
frontend package would be to build and maintain; revisit if a third frontend appears or the
shared files start to diverge. Dead files proven unused in `crypton-console` were removed
(`src/App.css`, `src/logo.svg`: no importers). In `crypton-main`, four components that were
defined but never referenced (`ArchDiagram`, `FeatureCard`, `VisionCard`, `CodeBlock` in
`Landing.js`) were removed and its six remaining lint warnings were resolved without changing
rendered output (duplicate `fontSize` key whose later value already won, literal `// ...` text
made an explicit string, two documented `eslint-disable` lines for mount-only effects and a
scroll-only anchor), so both frontends now build with `CI=true`. Known stale behaviour left
alone: `crypton-main` redirects admin-flavoured links to `<console>/admin/login`, a route the
consolidated console does not define.

## Generated and local-only artifacts (never tracked)

`graphify-out/` (Graphify graph, cache, reports; regenerate with `/graphify .`),
`risk-engine/risk_playground.log` (rewritten by `risk-engine` tests), `.stack-secrets/`
(throwaway runtime-suite secrets), `target/`, `node_modules/`, `.env*` (except `*.example`),
`.claude/worktrees/` (tool-managed worktrees; one had been committed as a stray gitlink).
Preserved on purpose even though machine-written: `crypton-bigidea-2026/evidence/` and
`labs/**/evidence/` (captured evidence with provenance), golden snapshots, fixtures.

## Branches

Three long-lived branches:

| Branch | Role | Rule |
|---|---|---|
| `main` | stable, verified | only fast-forwarded from `dev` after the full test list in `testing.md` is green |
| `dev` | integration and active development | the working branch; features branch from and merge back here |
| `lab` | experiments and research | exploratory work lives here; only verified implementation/evidence is promoted to `dev`, never wholesale |

`lab` starts at the same commit as `dev` (no divergent product copy). Short-lived topic
branches are fine; they are deleted after they are merged and their unique commits are
confirmed to be in `dev`. As of 2026-10-13 every local and remote branch was an ancestor of
`dev` (no unique commits) except two Dependabot branches, which carry only lockfile bumps for
frontends that no longer exist or have since changed (`crypton-demo`, `crypton-admin`, and an
older `crypton-main` lockfile): re-run `npm audit` / let Dependabot re-open rather than
merging them. No remote branch had been deleted; at that time `origin/main` and `origin/dev`
were behind local `dev` because nothing had been pushed.

### Actual state, 2026-10-21 (read from refs; the rule table above is no longer followed)

- `origin/main` (`4344036`) is the current line. `origin/dev` (`d882223`) is its ancestor and
  has not moved since 2026-10-13. `lab` exists only locally (`50d57e9`).
- Since then work reached `main` by pull request (#11, #12, #14), not by fast-forward from
  `dev`. The "only fast-forwarded from `dev`" rule is therefore not in effect; nothing in the
  repository enforces it either way.
- Open pull requests: #13 (rustls 0.23.46 advisories, into `main`), #10 (Agent Floor RC1, into
  `dev`; its content reached `main` via #11), #9 (gateway hardening into `feat/risk-engine`,
  last touched 2026-06-24), #5 and #6 (Dependabot npm bumps into `main`).
- Open decision for the CTO: keep the `main`/`dev`/`lab` model and actually follow it, or
  collapse to `main` plus short-lived topic branches, and either way close or retarget #9 and
  #10. See `docs/handoff/CTO-HANDOFF.md`.
