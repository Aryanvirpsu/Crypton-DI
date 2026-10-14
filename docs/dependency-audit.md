# Dependency audit record

Single record of what the dependency audits found, what was done and what is deliberately
open. Update it in the same change that moves a number here. An advisory is a statement
about a version, not proof of exploitability: every entry below says whether the vulnerable
code is compiled into a shipped binary.

## Rust (RustSec, `cargo audit`)

Gate: `bash scripts/rust-audit.sh` (CI job `rust-audit`, plus a weekly scheduled run so new
advisories surface without a code change). It fails on any vulnerability not listed, with a
justification and a reopen trigger, in `scripts/rust-audit-ignore.txt`; unmaintained, unsound
and yanked warnings are printed but do not fail. Reproduce one lockfile:
`cargo audit --file services/crypton-id/Cargo.lock`.

Method: `cargo audit --json` per lockfile, then `cargo tree -i <pkg>@<ver> -e normal,build
--target all` to decide whether the affected version is in the resolved dependency graph of a
binary ("compiled"), only in dev/test builds, or only listed in `Cargo.lock` (e.g. optional
features that are off).

### Before and after (baseline `3a02135`)

| Lockfile | Vulnerabilities before -> after | Warnings before -> after | Compiled into a shipped binary before -> after |
|---|---|---|---|
| `services/crypton-id` | 13 -> 1 | 7 -> 1 | 12 -> 0 |
| `services/crypton-gateway` | 11 -> 1 | 4 -> 0 | 5 -> 0 |
| `risk-engine` | 5 -> 2 | 3 -> 0 | 0 -> 0 (1 dev/test-only: `crossbeam-epoch`) |
| `policy-engine` | 0 -> 0 | 0 -> 0 | 0 |

Counts are per lockfile and overlap: 17 distinct RustSec IDs before; 2 remain
(`RUSTSEC-2023-0071`, `RUSTSEC-2025-0132`), both lockfile-only.

### What was changed

| Change | Result |
|---|---|
| Lockfile-only, semver-compatible (`f81f2b3`, plus `spin` in the closing commit) | `rustls` 0.23.37/.38 -> 0.23.45; `rustls-webpki` 0.103.9/.12 -> 0.103.15; `aws-lc-rs` 1.16.1 -> 1.18.2 and `aws-lc-sys` 0.38.0 -> 0.46.0; `h2` 0.4.13 -> 0.4.20; `quinn-proto` 0.11.14 -> 0.11.19; `anyhow` -> 1.0.104; `event-listener` -> 5.4.2; `rand` 0.8.5/0.9.2 -> 0.8.8/0.9.5 (identity); `crossbeam-epoch` -> 0.9.21; `spin` 0.9.8 -> 0.9.9 (was yanked) |
| `crypton-id`: `sqlx` 0.7 -> 0.8 (`ec9da31`, manifest change) | clears `RUSTSEC-2024-0363` and the three `rustls-webpki` 0.101.7 advisories (0.101 has no patched release; sqlx 0.7 was the only thing pulling it via `rustls` 0.21). The gateway already used sqlx 0.8. Removes `rustls` 0.21, `rustls-pemfile` 1.x, `paste`, `sct` |

No `Cargo.toml` constraint was loosened except the sqlx line, and no feature was disabled to
silence an advisory.

### Resolved (compiled, network-facing)

All of these were in a shipped binary's resolved graph before and are gone after:
`RUSTSEC-2026-0044`, `-0048` (`aws-lc-sys`, TLS via `axum-server` in `crypton-id`);
`RUSTSEC-2026-0258` (`h2`); `RUSTSEC-2026-0285` (`rustls`, both services);
`RUSTSEC-2026-0049`, `-0098`, `-0099`, `-0104` (`rustls-webpki` 0.103, both services) and the
0.101.7 variants of `-0098`, `-0099`, `-0104` (via sqlx 0.7); `RUSTSEC-2024-0363` (`sqlx`).
Also resolved, lockfile-only (reqwest's HTTP/3 stack is not enabled): five `quinn-proto`
advisories in the gateway lock (`-0185`, `-0338`, `-0339`, `-0340`, `-0342`); and dev-only
`RUSTSEC-2026-0204` (`crossbeam-epoch`).

### Remaining findings (all deferred with a written trigger)

| ID | Package | Reachability | Reason deferred | Reopen when |
|---|---|---|---|---|
| `RUSTSEC-2023-0071` | `rsa` 0.9.10 (Marvin timing sidechannel) | **Lockfile only**, in `crypton-id`, gateway and `risk-engine`: `cargo tree -i rsa` finds nothing in any binary's graph (it is listed because sqlx's macro crate declares the MySQL driver) | No upstream fix exists | a crate enables sqlx's `mysql` feature, `rsa` becomes compilable, or a fix is published |
| `RUSTSEC-2025-0132` | `maxminddb` 0.24.0 (unsound `Reader::open_mmap`) | **Lockfile only**: only the optional `geoip` feature of `risk-engine`, which is off in every build | Only `open_mmap` is affected and the code uses `open_readfile`; the fix (>= 0.27) is an API-breaking upgrade of an unused path | before enabling `geoip`, or if any code calls `open_mmap` |

Open warnings (not failing the gate): `rustls-pemfile` 2.2.0 unmaintained
(`RUSTSEC-2025-0134`), compiled into `crypton-id` through `axum-server`'s TLS support. It
parses PEM files for the optional local TLS listener; the replacement is upstream's
(`axum-server` moving to `rustls-pki-types`). Review at the next `axum-server` upgrade.

### Compatibility notes

- The dependency updates changed no source file. Verified after each group: `crypton-id` 41
  tests (Linux, real Postgres/Redis), gateway 32 unit + 37 integration, `risk-engine` 309,
  `policy-engine` 35, and the 88-check real-stack runtime suite (both images rebuilt from
  source on `rust:1.89`, so the new versions build on the pinned toolchain).
- `crypton-id`'s remaining clippy warnings (6, style: collapsible `if`s, unread fields) are
  pre-existing and unrelated.
- Lockfile resolution used a newer host toolchain than the Dockerfiles' `rust:1.89`; the
  Linux builds above are the evidence that nothing needs a newer compiler.

## npm (build and test tooling)

`npm audit` at the repo root reports 96 findings (4 critical, 76 high, 11 moderate, 5 low) on
`3a02135`. All flow through four direct dependencies: `react-scripts` 5 (Create React App,
end-of-life), `jest`, `jest-environment-jsdom` and `@types/jest`. They are build, test and
dev-server tooling; none is in the browser bundle (`react`, `react-dom`, `web-vitals`, the
SDK). There is no non-breaking fix (`npm audit fix --dry-run` changes nothing). The
advisory-relevant drift in `services/crypton-main/package-lock.json` (`follow-redirects`,
`brace-expansion`) was fixed in `3a02135`. The real remedy is migrating off Create React App
and upgrading Jest, tracked as its own workstream; it is intentionally not part of any
dependency patch.
