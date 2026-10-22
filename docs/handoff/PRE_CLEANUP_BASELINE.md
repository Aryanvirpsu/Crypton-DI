# Pre-cleanup baseline

Recorded 2026-10-21, before any change was made for the CTO-handoff cleanup. Everything here
was read from the repository and the local machine at that moment; nothing is carried over
from older reports.

## 1. Where the founder's working copy stood

| Item | Value |
|---|---|
| Working copy | `C:\Users\aryan vir\Aryan vir\projectsnstuff\Crypton-DI` |
| Branch / HEAD | `verification/big-idea-stabilization` @ `4d0257ecab010ac171d0fc358c6b0511024528a7` |
| Remote | `origin` = `https://github.com/Aryanvirpsu/Crypton-DI.git` (fetch and push) |
| Stashes | none |
| Staged changes | none |
| Unstaged, modified | `labs/agent-lab/.env.example`, `labs/experiments/001-langgraph-approval-drift/.env.local.example`, `package-lock.json`, `package.json`, `services/crypton-main/package.json` |
| Untracked | `DESIGN-IS-2026-10-13/`, `cleanup-inventory-2026-10-12.md`, `docs/marketing/`, `services/crypton-main/components.json`, `services/crypton-main/jsconfig.json`, `services/crypton-main/src/components/` (30 files in total with the modified ones) |
| Ignored, present locally | `graphify-out/`, `node_modules/`, per-lab `.venv/` and `__pycache__/`, `labs/agent-lab/.env`, `labs/agent-lab/data/agentlab.db`, `labs/experiments/001-langgraph-approval-drift/.env.local`, experiment `batch_results_*.jsonl` and `traces/`, `libs/crypton-sdk/dist/` |

This is active work by someone else or by the founder (a shadcn-style component setup in
`crypton-main`, a design-review folder, marketing notes). **None of it belongs to the
cleanup and none of it was touched.** The working copy was left exactly as found; the cleanup
was done in a separate worktree.

### Other worktrees that already existed (left alone)

| Path | Branch | HEAD |
|---|---|---|
| `...\Crypton-DI-floor` | `feat/agent-floor-slice` | `f5d048a` |
| `...\Crypton-DI-leads` | `feat/mobile-ux-overhaul` | `534b48b` |
| `...\Crypton-DI-release` | `release/agent-floor-rc1` | `f3869d5` |
| `...\Crypton-DI-rustls` | `chore/rustls-0.23.46-advisories` | `c644168` |

(`Crypton-DI-benchenvs` and `Crypton-DI-rc` also exist as plain directories next to the repo;
they are not registered worktrees and were not inspected.)

## 2. Which line of development is actually current

The founder's checked-out branch is **not** the current line. `origin/main` is.

| Ref | Commit | Relationship |
|---|---|---|
| `verification/big-idea-stabilization` | `4d0257e` | Merged into `dev` at `252f963` |
| `dev` | `d882223` | Ancestor of `origin/main` |
| `origin/main` | `4344036` (merge of PR #14) | **Current. This is what the cleanup is based on.** |
| `origin/chore/rustls-0.23.46-advisories` | `c644168` | One commit on top of `5a9ee4b`; changes three `Cargo.lock` files only; open PR #13 |

The local `main` branch is 46 commits behind `origin/main`. `origin/main` contains the gateway
token verification and risk-engine integration, the policy engine, `crypton-inquiries`, the
serverless assessment API and Supabase schema, the Agent Floor site, and the dependency
remediation. A handoff built on `4d0257e` would have described code that no longer matches
`main` (for example "the gateway is a plain reverse proxy with 0 tests"). The cleanup branch
was first cut from `4d0257e`, found stale, and re-cut from `origin/main` before any commit
was made; it carries no history from the stale base.

### Open pull requests (GitHub API, read-only, 2026-10-21)

| # | Title | Head -> base | Note |
|---|---|---|---|
| 13 | bump rustls to 0.23.46 (RUSTSEC-2026-0346/0347/0348) | `chore/rustls-0.23.46-advisories` -> `main` | CI all green; fixes the red `rust-audit` job on `main` |
| 10 | Agent Floor 3D experience and private assessment inbox (RC1) | `release/agent-floor-rc1` -> `dev` | Targets `dev`; its content reached `main` through #11 |
| 9 | harden risk-engine integration | `feat/gateway-hardening` -> `feat/risk-engine` | Last updated 2026-06-24; stale |
| 6 | bump follow-redirects in `crypton-main` (Dependabot) | -> `main` | |
| 5 | bump npm group across 3 directories (Dependabot) | -> `main` | Touches `crypton-admin`, which no longer exists |

`gh` is not authenticated on this machine, so PR state came from the public GitHub API.
Review comments and check details beyond job-level results were not read.

## 3. Recoverable snapshot (outside the repository)

Location: `C:\Users\aryan vir\Aryan vir\projectsnstuff\Crypton-DI-snapshots\pre-cleanup-20261021\`

| File | Contents |
|---|---|
| `repo-all-refs.bundle` | Every branch and tag of the repository (`git bundle create --all`, verified with `git bundle verify`) |
| `tracked-changes.patch` | `git diff HEAD --binary` of the 5 modified tracked files |
| `dirty-working-files.zip` | The 30 modified and untracked files, with their relative paths |
| `dirty-file-list.txt`, `status.txt`, `HEAD.txt` | The list, `git status --short`, and the exact HEAD |

Not copied on purpose: ignored files, because they include local secrets (`.env`,
`.env.local`) and large caches. They were not modified, so they are still where they were.

**Restore.**

```sh
# 1. Full history, any branch:
git clone repo-all-refs.bundle restored-crypton && cd restored-crypton
git checkout 4d0257ecab010ac171d0fc358c6b0511024528a7

# 2. The uncommitted work on top of it (either way):
git apply ../tracked-changes.patch            # modified tracked files
unzip -o ../dirty-working-files.zip -d .      # modified + untracked files, exact paths
```

Earlier snapshots from the previous stabilization pass live beside it
(`2026-10-12-pre-stabilization`, `fix-edits-stage`, and
`removed-from-tree-langchain-poc.zip`); they were not changed.

## 4. Where the cleanup was done

| Item | Value |
|---|---|
| Worktree | `C:\Users\aryan vir\Aryan vir\projectsnstuff\Crypton-DI-cleanup` |
| Branch | `chore/cto-handoff-cleanup` |
| Base | `origin/main` @ `4344036` |
| Pushed / PR opened | No. Nothing was pushed, merged or opened. |

Run logs for every verification command are kept outside the repository, in
`...\Crypton-DI-snapshots\cleanup-run-logs\` (earlier, stale-base runs) and
`...\cleanup-run-logs\main\` (the runs this handoff relies on).
