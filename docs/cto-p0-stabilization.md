# CTO P0 stabilization

Baseline reviewed November 14, 2026. The remote `main` tip was
`43440362041185df7a42e8e30994db07028e969f`. The patch is based on that revision.

The [last full CI run on this commit](https://github.com/Aryanvirpsu/Crypton-DI/actions/runs/38959492440)
passed Rust, frontend and real-stack runtime jobs. The Rust dependency audit
failed. The [November 9 scheduled audit](https://github.com/Aryanvirpsu/Crypton-DI/actions/runs/42655742540)
confirmed additional findings on the same branch.

## Changes

- Identity now returns HTTP 503 on an unavailable or timed-out Redis denylist
  check. The shared check covers authenticated handlers, bearer-authenticated
  enrollment and optional OAuth completion. Explicitly denylisted bearer sessions
  still receive HTTP 401. The regression suite covers the identity extractor and
  the full stack adds a check of identity's response during a Redis outage.
- Updated Rust lockfiles to `rustls` 0.23.46, `rustls-webpki` 0.103.16 and
  `webpki-roots` 1.0.10. Identity uses `openssl` 0.10.78. Release images now use
  `cargo build --locked`.
- The Rust audit gate checks the enabled graph, including development edges and
  all declared risk-engine features, before excluding the lockfile-only SQLite
  finding RUSTSEC-2026-0459. Graph-inspection errors leave the finding actionable.
  SQLx 0.8 resolves the older optional SQLite driver into the lockfile even when
  the Rust services use Postgres. Remove the exclusion when upstream lockfile
  resolution permits a patched SQLite library.
- Applied compatible npm audit fixes to the workspace and standalone marketing
  lockfiles. On `npm audit --omit=dev`, both now report 35 findings, including
  13 high and zero critical. The original workspace scan reported 56 findings,
  including 27 high and three critical.

## Pull requests

| PR | Disposition | Evidence |
| --- | --- | --- |
| [#5](https://github.com/Aryanvirpsu/Crypton-DI/pull/5) | Closed | Removed admin app and all eight requested version changes already present on `main`. |
| [#6](https://github.com/Aryanvirpsu/Crypton-DI/pull/6) | Closed | `follow-redirects` 1.16.0 already locked in both active frontend lockfiles. Later advisories require separate fixes. |
| [#9](https://github.com/Aryanvirpsu/Crypton-DI/pull/9) | Closed | Its head is an ancestor of both `origin/dev` and `origin/main`. |
| [#10](https://github.com/Aryanvirpsu/Crypton-DI/pull/10) | Closed | Release candidate head is an ancestor of `origin/main`, which also carries the later intake changes. |
| [#13](https://github.com/Aryanvirpsu/Crypton-DI/pull/13) | Open | The patch includes its rustls update and newer related fixes. It remains a valid update until overtaken on `main`. |
| [#15](https://github.com/Aryanvirpsu/Crypton-DI/pull/15) | Open | Valid patch for an archived experiment lockfile. Python research dependencies need a verified upgrade path that preserves historical evidence. |

## Reproduction

Use the checked-in lockfiles and the services configured in `.github/workflows/ci.yml`.

```sh
cargo install cargo-audit --locked --version 0.22.2
bash scripts/rust-audit.sh
bash tests/runtime/run.sh
npm ci
CI=true npm run build:sdk
CI=true npm --workspace libs/crypton-sdk test -- --runInBand
CI=true npm --workspace services/crypton-main run build
CI=true npm --workspace services/crypton-main test -- --watchAll=false --runInBand
CI=true npm --workspace services/crypton-console run build
CI=true npm --workspace services/crypton-console test -- --watchAll=false --runInBand
```

For the intake integration suite, set `ASSESSMENT_TEST_DATABASE_URL` to a
disposable PostgreSQL database with sufficient privileges, then run
`npm --workspace services/crypton-main run test:api`.

## Outstanding risks

- Remaining npm findings include dependencies of Create React App's build and
  development server. The automatic forced repair suggests replacing
  `react-scripts` with `0.0.0`, which would break builds. Review remaining paths
  and plan a supported toolchain upgrade.
- Dependabot also reports Python advisories in pinned LangGraph research
  environments. Track patched environments and rerun the recorded experiments
  separately so their original results remain reproducible.
- The production network boundary and deployment configuration require their
  own verification. Local tests exercise the checked-in stack.
