# Crypton — Technical Source of Truth (v1.2)

**RAT-6 PROVEN — v0.6 (Provable Multi-Hop Delegation with Scope
Attenuation) is built and tested** (2026-09-02). See §11 for the full
record: signing-principal decision, key model, canonicalization,
attenuation semantics, the chain-verification algorithm, and the RAT-6
acceptance scenario (Human -> Agent A -> Agent B -> Agent C) with its
allowed case and every attack case proven denied.

v1.2 supersedes v1.1 in place (same file — see the note at the top of §11
for why it wasn't renamed) — v1.1's own content (§1-10, the v0.5.1
reconciliation pass and its certification closure) is left intact below,
since it remains accurate; v0.6 is additive to it. **Supersedes the
graph-reconstructed "v1.0"** (`graphify-out/GRAPH_REPORT.md` /
`graphify-out/graph.json`, generated from a Graphify pass over the repo).
That artifact is a *reconstruction* — a graph with EXTRACTED (source-explicit)
and INFERRED (model-reasoned pattern-matches) edges, useful for navigation
and for surfacing questions, not itself a verified spec. This document is
the result of actually tracing v1.0's claims against source code,
migrations, and tests, across three passes: the v0.5.1 Ground Truth +
Security Baseline milestone (2026-08-31, certification closure
2026-09-02), and the v0.6 delegation-chain milestone (2026-09-02). Where
source and this document disagree, source wins; where this document and
anything else disagree, this document wins.

Every claim below is tagged:

- **CONFIRMED FROM SOURCE** — read directly from code/migrations in this pass.
- **CONFIRMED FROM TEST** — a passing test in this repo proves it, cited by name.
- **ARCHITECTURAL DECISION** — a deliberate choice, recorded in `DECISIONS.md`.
- **OPEN QUESTION** — genuinely undetermined; not guessed into a fact.
- **KNOWN LIMITATION** — true, acknowledged, and deliberately not fixed here.

Priority order when this document, `DECISIONS.md`, `docs/architecture.md`,
`CLAUDE.md`, `README.md`, `labs/README.md`, Agent Lab `LEARNING.md`, and
`docs/OMEN_HANDOFF.md` disagree: source code and tests win, then this
document, then the others in the order just listed. `docs/OMEN_HANDOFF.md`
is a **historical snapshot** (frozen at Agent Lab v0.3, 90/90 tests) —
treat every "current state" claim in it as a point-in-time record, not a
statement about the repo today.

---

## 1. Repo-wide facts

| # | Question | Answer | Tag |
|---|---|---|---|
| 1 | Actual current Agent Lab version | **v0.6** (v0.1→v0.2→v0.3→v0.4→v0.5→v0.5.1 shipped historically per `labs/agent-lab/LEARNING.md`'s addenda; v0.6 adds provable multi-hop delegation with scope attenuation — see §11) | CONFIRMED FROM SOURCE |
| 2 | Current passing test baseline | See §2 below | CONFIRMED FROM TEST |
| 22 | Which Technical SOT v1.0 claims were stale/inferred | `docs/architecture.md` said "Agent identity / delegation / agent-security testing — Not implemented" (stale — v0.4/v0.5 landed after that line was written, still not corrected); `labs/README.md` said "Status: v0.4 built" (stale — v0.5/Approval already existed); `docs/OMEN_HANDOFF.md` froze at v0.3/90 tests (correctly self-described as historical, but nothing downstream had since restated the delta); `README.md`s referenced `JWT_SECRET` throughout (now replaced by `JWT_SIGNING_KEY_PEM`/`JWT_PUBLIC_KEY_PEM`, see §6); the graph's `semantically_similar_to` edge between crypton-sdk `Actions` and Agent Lab `Approval` was read by a prior pass as "possible duplication" — investigated in §5, found not to be one. All of the above were corrected as part of this pass — see §9 (Changes Made) | CONFIRMED FROM SOURCE |

---

## 2. Test baseline

### Pre-change baseline (before this milestone's edits)

| Suite | Command | Result |
|---|---|---|
| Agent Lab (pytest) | `./.venv/Scripts/python.exe -m pytest -q` (from `labs/agent-lab/`) | **170 passed**, 0 failed, 1 warning (`InsecureKeyLengthWarning` — a test HMAC key under 32 bytes; harmless, pre-existing), 19.58s |
| crypton-sdk (jest) | `npm test -- --ci` (from `libs/crypton-sdk/`) | **10 passed**, 0 failed |
| crypton-sdk typecheck | `npm run typecheck` | clean, no errors |
| crypton-console (jest) | `CI=true npm test -- --watchAll=false` | **7 passed**, 0 failed |
| crypton-main (jest) | `CI=true npm test -- --watchAll=false --passWithNoTests` | 0 tests (expected — marketing site, no test files) |
| crypton-gateway (`cargo test`) | from `services/crypton-gateway/` | builds clean, **0 tests** (no `#[cfg(test)]` module in `main.rs`), `cargo fmt --check` clean |
| crypton-id (`cargo test`) | from `services/crypton-id/` | **could not run locally** — see below |

### crypton-id: environment gap, not a code failure

`cargo test`/`cargo check` fail during `openssl-sys`'s build script:
*"Could not find directory of OpenSSL installation"* — this Windows dev
machine has no OpenSSL dev headers/libs findable by `openssl-sys` (MSVC
target). Traced with `cargo tree -i openssl-sys`: the dependency chain is
`crypton-identity → webauthn-rs → webauthn-rs-core → openssl-attestation-ca
→ openssl-sys`, a genuine, pre-existing transitive dependency needed for
WebAuthn attestation-certificate verification — not something introduced
or removable in this pass. `cargo fmt --check` (which doesn't run
`build.rs`) passes clean. `.github/workflows/ci.yml` runs `cargo test` on
`ubuntu-latest`, where `openssl-sys` resolves against system packages —
that is the authoritative pass/fail signal for crypton-id and was not
reproducible in this sandbox (no Docker/Linux/Postgres/Redis available
here). This matches `docs/OMEN_HANDOFF.md`'s prior finding of a blocked
Rust toolchain on this same class of machine, though the specific root
cause (missing OpenSSL dev libs for `openssl-sys`, not an MSVC linker
issue per se) is more precise here than that document's characterization.
**Category: missing infrastructure (Windows dev-machine OpenSSL), not a
genuine code failure.**

### Post-change baseline (after this milestone's edits)

| Suite | Result | Delta |
|---|---|---|
| Agent Lab (pytest) | **185 passed**, 0 failed, ~13-22s | +15 (9 new `test_enforcement_boundary.py` scenarios, +6 net in rewritten `test_identity.py` — see §9) |
| crypton-sdk (jest + typecheck) | unchanged, 10 passed, typecheck clean | 0 (untouched this pass) |
| crypton-console (jest) | unchanged, 7 passed | 0 (untouched this pass) |
| crypton-main (jest) | unchanged, 0 tests, `--passWithNoTests` | 0 (untouched this pass) |
| crypton-gateway | unchanged, `cargo check` clean, builds, 0 tests, fmt clean | 0 (untouched this pass) |
| crypton-id | **`cargo check` clean, `cargo test` → 38 passed / 0 failed (incl. all 7 JWT unit tests), `cargo fmt --check` clean** — see §10 for the environment fix that unblocked this | resolved this pass |

---

## 3. Delegation, attenuation, and authorization — traced from source

| # | Question | Answer | Tag |
|---|---|---|---|
| 3 | Single-hop or multi-hop delegation | **As of v0.6: both — multi-hop is now built, single-hop remains the (unchanged) degenerate case.** *(As of v0.5.1, this was single-hop only — see §11 for the full v0.6 record.)* `delegation.py`'s `Delegation` now carries `parent_delegation_id`/`parent_hash`/`issuer_type`/`issuer_id`/`signing_key_id`/`signature` (additive columns — see §11's schema section). A root delegation (`parent_delegation_id IS NULL`, still issued directly by a `HumanIdentity`) behaves identically to the old single-hop model; `delegation.grant_chained_delegation` issues non-root, agent-signed delegations that narrow a parent's authority. | CONFIRMED FROM SOURCE + TEST |
| 4 | Can a delegation reference another delegation | **As of v0.6: yes**, via `parent_delegation_id` + `parent_hash` (both integrity-protected — see §11). *(As of v0.5.1: no.)* | CONFIRMED FROM SOURCE + TEST |
| 5 | Does scope attenuation exist today | **As of v0.6: yes** — `chain.verify_attenuation(parent, child)`, one authoritative function, called at both issuance time and verification time. *(As of v0.5.1: no.)* See §11. | CONFIRMED FROM SOURCE + TEST |
| 6 | What attenuation covers | `allowed_tools` (frozenset subset), `resource_constraints` (per-resource_type exact-id-set subset — no prefix/wildcard matching), `expires_at` (`child <= parent`). Exact equality is valid at every dimension; an ambiguous case (e.g. a resource_type the parent never scoped at all) denies rather than inventing authority. Full detail: §11. | CONFIRMED FROM SOURCE + TEST |
| 7 | Where authorization happens relative to tool execution | Immediately before, inside `agent.py`'s `_handle_tool_call`: unknown-tool check → `authorization.authorize_tool_call` → argument validation (Pydantic) → (if `requires_human_approval`) `approval.verify_and_consume_approval` → `tools.TOOL_FUNCTIONS[call.name](...)`. Every earlier boundary returns before `tools.TOOL_FUNCTIONS` is ever indexed. | CONFIRMED FROM SOURCE (`agent.py:369-532`) |
| 8 | Is authorization fail-closed | **Yes.** As of v0.6, `agent.py`'s boundary 2 calls `chain.verify_delegation_chain`, which wraps its entire check — including the leaf-level `authorization.authorize_tool_call` call it makes internally — in a catch-all that converts any unexpected internal exception into a persisted DENY (`CHAIN_VERIFICATION_ERROR`) rather than letting it crash `run()` uncaught. *(As of v0.5.1: the tool was still unreachable on an internal error, but via an uncaught crash rather than a graceful DENY — a deliberate v0.6 strengthening, not a regression; see §11.)* See `test_authorization_error_prevents_execution` (updated) and `test_delegation_chain.py`. | CONFIRMED FROM SOURCE + TEST |
| 9 | Can a normal AgentRunner path execute a protected tool before authorization | **No** — traced above; every DENY/error path returns before `tools.TOOL_FUNCTIONS[call.name]` is indexed. Proven by 9 new regression tests, `labs/agent-lab/tests/test_enforcement_boundary.py`, each asserting a durable-side-effect row count stays at 0 (or exactly 1 for the positive case) across every denial path enumerated in this milestone's spec. | CONFIRMED FROM TEST |
| 10 | Ordering between authorization and human approval | Fixed: `authorization.authorize_tool_call` (Delegation) always runs first; `approval.verify_and_consume_approval` runs only afterward, only for a tool where `models.requires_human_approval(tool_name)` is `True`, and only if `authorize_tool_call` already returned ALLOW. Neither can substitute for or override the other — a DENY from either is final. Proven directly by the pre-existing `test_valid_approval_never_overrides_a_revoked_delegation` and this pass's `test_authorization_deny_prevents_execution`. | CONFIRMED FROM SOURCE + TEST |
| 11 | Where authorization/approval/delegation traces are persisted | Two separate stores, deliberately: (a) `authorization_decisions` (SQLite, `labs/agent-lab/data/agentlab.db`) — one row per attempted consequential tool call, ALLOW or DENY, append-only, via `authorization.record_decision`; (b) `data/traces/<request_id>.jsonl` — the full step-by-step execution trace (`tracing.Tracer`), a superset covering model calls, tool results, and lifecycle events, not just authz decisions. Both are unencrypted, un-access-controlled local files — see `LEARNING.md`'s v0.4 addendum, "attack surfaces" section, still accurate. | CONFIRMED FROM SOURCE |
| 12 | Current agent ownership model | `agents.Agent.owner_user_id`, set once at `register_agent` time from the calling `HumanIdentity`, never changed after (`agents.py` has no ownership-transfer function). Delegation and Approval both re-verify this ownership on every use (`agents.get_agent` raises `AgentAccessError` for a foreign `agent_id`). | CONFIRMED FROM SOURCE |
| 13 | What constitutes Agent, AgentInstance, HumanIdentity today | `agents.Agent` = a persisted row (`agent_id`, `name`, `version`, `owner_user_id`, `status`, timestamps) — the *application registration*. `identity.AgentInstance` = one `AgentRunner.run()` call's worth of execution (`instance_id`, `agent_id`, `human_user_id`, `session_id`, `task_id`, timestamps, status) — minted fresh every run, never reused. `identity.HumanIdentity` = `(user_id, username, cred_id)`, decoded directly from a verified crypton-id JWT's claims — "not a parallel identity system... it *is* that token, decoded" (identity.py's own docstring, still accurate). | CONFIRMED FROM SOURCE |
| 14 | Any cryptographic identity attached to an agent or instance | **As of v0.6: yes, for `Agent` — no, for `AgentInstance`.** `agent_id` is still `f"agt_{uuid.uuid4().hex}"` (a random string, not itself cryptographic), but an `Agent` may now hold one or more Ed25519 signing keys (`agent_signing_keys`, via `signing_keys.create_signing_key`) — the durable `Agent` is the signing principal (see §11's rationale for why `Agent`, not `AgentInstance`). `instance_id` remains a bare UUID4 with no cryptographic material — deliberately: it's a runtime correlator, never a signing identity. *(As of v0.5.1: no cryptographic identity existed anywhere in Agent Lab.)* | CONFIRMED FROM SOURCE + TEST |
| 15 | How revocation currently works | `delegation.revoke_delegation`: the only function that writes `delegations.status`; one-way `active → revoked` (`lifecycle.validate_delegation_transition`); checked live, off the DB row, on every `authorize_tool_call` — no caching, no background sweep. Revoking never deletes the row (kept as evidence). Agents have a separate, bidirectional `active ⇄ disabled` status (`agents.disable_agent`/`enable_agent`) — disabling stops new `AgentInstance`s from starting (`require_active_agent`), it is not itself a delegation revocation. | CONFIRMED FROM SOURCE + TEST (`test_revoked_delegation_prevents_execution`) |
| 16 | How delegation expiry currently works | `Delegation.expires_at` is mandatory and finite — `grant_delegation` rejects `ttl_seconds <= 0` or `> MAX_TTL_SECONDS` (30 days). Checked live: `authorize_tool_call` compares `time.time() > delegation.expires_at` fresh on every call, no background expiry sweep — an expired-but-`active`-status row simply starts failing `DELEGATION_EXPIRED` the instant the clock passes. `Approval` has its own, much shorter cap (`MAX_APPROVAL_TTL_SECONDS` = 15 minutes) and lazily flips to an explicit `expired` status the first time a stale row is read (`approval._load_and_maybe_expire`). | CONFIRMED FROM SOURCE + TEST (`test_expired_delegation_prevents_execution`) |
| 17 | How WebAuthn exact-action/exact-argument binding works | `approval.compute_argument_hash(tool_name, resource, validated_arguments)` — sha256 over the tool name, resource identity, and a canonically-encoded (sorted keys, no whitespace) dump of already-Pydantic-validated arguments. The WebAuthn step-up challenge itself (`approval.approve`) is derived as `f"{approval_id}:{argument_hash}"`, never caller-supplied — so completing the ceremony is bound to this exact approval's digest, not "some approval or other." At consume time (`verify_and_consume_approval`), the digest is recomputed from the arguments about to actually execute and compared — any mismatch (different tool, different resource, different arguments) denies with `APPROVAL_ARGUMENT_MISMATCH`. | CONFIRMED FROM SOURCE + TEST |
| 18 | How many separate WebAuthn approval implementations really exist | **One real, one fake — not two competing real ones.** See §5 below (full investigation). | CONFIRMED FROM SOURCE |
| 19 | Where is the JWT signing key used | See §6 below (full mapping). | CONFIRMED FROM SOURCE |
| 20 | Which components must possess the signing key | Only `crypton-id` (issuer). Every other component — including `labs/agent-lab` — needs only the public key. See §6. | CONFIRMED FROM SOURCE + TEST |
| 21 | SQL tables/constraints/indexes | See §4 below. | CONFIRMED FROM SOURCE |

---

## 4. SQL schema — authoritative, manually inspected + Graphify-verified

### Production (Postgres, `services/crypton-id/migrations/001-009`)

| Table | Key columns | Constraints/indexes |
|---|---|---|
| `users` | `id uuid PK`, `username text unique`, `role text` (added 009) | `users_role_check CHECK (role IN ('user','admin','super_admin'))` (009), default `'user'` |
| `credentials` | `id uuid PK`, `user_id → users(id) ON DELETE CASCADE`, `credential_id bytea unique`, `public_key bytea` (nullable since 003 — superseded by `passkey`), `passkey text` (002, authoritative serialized WebAuthn credential), `sign_count bigint`, `status text`, `user_agent text` (006) | `credentials_status_check CHECK (status IN ('active','lost','revoked'))` (004→007); indexes on `user_id`, `status` |
| `audit_logs` (005) | `id uuid PK`, `user_id → users(id) ON DELETE SET NULL`, `actor`, `credential_id`, `action`, `detail jsonb`, `outcome`, `created_at` | indexes on `user_id`, `created_at DESC` |
| `recovery_requests` (008) | `id uuid PK`, `user_id → users(id) ON DELETE CASCADE`, `status`, `method`, `approved_by_credential_id uuid` (no FK — survives revocation), `expires_at`, `public_token_hash` (009) | `recovery_status_check CHECK (status IN ('pending','approved','completed','rejected'))`, `recovery_method_check CHECK (method IN ('trusted_device','admin'))`; index on `(user_id, status)` |

No `agents`/`delegations`/`approvals`/`authorization_decisions` tables
exist in Postgres — that's deliberate (invariant 10: labs stay isolated
from production stores), not an oversight.

### Agent Lab (SQLite, `labs/agent-lab/src/agentlab/db.py`, isolated, disposable)

| Table | Key columns | Constraints/indexes |
|---|---|---|
| `customers`, `orders`, `case_notes` | seeded demo business data, each row carries `owner_user_id` | — |
| `agents` (v0.3) | `agent_id PK`, `name`, `version`, `owner_user_id`, `status`, timestamps | `UNIQUE(owner_user_id, name, version)`; index on `owner_user_id` |
| `sessions`, `tasks`, `agent_instances` (v0.2/v0.3) | FK chain `sessions ← tasks ← agent_instances`, `tasks.agent_id → agents`, `agent_instances.agent_id → agents` | indexes on `session_id`/`human_user_id`/`task_id` |
| `delegations` (v0.4) | `delegation_id PK`, `principal_user_id`, `agent_id → agents`, `task_id` (nullable), `allowed_tools`/`resource_constraints` (JSON text), `issued_at`/`expires_at`, `status`, `revoked_at` | indexes on `agent_id`, `principal_user_id` |
| `authorization_decisions` (v0.4) | `decision_id PK`, `decision`, `reason_code`, full attribution tuple, `resource`, `timestamp` | index on `instance_id` |
| `approvals` (v0.5) | `approval_id PK`, `principal_user_id`, `agent_id → agents`, `instance_id`/`task_id` (nullable), `delegation_id → delegations`, `tool_name`, `resource`, `argument_hash`, `issued_at`/`expires_at`, `status`, `approved_at`/`consumed_at`, `webauthn_reference` | indexes on `principal_user_id`, `agent_id` |

### Graphify SQL blind spot — fixed

`tree_sitter_sql` was previously absent, so all 9 crypton-id migrations
contributed 0 nodes to the reconstructed graph. Installed this pass
(`pip install "graphifyy[sql]"` — straightforward, no build issues) and
the graph regenerated with real SQL entities: e.g.
`services_crypton_id_migrations_001_init_users`,
`..._001_init_credentials`, `..._005_audit_logs_audit_logs`,
`..._008_recovery_requests_recovery_requests` now appear as nodes with
`source_file` pointing at the actual migration. `docs/OMEN_HANDOFF.md`
was included in this re-run (no technical reason to exclude it — it was
excluded from an earlier, unrelated `/graphify` invocation at the user's
explicit request in that session, not for any reason that applies here).

---

## 5. WebAuthn duplication — investigated, not duplicated

**Claim under test:** the prior graph reconstruction flagged a
`semantically_similar_to` (INFERRED, 0.85) edge between crypton-sdk's
`Actions` module and Agent Lab's `Approval`/WebAuthn step-up, read by a
previous pass as a possible sign of duplicated implementation.

**Traced from source:**

- `services/crypton-id/src/state.rs` constructs exactly **one**
  `webauthn_rs::Webauthn` instance (`AppState.webauthn`), built once at
  startup. `services/crypton-id/src/routes/auth_webauthn.rs`
  (login/registration) and `services/crypton-id/src/routes/actions.rs`
  (`/actions/challenge`, `/actions/execute` — step-up) both call methods
  on that same shared instance. The only other `WebauthnBuilder::new(...)`
  call sites in the codebase (`devices.rs`, `recovery.rs`, `admin.rs`) are
  inside `#[cfg(test)] mod tests` blocks — local test fixtures, never
  compiled into production.
- `labs/agent-lab/src/agentlab/webauthn.py` defines one interface
  (`WebAuthnVerifier`) and exactly one implementation:
  `FakeWebAuthnVerifier`, which "never talks to an authenticator or to
  crypton-id" (its own docstring) and unconditionally sets
  `WebAuthnEvidence.is_simulated=True`. There is no
  `CryptonIdWebAuthnVerifier` or any other real implementation in the
  codebase.
- The binding semantics differ meaningfully: crypton-id's `/actions/*`
  binds a step-up ceremony to a caller-supplied `intent` string (any
  ASCII-ish label ≤64 chars, meant as an audit tag); Agent Lab's
  `Approval` binds to `compute_argument_hash(tool_name, resource,
  validated_arguments)` — a stricter, more specific digest. A real
  `CryptonIdWebAuthnVerifier` would pass `f"{approval_id}:{argument_hash}"`
  as that `intent` string; the two shapes are *compatible*, not
  identical.

**Conclusion: no unification needed or performed.** One canonical
production WebAuthn engine (crypton-id's); one interface + one clearly-
marked fake in Agent Lab, already the target architecture pattern
(`ApprovalVerifier`-shaped: interface, real impl, test impl) the milestone
asked for — it just doesn't have its "real impl" wired up yet, which is
a distinct, larger integration task (getting a browser-completed WebAuthn
assertion into a synchronous, headless CLI process), not a deduplication.
Recorded in `DECISIONS.md`; full narrative in `labs/agent-lab/LEARNING.md`'s
v0.5.1 addendum.

---

## 6. JWT — old trust model → new trust model

### Before this pass (ARCHITECTURAL DECISION superseded)

`crypton-id` signed with **HS256** (symmetric) using a single
`JWT_SECRET` env var. `AuthUser`'s extractor, and three *other*
independent call sites (`auth_webauthn.rs`'s `authenticated_user_id_from_headers`
and `logout`, `oauth.rs`'s `oauth_complete`) each inlined their own
`jsonwebtoken::decode::<Claims>(..., DecodingKey::from_secret(state.jwt_secret...))`
— four copies of the verification logic, all needing the same secret.
`labs/agent-lab/src/agentlab/identity.py`'s `verify_human_identity` read
the identical `JWT_SECRET` to verify, and its `issue_dev_token` helper
used the same secret to *sign* — meaning Agent Lab (a verifier) was
structurally also a signer, violating `DECISIONS.md`'s invariant 5. This
was tracked, not hidden (see the pre-v0.5.1 "Known tension" note in
`DECISIONS.md`'s history), but never closed.

### After this pass (ARCHITECTURAL DECISION, recorded in `DECISIONS.md`)

**Ed25519 (EdDSA), asymmetric.** `crypton-id` holds the private key
(`JWT_SIGNING_KEY_PEM` / `JWT_SIGNING_KEY_PEM_FILE`) and is the only
component that constructs a `jsonwebtoken::EncodingKey`. Every verifier —
`crypton-id`'s own `AuthUser`/`AdminUser`, and `labs/agent-lab` — holds
only `JWT_PUBLIC_KEY_PEM` / `JWT_PUBLIC_KEY_PEM_FILE`. A single choke
point, `jwt::verify_token(state, token)`, now backs all four
previously-independent verification call sites. Config supports either a
direct PEM env var (dotenvy/python-dotenv unescape `\n` inside a
double-quoted value) or a `{NAME}_FILE` path — used by the generated
local-dev keys and CI's per-run key.

**Who signs:** `crypton-id`'s `auth_webauthn.rs` login-finish and
recovery-completion handlers, via `jwt::issue_jwt`, the only production
call site.
**Who verifies:** `crypton-id`'s `AuthUser`/`AdminUser` extractors, the
optional-auth helper, `logout`, `oauth.rs`'s bearer check (all via
`jwt::verify_token`); `labs/agent-lab`'s `identity.verify_human_identity`.
**Who possesses the private key:** `crypton-id` only.
**What happens if a verifier is compromised:** an attacker who
compromises Agent Lab (or any other verifier) obtains only
`JWT_PUBLIC_KEY_PEM` — cryptographically insufficient to mint a token,
proven structurally by `jwt.rs::public_key_pem_cannot_be_used_as_a_signing_key`
(an SPKI public key PEM does not even parse as a PKCS8 signing key) and
`test_identity.py::test_public_key_cannot_be_used_to_mint_a_token`
(same property, Python side). Compromising `crypton-id` itself remains
full compromise — that was always true and is out of scope for this
migration (root authority has to live somewhere).

**`issue_dev_token`** still exists for local dev, but its `private_key`
parameter is now required with no env-var fallback; the CLI's
`--dev-user` path generates a fresh, ephemeral, throwaway keypair per
invocation (`identity.generate_dev_keypair`) — never crypton-id's real
key, structurally.

**Deliberately not done** (see `DECISIONS.md` for the explicit bounded
scope): no JWKS endpoint / dynamic key discovery, no rotation tooling
(a `kid` header, `"v1"`, is a seam for later, not a working rotation
mechanism — `Validation` still pins one `DecodingKey`), no `iss`/`aud`
claim validation (wasn't present before this migration either — adding it
now would be a claims-shape change, not a signing-algorithm one; a
candidate for a future pass).

---

## 7. Enforcement coverage (threat model — a deployment limitation, not a bug)

```
routed through Crypton's enforcement point  →  enforceable
bypasses Crypton entirely                   →  outside enforcement coverage
```

`crypton-gateway` is a transparent reverse proxy today (routing + CORS +
request-ID + logging) with a **marked, unimplemented** extension point in
`main.rs` for agent-authorization middleware. It cannot enforce traffic
that never reaches it — a direct DB connection, a different network
route, or a second, misconfigured ingress are all outside what any
PEP-based model can see. This is inherent to the architecture, not
something host-level interception/kernel hooks/endpoint agents would fix
(none of which are planned). Documented in `docs/architecture.md`'s new
"Enforcement coverage" section — a design constraint to route deployments
around, not a defect to chase.

---

## 8. Known Limitations (as of this milestone)

*(Resolved since this section was first written: "crypton-id's Rust changes
are unverified by a local compiler/test run" — closed 2026-09-02, see §10.
Left out of the list below because it is no longer true, not because it was
unimportant; §10 is the record of how it was closed.)*

- **Delegation is single-hop; no scope attenuation exists.** v0.6's core
  deliverable, not started here (confirmed by design — see the task's own
  "do not begin v0.6" instruction).
- **Agent/instance identity is not cryptographic** — a bare random ID, no
  keypair or attestation. v0.6's delegation-passport work needs this.
- **No JWKS / key rotation.** One static keypair, `kid: "v1"` as an unused
  seam. Acceptable for closing the mint-vs-verify gap; not production-
  hardened key management.
- **`CryptonIdWebAuthnVerifier` (real WebAuthn wiring for Agent Lab
  approvals) still doesn't exist.** Confirmed not a duplication (§5); the
  actual integration work — bridging a browser WebAuthn ceremony into a
  headless CLI tool — remains future work.
- **Enforcement coverage is inherently partial** (§7) — by design, not a
  bug, but worth re-stating as a limitation a deployer must account for.
- **`iss`/`aud` JWT claims are not validated** (pre-existing gap, not
  introduced or fixed by the asymmetric-signing migration).

---

## 9. What v0.6 should consume from this milestone (documentation only — not built here)

- The single-hop `Delegation` shape (`allowed_tools`, `resource_constraints`,
  `expires_at`) is the natural per-hop scope shape a multi-hop chain
  should narrow monotonically from.
- The now-asymmetric JWT trust model is the correct foundation for a
  delegation-chain proof — a chain of grants is only as trustworthy as
  the identity tokens anchoring each hop, which are no longer
  forgeable-by-any-verifier.
- Agent/instance identity has no cryptographic binding yet (§8) — v0.6's
  delegation-passport work will need to add one; nothing here does.
- `WebAuthnVerifier`'s interface shape is the right pattern to extend for
  a real step-up integration, whenever that's tackled — not touched here.

---

## 10. v0.5.1 certification closure (2026-09-02)

This section records how the one open risk flagged at the end of the
2026-08-31 pass — "crypton-id's Rust changes have never been compiled" —
was closed, and certifies v0.5.1 complete. Nothing in this section changes
the architecture recorded in §§1-9; it only proves it compiles and runs.

### OpenSSL root cause

`cargo tree -i openssl-sys` traced the dependency precisely:
`crypton-identity → webauthn-rs → webauthn-rs-core → webauthn-attestation-ca
→ openssl → openssl-sys` — a genuine, load-bearing dependency needed for
WebAuthn attestation-certificate-chain verification, not something to
remove or work around. `openssl-sys`'s build script failed with *"Could not
find directory of OpenSSL installation"* because this Windows machine had
no OpenSSL **development** package findable: Git for Windows ships
`openssl.exe` as a runtime binary only (`Program Files\Git\{mingw64,usr}\bin`)
— no `include/openssl/*.h` headers, no `.lib` import libraries — and no
`OPENSSL_DIR`/`VCPKG_ROOT`/`PKG_CONFIG_PATH` were set. Separately, and
distinct from the OpenSSL gap: `docs/OMEN_HANDOFF.md`'s older note that the
"Rust toolchain remains BLOCKED: MSVC linker" no longer describes this
machine — VS Build Tools 2022 and Windows SDK 10.0.26100.0 are both
installed, and `rustc`/`cargo` 1.97.1 (`stable-x86_64-pc-windows-msvc`)
link every other dependency (`ring`, `aws-lc-sys`, etc.) without issue. The
`openssl-sys` gap was the only real blocker.

### Environment fix (reproducible)

A working vcpkg installation already existed on this machine
(`C:\dev\vcpkg`) with `openssl:x64-windows-static-md` 3.6.4 already built
(proper `.lib` import libraries and headers present). Setting the standard
`VCPKG_ROOT` environment variable let `openssl-sys`'s built-in vcpkg
auto-detection find it — **no code change, no new Cargo dependency, no
vendored build, no absolute machine-specific path committed anywhere.**
Reproducible on any Windows-MSVC dev machine via the standard vcpkg
workflow: `git clone https://github.com/microsoft/vcpkg`, run
`bootstrap-vcpkg.bat`, `vcpkg install openssl:x64-windows-static-md`, set
`VCPKG_ROOT` to the clone path. This is a session/shell environment
variable, not a repo file — nothing was committed that only works on this
one machine. CI is unaffected: `.github/workflows/ci.yml` runs on
`ubuntu-latest`, where `openssl-sys` resolves against the distro's system
OpenSSL without any of this.

### Rust compilation issues found after OpenSSL was fixed

**None.** `cargo check` succeeded on the first attempt once `VCPKG_ROOT`
was set — zero errors, one pre-existing `dead_code` warning on
`AdminUser.cred_id`/`.role` (unrelated to the JWT migration, present
before this pass). The `jsonwebtoken` 9.3.1 API usage
(`EncodingKey::from_ed_pem`, `DecodingKey::from_ed_pem`, `Algorithm::EdDSA`,
`Header::new(Algorithm::EdDSA)`) was correct as reasoned about in the prior
pass, confirmed now by an actual compile against the exact installed crate
version, not just documentation.

### Full results (this pass, this machine)

| Suite | Command | Result |
|---|---|---|
| crypton-id | `cargo check` (from `services/crypton-id`, `VCPKG_ROOT` set) | clean, 1 pre-existing warning |
| crypton-id | `cargo test` (same env, `DATABASE_URL`/`REDIS_URL` pointed at the running `infra/docker-compose.yml` containers) | **38 passed, 0 failed** |
| crypton-id | `cargo fmt --check` | clean |
| crypton-id | release boot test (`cargo build --release`, then run with `JWT_SIGNING_KEY_PEM_FILE`/`JWT_PUBLIC_KEY_PEM_FILE` pointed at a freshly `openssl genpkey`-generated Ed25519 pair) | started cleanly; migrations applied; `GET /health` → 200; `GET /me` with a garbage bearer token → 401 via the live `jwt::verify_token` path (no panic) |
| crypton-gateway | `cargo check`, `cargo test`, `cargo fmt --check` | clean; 0 tests (proxy-only, expected); fmt clean |
| Agent Lab | `./.venv/Scripts/python.exe -m pytest -q` | **185 passed** — matches the prior baseline exactly, no regression |
| Agent Lab | `pytest tests/test_enforcement_boundary.py -v` (re-run specifically) | **9/9 passed** — the fail-closed pre-execution boundary still holds after the JWT refactor; `agent.py`/`authorization.py`/`approval.py`/`delegation.py` are untouched by this pass (`git diff --stat` on those four files is empty) |
| crypton-sdk | `npm test -- --ci`, `npm run typecheck` | **10 passed**, typecheck clean — unchanged |
| crypton-console | `CI=true npm test -- --watchAll=false` | **7 passed** — unchanged |

The 7 JWT-specific `cargo test` cases, individually: `issue_jwt_roundtrips_claims`,
`decode_fails_with_wrong_public_key`, `public_key_pem_cannot_be_used_as_a_signing_key`,
`token_signed_by_a_different_private_key_is_rejected`, `tampered_claims_are_rejected`,
`decode_rejects_expired_token`, `decode_accepts_token_at_the_expiry_boundary` — all
passed.

### Runtime configuration, verified live

`config.rs::read_pem_env` supports `{NAME}_FILE` (read a path from disk —
real newlines, no escaping) or `{NAME}` directly (dotenvy unescapes `\n`
inside a double-quoted `.env` value). `start.sh`/`run.bat` use the reliable
file-loading path exclusively: they generate a fresh Ed25519 keypair to
`jwt_signing_key.local.pem`/`jwt_public_key.local.pem` (gitignored via the
existing `*.pem` pattern) on first run and point
`JWT_SIGNING_KEY_PEM_FILE`/`JWT_PUBLIC_KEY_PEM_FILE` at them — no fragile
multi-line-PEM-crammed-into-one-`.env`-line path is used by any script.
`.env.example`/`.env.production.example`/`.env.staging.example` show the
inline-PEM form only as a documented, clearly-labeled
(`change_me_generate_your_own_key`) alternative for environments that
prefer real env vars over files. CI generates its own throwaway keypair
into `/tmp` and passes it via the same `_FILE` convention. No real
production private key is committed anywhere — confirmed by grepping the
full diff for `BEGIN PRIVATE KEY`: every match is either a placeholder
string or `jwt.rs`'s `#[cfg(test)]`-gated, doc-commented, admittedly-fake
test fixture (excluded from the release binary, never used outside
`cargo test`).

### Security boundary, re-confirmed

`parse → validate → authorization → required approval → tool execution`
still holds exactly as documented in §3 — the JWT/identity refactor touched
only `identity.py` (verification), never `agent.py`/`authorization.py`/
`approval.py`/`delegation.py` (enforcement), so there is no new or
alternate execution path. Re-run and re-verified: `test_enforcement_boundary.py`,
9/9 passed (table above).

### Certification

**v0.5.1 CERTIFIED — READY FOR v0.6.** All items in the milestone's
Definition of Done are satisfied: `cargo check`/`cargo test`/`cargo fmt
--check` all pass for `crypton-id`; the Ed25519 JWT tests pass; runtime PEM
loading is proven live, not just unit-tested; Agent Lab's full suite and
the enforcement-boundary tests are unregressed; SDK, console, and gateway
remain green; no real secret was committed. Genuine remaining debt (no
JWKS/rotation, no `iss`/`aud` validation, `CryptonIdWebAuthnVerifier` not
built, enforcement coverage inherently partial) is recorded in §8 as
deliberate scope boundaries for a future pass, not blockers to this one.

---

## 11. v0.6 — Provable Multi-Hop Delegation with Scope Attenuation (2026-09-02)

*This section is written directly into the existing v1.1 file rather than
a new `_v1.2.md` file — the header above bumps the document's own version
marker to v1.2; the filename stays `CRYPTON_TECHNICAL_SOT_v1.1.md` because
exactly two other files (`DECISIONS.md`, `labs/agent-lab/LEARNING.md`)
already reference it by that path and a rename would need to update both
consistently, for no benefit over versioning the header instead — the
document's own `## 11.` numbering here is the authoritative version
marker, not the filename.*

The technical wedge itself: can Crypton prove that an action performed by
Agent C was authorized through `Human -> Agent A -> Agent B -> Agent C`,
where every hop is cryptographically attributable and authority can only
narrow downstream? **Yes — proven below, RAT-6.**

### 11.1 Signing-principal decision

**`Agent`, not `AgentInstance`, is the cryptographic principal.** Full
reasoning: `DECISIONS.md`'s "Signing principal" section. Short version:
`Delegation` has always been `agent_id`-scoped, reused across many runs of
the same agent since v0.4; `AgentInstance` is minted fresh every
`AgentRunner.run()` call, so making it the signing principal would force
re-signing every hop on every single run. `Agent` keeps one identity
concept (ownership, delegation subject, signing identity) for one durable
entity, instead of two competing ones.

### 11.2 Key model

Ed25519, via the `cryptography` library (no hand-rolled crypto). New
table `agent_signing_keys` — public key only; private keys are generated
in-process, returned once to the caller, never persisted. Lifecycle:
create / resolve (`signing_keys.get_signing_key_unchecked`, used only by
the chain verifier) / revoke — one-way, mirroring `DelegationStatus`.
**Revocation is retroactive**: a revoked key untrusts every delegation it
ever signed, checked live on every verification, never cached. Not every
agent needs a key — only agents that issue a *child* delegation do (Agent
C in the RAT-6 scenario below never issues one, and has no signing key).
No KMS/HSM, no rotation-with-overlap, no hardware attestation — a
prototype proof of the signing primitive, explicitly not production key
custody (`DECISIONS.md`).

### 11.3 Delegation schema — evolved, not replaced

Seven additive columns on the pre-existing `delegations` table (via
idempotent `ALTER TABLE`, `db.py::_ensure_column` — never a
drop/recreate): `format_version`, `issuer_type`, `issuer_id`,
`parent_delegation_id`, `parent_hash`, `signing_key_id`, `signature`. A
database created before v0.6 gets these added in place; every existing
row reads back as a valid *root* delegation. `grant_delegation` (v0.4's
function) is unchanged for every existing caller. No parallel
"DelegationProof" table was built.

### 11.4 Canonical serialization

`delegation.canonical_delegation_bytes` — deterministic JSON (sorted
keys, no whitespace, sets sorted, `format_version` itself signed) over
every security-relevant field *except* `status`/`revoked_at` (revocation
is a live, separately-checked fact, deliberately outside the signed
envelope — same reasoning crypton-id's own JWTs don't encode "is this
session still valid," the Redis denylist does that live). `delegation_digest`
is sha256 of those bytes — what a child's `parent_hash` binds to.
`tests/test_delegation_signing.py`: 30 tests, proving determinism
(identical logical delegations -> identical bytes, regardless of
Python dict/set construction order) and that every one of 13 covered
fields, mutated individually, both changes the digest and invalidates a
prior signature.

### 11.5 Signature model

Issuer signs `canonical_delegation_bytes` with its Ed25519 private key
(`delegation.sign_delegation`); a verifier resolves the issuer's
*registered* public key (via the delegation's own `signing_key_id`) and
checks it (`delegation.verify_delegation_signature`) — never trusts a
caller-supplied key. Any failure mode (missing signature, wrong key,
tampered bytes, malformed PEM) returns `False`, never raises — the caller
(chain.py) treats "invalid" and "verification itself errored" identically:
both DENY.

### 11.6 Attenuation semantics

One function, `chain.verify_attenuation(parent, child)`, called at both
issuance time (`delegation.grant_chained_delegation`) and verification
time (`chain.py`) — never duplicated.

- **Tools:** `child.allowed_tools <= parent.allowed_tools`. Equal valid;
  narrower valid; broader denies.
- **Resources:** per `resource_type`, `child_ids <= parent_ids`; a
  `resource_type` the parent never scoped at all denies (mirrors
  `authorization.py`'s pre-existing "absent means nothing of that type is
  authorized" rule). No prefix/wildcard matching — the existing model is
  exact id sets, and that's the only unambiguous, safe subset semantic
  for it.
- **Expiry:** `child.expires_at <= parent.expires_at`.

`tests/test_attenuation.py`: 12 tests covering equal/narrower/broader for
every dimension, plus the "parent never scoped this type" and "no
prefix-matching" boundary cases explicitly.

### 11.7 Chain-verification algorithm

`chain.verify_delegation_chain` — the single authoritative verifier.
Never trusts a caller-supplied chain; every row is loaded fresh by ID.

1. Leaf check: `authorization.authorize_tool_call` (v0.4's function,
   called exactly as before — this is what makes a root-only delegation's
   behavior identical to pre-v0.6).
2. If the leaf has a parent, walk upward — for the leaf then each
   ancestor: depth/cycle guard → `format_version` supported → shared root-
   principal check → **signature** (key exists, not revoked, verifies) →
   resolve parent, detect a repeat → **issuer/subject continuity**
   (`current.issuer_id == parent.agent_id`) → **parent-hash match**
   (`current.parent_hash == delegation_digest(parent)`, live) →
   **attenuation**.
3. Root liveness: the delegation with no parent must itself not be
   revoked or expired.

Chain-specific denials (`CHAIN_SIGNATURE_INVALID`,
`CHAIN_PARENT_HASH_MISMATCH`, `CHAIN_ISSUER_CONTINUITY_VIOLATION`,
`CHAIN_ATTENUATION_VIOLATION`, `CHAIN_ANCESTOR_REVOKED`,
`CHAIN_ANCESTOR_EXPIRED`, `CHAIN_CYCLE_DETECTED`,
`CHAIN_DEPTH_EXCEEDED`, `CHAIN_VERIFICATION_ERROR`, others) are persisted
through the *same* evidence ledger v0.4 established
(`authorization.record_decision`) — one decision system, not two.

**A verified emergent property:** `expires_at` is defended by four
independent layers (signature, hash-chain, attenuation ordering, live
clock) — in an honest chain the first three make the fourth practically
unreachable, since attenuation's monotonic-narrowing rule means the leaf
always expires first. `tests/test_delegation_chain.py`'s expiry tests
prove the live check exists and works by deliberately bypassing the other
three, documented as defense-in-depth. The same is true of cycle
detection: a genuine cycle is cryptographically unconstructable without
first breaking a signature (`parent_delegation_id` is itself signed), so
`test_cycle_is_denied` proves the guard the same way.

`MAX_CHAIN_DEPTH = 32` — a generous prototype bound (the milestone needed
>=3), denying rather than recursing unboundedly on overflow.

### 11.8 Human root

`HumanIdentity` gets no new signing key. A root delegation (Human -> Agent)
is attributed by the same mechanism every human-initiated write in this
codebase already uses: an authenticated, JWT-verified `HumanIdentity`
making the call — not a per-human signature. crypton-id's real WebAuthn
already proves human presence for that JWT; a second, delegation-specific
signing identity for humans would duplicate that, for no property RAT-6
needs. A root still gets a `delegation_digest` (so a child can hash-bind
to it); it just isn't Ed25519-signed. Recorded as a deliberate scope
boundary, not hidden — see §12 (Known Limitations, updated below) and
`DECISIONS.md`.

### 11.9 Authorization integration

`agent.py`'s boundary 2 now calls `chain.verify_delegation_chain`, whose
first action is calling `authorization.authorize_tool_call` itself,
unchanged, by the same module-attribute reference agent.py always used —
one authorization system, extended, not two running in parallel. Approval
(v0.5) is untouched, strictly after chain verification, exactly as
before — see 11.10.

Full order: `parse tool call → validate arguments → resolve acting agent
/ delegation → verify full delegation chain → authorization decision →
required approval → execute → trace`.

### 11.10 Approval interaction

Delegation (now: the full chain) answers "is this agent authorized;"
Approval answers "did a human confirm this exact action." Approval is
reached only after chain verification already returned ALLOW — a valid
approval can never rescue an invalid chain, because the code path to
consult it is never entered otherwise. Proven directly:
`test_delegation_chain.py::test_valid_approval_cannot_rescue_an_invalid_chain`
(a fully valid, human-approved Approval sitting in the database; the
underlying delegation is then revoked; execution count stays 0) and the
positive mirror, `test_valid_chain_plus_required_approval_executes_once`.

### 11.11 RAT-6 proof

Scenario (`tests/test_delegation_chain.py::three_hop_chain`), using
Agent Lab's existing tools/resource-constraint shapes rather than invented
`customer.read`-style infrastructure:

```
Human alice
  |  D0: {get_customer, get_order, add_case_note}, {customer_ids: cust_1, cust_2}, T+3h
  v
Agent A
  |  D1: {get_customer, get_order},               {customer_ids: cust_1, cust_2}, T+2h
  v
Agent B
  |  D2: {get_customer},                          {customer_ids: cust_1},         T+1h
  v
Agent C
```

**Allowed case** — Agent C requests `get_customer` on `cust_1`: **ALLOW**,
`delegation_depth == 3`, `authority_path == [human alice, agent A, agent
B, agent C]`. Proven both at the verifier level
(`test_rat6_allowed_action_verifies`) and through the real `AgentRunner.run()`
loop with a call-count spy on `tools.TOOL_FUNCTIONS["get_customer"]`
proving exactly one execution
(`test_rat6_allowed_action_executes_exactly_once_through_the_real_loop`).
Reproduced live, outside the test harness, via the CLI:
`python -m agentlab.cli --chain-verify <D2> --requested-tool get_customer --requested-resource-type customer_ids --requested-resource-id cust_1` → prints the full ROOT/ACTING/DEPTH/AUTHORITY-PATH/DECISION report, `ALLOW`.

### 11.12 Attack cases — all denied, zero execution

| Attack | Mechanism | Reason code |
|---|---|---|
| Permission amplification (`add_case_note`, dropped at D1) | Leaf check | `TOOL_NOT_ALLOWED` |
| Undelegated action (`delete_customer`, never granted anywhere) | Leaf check | `TOOL_NOT_ALLOWED` |
| Resource escape (`cust_2`, narrowed away by D2) | Leaf check | `RESOURCE_NOT_ALLOWED` |
| Ancestor revocation (D1 revoked, D2 untouched) | Chain walk | `CHAIN_ANCESTOR_REVOKED` |
| Ancestor expiry (defense-in-depth — see 11.7) | Chain walk | `CHAIN_ANCESTOR_EXPIRED` |
| Tampered child (D2's resource_constraints edited post-signing) | Signature | `CHAIN_SIGNATURE_INVALID` |
| Parent substitution (D2 re-pointed at D0 instead of D1) | Signature (parent_delegation_id is itself signed) | `CHAIN_SIGNATURE_INVALID` |
| Ancestor content tampered post-issuance (D1 edited directly, D2 untouched) | Parent hash | `CHAIN_PARENT_HASH_MISMATCH` |
| Unauthorized issuer (Agent A tries to issue off D1, whose subject is Agent B) | Issuance-time rejection | `DelegationIssuerMismatchError` |
| Unauthorized issuer, forged row (bypassing issuance validation) | Chain walk | `CHAIN_ISSUER_CONTINUITY_VIOLATION` |
| Revoked signing key (retroactive) | Chain walk | `CHAIN_SIGNING_KEY_REVOKED` |
| Cycle (defense-in-depth — see 11.7) | Chain walk | `CHAIN_CYCLE_DETECTED` |
| Depth exceeded | Chain walk | `CHAIN_DEPTH_EXCEEDED` |
| Verifier internal error | Catch-all | `CHAIN_VERIFICATION_ERROR` |

Every row above is a passing test in `tests/test_delegation_chain.py`,
each asserting `result.allowed is False` and, for the two run-through-the-
real-loop cases (permission amplification, invalid-chain-plus-approval),
a durable side-effect count of exactly 0.

### 11.13 Revocation/expiry proof

Ancestor revocation and ancestor expiry both deny without any explicit
cascade update to descendant rows — the chain walk naturally fails the
moment it reaches the revoked/expired ancestor, live, every time
(`test_rat6_ancestor_revocation_denied`, and the two defense-in-depth
expiry tests in 11.7/11.12).

### 11.14 Trace proof

`agent.py` now records a `chain_verified` trace event on every tool call
(`tracing.Tracer`, unchanged mechanism), carrying: `allowed`,
`root_principal`, `acting_principal`, `delegation_depth`, `authority_path`,
`chain_hash`, `failed_hop`, `denial_reason` — enough to answer "who is the
accountable human, who actually acted, how deep was the chain, and
exactly where did it fail" from the trace alone, without reconstructing
the chain by hand. No private key is ever traced or logged.

### 11.15 Compatibility

A pre-v0.6 (root-only) `Delegation` walks through `chain.verify_delegation_chain`
and returns after the leaf check with `delegation_depth == 1` — identical
decisions, identical reason codes, to what `authorization.authorize_tool_call`
alone produced before v0.6
(`test_root_only_delegation_verifies_identically_to_v05`). All 185
pre-v0.6 tests pass unmodified against the new code path. One test's
*expectations* were updated, not its security property — see 11.9's note
in `DECISIONS.md` on the internal-error-to-DENY strengthening.

### 11.16 Tests

| Suite | Command | Result |
|---|---|---|
| Agent Lab, before this pass | `pytest -q` | 185 passed |
| Agent Lab, after this pass | `pytest -q` | **263 passed** (+78: 8 signing-key, 30 canonicalization/signing, 12 attenuation, 26 chain/RAT-6, 2 CLI-output; 1 pre-existing test's assertions updated, not removed) |
| crypton-id (untouched this pass, re-verified) | `cargo check` / `cargo test` / `cargo fmt --check` | clean / 38 passed / clean |
| crypton-gateway (untouched) | same | clean / 0 tests (proxy-only) / clean |
| crypton-sdk (untouched) | `npm test -- --ci` / `npm run typecheck` | 10 passed / clean |
| crypton-console (untouched) | `npm test -- --watchAll=false` | 7 passed |

### 11.17 What this milestone does not claim

- Agent Lab's WebAuthn remains simulated — `CryptonIdWebAuthnVerifier`
  still doesn't exist. Approval's *ceremony* is unreal; the delegation-
  chain proof above is orthogonal to that and unaffected by it.
- `crypton-gateway` still does not enforce agent authorization — routed
  traffic could later be governed by it; nothing currently is.
- Production agent-key custody does not exist — no KMS/HSM, no
  certificate infrastructure, no hardware attestation.
- MCP integration does not exist.
- No JWKS-style key discovery or rotation-with-overlap for
  `agent_signing_keys` — create/resolve/revoke only.
- The human root is attributed by authenticated-call convention, not a
  cryptographic signature — see 11.8.

None of the above are v0.6 failures — they are the milestone's own
explicit scope boundary (`DECISIONS.md`), carried forward honestly rather
than implied-solved by RAT-6's proof.
