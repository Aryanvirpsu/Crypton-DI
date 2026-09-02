# Architecture decisions

## Non-negotiable invariants (2026-08, agent-identity foundation)

These are constraints on everything built from here forward — Agent Lab,
its eventual security-scanner successor, and any other agent-identity
work. They're "non-negotiable" in the specific sense that violating one
isn't a normal trade-off call the way the cleanup decisions below were;
it re-opens a security question that was deliberately closed. Changing
one requires updating this file in the same change, not a silent drift.

1. **`HumanIdentity`, `AgentIdentity`, and `AgentInstance` are separate
   entities.** A human's identity (who authenticated), an agent
   application's identity (which registered software is running), and an
   agent instance's identity (this specific run) answer three different
   questions and must never be collapsed into one ID. Collapsing them
   would make it impossible to distinguish "the same agent acting for
   two different humans" from "one run gone wrong." Reference
   implementation: `labs/agent-lab/src/agentlab/identity.py`.
2. **LLMs never make final authorization decisions.** A model may
   *request* a tool call; whether that call is permitted is decided by
   deterministic code the model does not control and cannot talk its way
   around. The model is a proposer, never the enforcer.
3. **Authorization happens deterministically immediately before
   execution.** Not at session start, not "probably fine because it was
   allowed earlier in the conversation" — every consequential action is
   re-checked against the current allowlist/policy at the moment it's
   about to run, using plain, auditable code (a dict lookup, a DB
   permission check), not model inference.
4. **Every consequential agent action must be attributable to an
   originating human/authority and agent instance.** No action executes
   without a resolvable `(human_user_id, agent_id, instance_id)` triple
   attached to it, captured at the time of execution — not
   reconstructible later from partial logs.
5. **Agent services must not unnecessarily possess token-signing/root
   credentials.** A service that only needs to *verify* identity should
   hold verification-only material, not the ability to mint new tokens
   or otherwise act as a root authority. See "Known tension" below —
   this one is not yet fully satisfied by the current code and is
   tracked, not silently ignored.
6. **PostgreSQL owns durable security/authority state; Redis only holds
   temporary state.** Users, credentials, roles, and the audit log live
   in Postgres. Redis holds only things that are correct to lose
   (WebAuthn challenges, the JWT logout-denylist) — never the
   system-of-record for who is allowed to do what.
7. **Security audit evidence is separate from ordinary application
   logs.** An audit trail that answers "who did what, when, and was it
   authorized" is a distinct artifact from debug/operational logging —
   different retention, different access control, different purpose. One
   must never be the only copy of the other.
8. **Model providers are replaceable infrastructure.** No code outside a
   `ModelProvider`-shaped abstraction may depend on a specific vendor's
   SDK or wire format. Swapping the LLM behind an agent must never
   require touching the agent loop, authorization, or tracing code.
9. **Agent frameworks may orchestrate but can never become the
   authorization/security boundary.** If a framework (LangChain,
   LangGraph, CrewAI, MCP, or anything adopted later) is introduced, it
   may sequence calls and manage conversation state — but the allowlist
   check, argument validation, and execution boundary stay in code this
   repo owns and can audit, never delegated to framework internals or
   trusted on the framework's say-so.
10. **Lab environments remain isolated from production resources.** Labs
    (`labs/agent-lab/` and anything that follows it) use their own
    isolated data stores and never point at production Postgres, Redis,
    or a production `JWT_SECRET`. A lab is where things are allowed to
    break; production is not allowed to notice.
11. **Every future vulnerability should eventually have attack → oracle →
    fix → regression test.** Finding a security issue isn't done when
    it's patched — it's done when there's a reproducible attack, a way to
    detect it (oracle), the fix, and a regression test that fails if the
    fix is ever reverted. This is the standard the eventual
    agent-security scanner is held to, and the standard any
    hand-found issue should be brought to as well.
12. **The existing WebAuthn/passkey core should not be rewritten without
    a concrete security reason.** `crypton-id`'s WebAuthn
    register/login/recovery implementation is real, working, and
    load-bearing. "It would be cleaner" or "agent work wants a different
    shape" are not sufficient reasons to touch it — a specific
    vulnerability, spec compliance gap, or concrete new requirement is.

### Current system boundaries

Three language boundaries exist in this repo on purpose — not
accidental sprawl, and not to be casually blurred:

| Language | Components | Role |
|---|---|---|
| **Rust** | `crypton-id`, `crypton-gateway` | The identity/security core and the network edge. WebAuthn, JWT issuance/verification, audit log, admin, and the proxy in front of them. This is where root authority actually lives. |
| **TypeScript** | `crypton-sdk`, `crypton-console` | The client side of that identity system — the only code that speaks to `crypton-id` over HTTP (`crypton-sdk`), and the authenticated product UI built on it (`crypton-console`). |
| **Python** | `labs/agent-lab/` and future security-research tooling | Agent-identity experimentation and (eventually) attack tooling. Verifies the Rust-issued identity locally but does not extend, replace, or gain authority over it. Isolated data (SQLite), isolated from production per invariant 10. |

A new component belongs in whichever boundary matches its role, not
whichever language is most convenient — agent/security research stays
Python and stays out of the identity core; anything that needs to *issue*
identity or hold root/signing authority stays in `crypton-id` (Rust).

### Known tension against these invariants — RESOLVED 2026-08 (v0.5.1)

- **Invariant 5 vs. JWT signing — CLOSED.** See "JWT: asymmetric signing
  (Ed25519) replaces the shared HS256 secret" below. The paragraph that
  used to be here described `crypton-id` signing with HS256 (a symmetric
  algorithm, so any verifier — including `labs/agent-lab`, via
  `issue_dev_token` — was also a de facto signer). That is no longer
  true: `crypton-id` now signs with Ed25519 and holds the only private
  key; every verifier, Agent Lab included, holds only a public key and
  has no code path capable of minting a token the real public key would
  accept. `labs/agent-lab/LEARNING.md`'s v0.1/v0.4/v0.5 addenda still
  describe the old HS256 gap in their own prose (left as history, per
  that document's own framing) — its v0.5.1 addendum records the
  resolution in the same place the gap was originally named.

### Agent Lab v0.4: Delegation is the reference implementation of invariants 2-4

Not a new invariant — invariants 2 ("LLMs never make final authorization
decisions"), 3 ("authorization happens deterministically immediately
before execution"), and 4 ("every consequential action must be
attributable...") already required this; v0.4 is the first concrete
implementation that actually satisfies all three at once, and is worth
recording because it replaces one that had drifted from them without
anyone updating this file.

Agent Lab v0.1-v0.3's tool authority (`authz.REGISTRY`) was a static
Python dict keyed by agent **name**, not canonical `agent_id` — a real
bug, not a style choice: two of the same human's own agent registrations
sharing a name (e.g. the same "support-agent" at two different versions,
allowed under the `(owner_user_id, name, version)` uniqueness rule) would
silently inherit each other's authority, because the permission lookup
couldn't tell them apart. That is a live violation of invariant 4 in
spirit — the attributable unit for authority should be the specific
`agent_id`, not a label two different agents can share.

v0.4 (`labs/agent-lab/src/agentlab/delegation.py`,
`labs/agent-lab/src/agentlab/authorization.py`) replaces it with
**Delegation**: a persisted, explicit, finite-lived, revocable grant of
authority from a human principal to one canonical `agent_id` — never a
name — checked fresh against the live database row on every single tool
call (`authorization.authorize_tool_call`), never cached or decided once
per run. Every attempt, allowed or denied, is persisted as an
`AuthorizationDecision` with a stable reason code, so a denial is
evidence, not silence. `authz.py` is deleted; there is exactly one
authorization system now, not two competing ones. Full rationale:
`labs/agent-lab/LEARNING.md`'s v0.4 addendum.

Known, deliberately-deferred gap this does not close: invariant 5's
tension (§"Known tension" above, `JWT_SECRET` being symmetric) is
unchanged by v0.4 — Agent Lab can still mint tokens it only needs to
verify. That remains tracked, not solved, pending asymmetric signing /
OIDC-JWKS.

### Agent Lab v0.5: Approval is a second, independent check — not part of Delegation

A genuinely new decision, not a reference implementation of an existing
invariant: **authorization (does this agent have standing authority) and
approval (did a human confirm this exact action) are two separate,
independently-enforced checks, run in a fixed order, neither able to
substitute for or override the other.** It would have been simpler to
fold "requires approval" into `Delegation` itself (e.g. an
`approved_actions` field), but that would conflate a *standing* grant
with a *per-action* confirmation and make it impossible to express "valid
delegation, but this one call still needs a human to look at it right
now" — which is exactly the property v0.5 was asked to add.

`labs/agent-lab/src/agentlab/approval.py` implements **Approval**: a
persisted, human-issued confirmation of one exact tool call, bound via
`compute_argument_hash(tool_name, resource, validated_arguments)` — not
the tool name alone — and consumable exactly once
(`verify_and_consume_approval`'s atomic `approved -> consumed` update).
`agent.py`'s execution order runs `authorization.authorize_tool_call`
first; only a tool flagged `models.requires_human_approval` then also
requires `approval.verify_and_consume_approval` to return ALLOW before
executing. A denial from either check is final — this is what makes "a
valid approval never overrides a revoked delegation" and "a valid
delegation never bypasses a required approval" true by construction
rather than by convention. Full rationale, including the WebAuthn
boundary and what's real vs. simulated in this Lab today:
`labs/agent-lab/LEARNING.md`'s v0.5 addendum.

Known, deliberately-deferred gaps at the time: the `JWT_SECRET` tension
was unchanged (same as v0.4 — since closed, see below);
`webauthn.FakeWebAuthnVerifier` is a Lab-only simulation with no live
crypton-id integration yet (still true — see below);
`Approval.instance_id`/`task_id` are optional bindings, not mandatory
ones (mirroring `Delegation.task_id`'s existing precedent).

---

## v0.5.1: Ground Truth + Security Baseline (2026-08, certified 2026-09-02)

**v0.5.1 CERTIFIED — READY FOR v0.6.** A reconciliation-and-hardening
milestone, not a new capability, run before starting v0.6 (multi-hop
delegation-chain proof). Full investigation, baseline, and verification
detail: `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` (see its §10 for the
certification closure — `services/crypton-id` compiles clean and its full
test suite, 38/38 including every JWT test, passes; the earlier "unverified
by a local compiler run" risk is closed). Two decisions made here;
everything else in this pass was verification of what already existed.

### JWT: asymmetric signing (Ed25519) replaces the shared HS256 secret

**Decision:** `crypton-id` signs JWTs with Ed25519 (EdDSA) instead of
HS256. It holds the private key (`JWT_SIGNING_KEY_PEM` /
`JWT_SIGNING_KEY_PEM_FILE`); every verifier — `AuthUser`/`AdminUser`
inside `crypton-id` itself, and `labs/agent-lab` — is handed only the
public key (`JWT_PUBLIC_KEY_PEM` / `JWT_PUBLIC_KEY_PEM_FILE`). This
closes invariant 5's tracked tension (see above): a verifier can no
longer mint a token, structurally, not just by policy.

**Why now:** the tension was named at v0.1 and re-confirmed unresolved at
v0.4 and v0.5 without anyone closing it. It's exactly the kind of gap
v0.5.1 exists to find and fix before v0.6 builds a multi-hop delegation
chain on top of this identity layer — a chain proof is only as trustworthy
as the tokens it's chained from.

**What changed:**
- `services/crypton-id/src/jwt.rs`: `issue_jwt` takes an `&EncodingKey`
  (Ed25519) instead of a secret string; a new `verify_token(state, token)`
  is the single verification choke point, used by `AuthUser`'s extractor
  and the three call sites that used to each inline their own
  `jsonwebtoken::decode` (`auth_webauthn.rs`'s optional-auth helper and
  `logout`, `oauth.rs`'s `oauth_complete`) — one place to get the
  algorithm and key right, not four.
- `services/crypton-id/src/config.rs` / `state.rs`: `Config`/`AppState`
  carry `jwt_signing_key_pem`/`jwt_encoding_key` and
  `jwt_public_key_pem`/`jwt_decoding_key` instead of one `jwt_secret`
  string. A `{NAME}_FILE` env-var convention (`read_pem_env`) lets
  multi-line PEM content come from a file instead of cramming it into one
  `KEY=VALUE` line — used by `start.sh`/`run.bat`/`start-all.ps1`'s
  generated local dev keys and CI's generated per-run key.
- `labs/agent-lab/src/agentlab/identity.py`: `verify_human_identity` now
  verifies EdDSA against `JWT_PUBLIC_KEY_PEM` only.
  `issue_dev_token`'s `private_key` parameter is now required with no
  env-var fallback; the CLI's `--dev-user` path mints a fresh, ephemeral,
  throwaway keypair per invocation (`identity.generate_dev_keypair`) —
  never crypton-id's real key, by construction.
- Tests added on both sides proving the actual security property (not
  just "the code compiles with a new type"): a verifier holding only a
  public key cannot construct a signing key from it
  (`jwt.rs::public_key_pem_cannot_be_used_as_a_signing_key`,
  `test_identity.py::test_public_key_cannot_be_used_to_mint_a_token`), and
  a token forged with any other keypair is rejected
  (`jwt.rs::token_signed_by_a_different_private_key_is_rejected`,
  `test_identity.py::test_token_signed_by_a_different_private_key_is_rejected`).

**Deliberately not done:** a JWKS endpoint / dynamic key discovery, key
rotation tooling, or multi-key support. There is exactly one signing key
today, carried with a `kid` header (`"v1"`) as a seam for later rotation,
not a working rotation mechanism — `jsonwebtoken`'s `Validation` still
pins to a single `DecodingKey`. A real rotation story (publish new key,
accept both old and new for an overlap window, retire old) is legitimate
future work, not needed for v0.5.1's goal of closing the mint-vs-verify
gap. Also not done: `iss`/`aud` claim validation — neither was present
before this pass either (`Validation::default()`'s claim set was already
just expiry), so adding them now would be a scope-expanding claims-shape
change, not a signing-algorithm migration; flagged as a candidate for a
future pass, not silently dropped.

### WebAuthn: investigated, found not duplicated — no unification needed

**Decision:** leave `services/crypton-id/src/routes/actions.rs` and
`labs/agent-lab/src/agentlab/webauthn.py` exactly as they are.

**Why:** the Technical SOT's graphify-inferred `semantically_similar_to`
edge between crypton-sdk's `Actions` module and this lab's `Approval` was
a pattern-similarity inference, not a code fact. Read from source: there
is exactly one real, production WebAuthn verification engine in this
repo (`crypton-id`'s single `AppState.webauthn` instance, shared by
login/registration and step-up); Agent Lab's `webauthn.py` is an
interface plus one **fake, Lab-only** implementation that has never
called a real ceremony — already documented as such in the v0.5 addendum.
Two implementations of two different things (crypton-id's real
general-purpose step-up; Agent Lab's fake, exact-argument-hash-bound
simulation) is not the same as one thing implemented twice. Full
investigation: `labs/agent-lab/LEARNING.md`'s v0.5.1 addendum.

**Deliberately not done:** building `CryptonIdWebAuthnVerifier` (a real
integration wiring Agent Lab to crypton-id's `/actions/*` over HTTP).
Not because it's out of scope for the *idea* — `webauthn.py`'s interface
exists precisely so this is a pluggable swap later — but because it
needs a way to get a browser-completed WebAuthn assertion into a
synchronous, headless CLI tool, which is new integration surface, not a
deduplication. Tracked as a bounded follow-up in
`docs/CRYPTON_TECHNICAL_SOT_v1.1.md`, not attempted here.

---

## v0.6: Provable Multi-Hop Delegation with Scope Attenuation (2026-09)

The technical wedge: `Human -> Agent A -> Agent B -> Agent C`, every hop
cryptographically attributable, authority narrowing monotonically at each
step, proven by a single authoritative verifier (`chain.py`) rather than
asserted by convention. Full investigation, algorithm detail, and
certified test results: `docs/CRYPTON_TECHNICAL_SOT_v1.1.md`. RAT-6 (the
milestone's acceptance test) is proven — see that document's v0.6 section
for the exact scenario and results. Explicitly *not* built this pass:
gateway enforcement, real Agent Lab WebAuthn wiring, MCP, production key
custody — see "What v0.6 deliberately did not build" below.

### Signing principal: `Agent`, not `AgentInstance`

**Decision:** the cryptographic principal that signs a delegation is the
durable, registered `Agent` (`agents.py`'s `agent_id`) — the same entity
`Delegation.agent_id` has always named as the *subject* of a grant, since
v0.4. `AgentInstance` (one row per `AgentRunner.run()` call, minted fresh
every run) is not a signing principal at all; it remains exactly what it
already was — a runtime execution correlator, recorded in traces, never
cryptographically material.

**Why, from the actual code, not preference:** `Delegation` has never been
`instance_id`-scoped — it is granted once and reused across many runs of
the same agent (`grant_delegation`'s existing v0.4 semantics, unchanged).
If `AgentInstance` were the signing principal, every hop of a chain would
need re-signing on every single `AgentRunner.run()` call, since a fresh
`AgentInstance` is minted every time — turning a *standing* grant into
something that has to be re-established per execution, which contradicts
the property Delegation has had since v0.4 ("checked fresh against the
live row on every call," not "re-issued every call"). Making `Agent` the
signing principal keeps `agent_id` the single, consistent answer to "which
entity does this authority belong to" across ownership (`agents.Agent.owner_user_id`),
delegation subject (`Delegation.agent_id`, unchanged since v0.4), and now
signing identity (`agent_signing_keys.agent_id`) — one identity concept,
not two competing ones for the same durable entity.

**Consequence:** not every agent needs a signing key. A leaf agent that
only ever *receives* delegated authority (Agent C in the RAT-6 scenario)
never issues a child delegation and never needs one — `signing_keys.py`'s
`create_signing_key` is a separate, explicit call from `agents.register_agent`,
the same way granting a `Delegation` has always been separate from
registering the `Agent` it's granted to.

### Key model

Ed25519 (via the `cryptography` library — no hand-rolled crypto, same
discipline the v0.5.1 JWT migration already established). One new table,
`agent_signing_keys` (`key_id`, `agent_id`, `public_key_pem`, `algorithm`,
`created_at`, `revoked_at`, `status`) — **public key only**; the private
half is generated, returned once to the caller (the CLI's
`--create-signing-key` output, or a test fixture), and never persisted
anywhere by this codebase. An agent may hold more than one key over its
lifetime (no uniqueness constraint forces "at most one") — there is no
rotation-with-overlap story, just create/resolve/revoke, mirroring
`DelegationStatus`'s one-way shape (`lifecycle.SigningKeyStatus`).

**Revocation policy — retroactive, not grandfathered:** per this milestone's
own guidance, revoking a signing key untrusts *every* delegation it ever
signed, immediately, not only delegations signed afterward. `chain.py`
never caches a key's status — it re-resolves the signing key and checks
`revoked_at IS NULL` on every single verification, the same "never cache,
always re-check the live row" discipline `Delegation`'s own
expiry/revocation checks have used since v0.4. Chosen over grandfathering
because this milestone's threat model treats a compromised or
mistakenly-issued key as something that should stop being trusted the
moment that's known, not continue authorizing past decisions made under
it — the safer of the two options the milestone's own guidance offered,
and the one consistent with `Delegation.revoked_at` already meaning "stop
trusting this, retroactively as of now," not "stop trusting new grants
from here."

### Delegation schema: evolved, not replaced

No parallel "DelegationProof" table. `delegations` gained seven additive
columns (`format_version`, `issuer_type`, `issuer_id`, `parent_delegation_id`,
`parent_hash`, `signing_key_id`, `signature`) via idempotent `ALTER TABLE`
(`db.py::_ensure_column`) rather than a rewritten `CREATE TABLE` — a
database created before v0.6 gets these columns added in place, never
dropped/recreated, and every pre-v0.6 row reads back as a valid *root*
delegation (`parent_delegation_id IS NULL`, `issuer_type` defaults to
`'human'`, `issuer_id` defaults to the row's own `principal_user_id`) —
see `delegation.py::_delegation_from_row`. `grant_delegation` (v0.4's
original function) is unchanged in signature and behavior for every
existing caller; it now also stamps the new columns with root-shaped
defaults. `Delegation.is_root` (`parent_delegation_id is None`) is the one
new property that distinguishes the two shapes — everywhere else in the
codebase, a root and a chained delegation are the same type, walked by
the same verifier.

### Canonical serialization and what a signature covers

`delegation.canonical_delegation_bytes` — deterministic JSON (sorted
keys, no whitespace, sets sorted before encoding, `format_version` itself
part of the payload) over exactly the fields that define what a
delegation *grants*: `delegation_id`, `issuer_type`/`issuer_id`,
`principal_user_id`, the subject `agent_id`, `task_id`, `allowed_tools`,
`resource_constraints`, `issued_at`/`expires_at`, `parent_delegation_id`,
`parent_hash`, `signing_key_id`. Deliberately **excludes** `status` and
`revoked_at` — revocation is a live fact, re-checked on every
verification (see key-revocation policy above and `Delegation`'s existing
v0.4 revocation model), never something a signature should freeze at
issuance time. `delegation_digest` (sha256 of the canonical bytes) is what
a *child*'s `parent_hash` binds to; `sign_delegation`/`verify_delegation_signature`
sign/verify those same bytes with Ed25519. Proven in
`tests/test_delegation_signing.py`: identical logical delegations produce
identical bytes regardless of Python dict/set construction order, and
every one of the thirteen covered fields, mutated individually, changes
both the digest and invalidates a prior signature.

### Attenuation semantics

One authoritative function, `chain.verify_attenuation(parent, child)`,
never duplicated at any call site (issuance-time validation in
`delegation.grant_chained_delegation` calls the exact same function
verification-time validation in `chain.py` does — see "why runtime
verification never assumes issuance-time validation happened" below).

- **Tools:** `child.allowed_tools <= parent.allowed_tools` (frozenset
  subset). Equal is valid; dropping a tool is valid; adding one the
  parent never granted is not.
- **Resources:** for every `resource_type` the child scopes at all, the
  parent must scope that exact type too, and the child's id set must be a
  subset of the parent's for it. A `resource_type` the child names that
  the parent never constrained at all denies — mirrors
  `authorization.py`'s existing, pre-v0.6 rule that an unscoped type means
  "nothing of that type is authorized," never "unconstrained." No
  prefix/wildcard matching — Agent Lab's resource-constraint model has
  always been exact id sets (`{"customer_ids": {"cust_1"}}`), and exact-set
  subset is the only unambiguous, safe narrowing semantic for that model;
  a naive string-prefix scheme (where `"customer/1"` would appear to
  narrow `"customer/123"`) was explicitly not built.
- **Expiry:** `child.expires_at <= parent.expires_at`.
- Ambiguous cases deny, never invent authority — consistent with
  invariant 8.

### Chain verification algorithm

`chain.verify_delegation_chain` is the single authoritative entry point
(§18-19 of the milestone spec). It never trusts a caller-supplied chain —
every delegation in a walk is loaded fresh from the database by ID, the
same `delegation.get_delegation_unchecked` the pre-existing leaf check
already used. Order:

1. **Leaf check** — `authorization.authorize_tool_call` (v0.4's original
   function, called exactly as it always has been: agent identity,
   instance/principal match, delegation existence/ownership/agent-match/
   task-match/expiry/revocation, tool allowlist, resource scope). This is
   what makes a root-only (v0.5-shaped) delegation's behavior *identical*
   to before v0.6 — zero ancestors to walk means this function does
   nothing else, for every pre-v0.6 caller (`tests/test_delegation_chain.py::test_root_only_delegation_verifies_identically_to_v05`).
2. **Ancestor walk**, only if the leaf has a parent — for the leaf, then
   each ancestor in turn, toward the root: depth/cycle guard ->
   `format_version` supported -> shared root-principal check (every hop
   must trace to the same accountable human) -> **signature** (resolve the
   delegation's `signing_key_id`, confirm the key exists and is not
   revoked, verify the signature over the delegation's own canonical
   bytes) -> resolve parent, detect a repeat visit -> **issuer/subject
   continuity** (`current.issuer_id == parent.agent_id`, never trusting
   that issuance-time validation ran) -> **parent-hash match** (`current.parent_hash == delegation_digest(parent)`,
   right now, not cached) -> **attenuation** (`verify_attenuation(parent, current)`).
3. **Root liveness** — once a delegation with no parent is reached, it
   must itself not be revoked or expired.

Every DENY is persisted through the exact same evidence ledger v0.4
established (`authorization.record_decision`/`authorization_decisions`) —
chain-specific reason codes (`CHAIN_SIGNATURE_INVALID`,
`CHAIN_PARENT_HASH_MISMATCH`, `CHAIN_ISSUER_CONTINUITY_VIOLATION`,
`CHAIN_ATTENUATION_VIOLATION`, `CHAIN_ANCESTOR_REVOKED`,
`CHAIN_ANCESTOR_EXPIRED`, `CHAIN_CYCLE_DETECTED`,
`CHAIN_DEPTH_EXCEEDED`, `CHAIN_VERIFICATION_ERROR`, and others — see
`chain.py`) in the same vocabulary style as v0.4's, never a second
decision system.

**A notable, verified emergent property:** `expires_at` is defended by
four independent layers at once — it's part of the signed envelope
(tampering it breaks the delegation's own signature), part of the hashed
parent-link (tampering an *ancestor's* copy breaks a descendant's
`parent_hash`), part of attenuation ordering (shortening an ancestor's
expiry out of step with its children's already-narrower values violates
the subset rule), and finally checked live against the clock. In an
honestly-issued, non-tampered chain, the first three make the fourth
effectively unreachable — expiry attacks are caught upstream, not by
accident but because attenuation's monotonic-narrowing guarantee means
the *leaf* always expires first, never an ancestor while a descendant
remains locally valid. `tests/test_delegation_chain.py`'s ancestor-expiry
tests prove the live check exists and works correctly as the last of four
layers by deliberately bypassing the first three — documented there as
defense-in-depth, not the primary mechanism.

**Cycle and depth protection:** a `visited` set of delegation IDs is
tracked across the walk; a repeat denies with `CHAIN_CYCLE_DETECTED`. A
hard bound, `chain.MAX_CHAIN_DEPTH = 32`, denies with
`CHAIN_DEPTH_EXCEEDED` rather than recursing unboundedly — chosen as a
generous prototype ceiling well above the milestone's own >=3-hop
requirement, not a business rule. Like the expiry property above, a
*true* cycle turns out to be cryptographically unconstructable without
first breaking a signature or parent-hash check (`parent_delegation_id`
is itself a signed field — re-pointing it without re-signing is caught as
tampering, same as any other field), so the guard is verified the same
way: with those checks deliberately bypassed, proving the traversal still
terminates cleanly rather than looping forever.

**Why runtime verification never assumes issuance-time validation
happened:** `delegation.grant_chained_delegation` rejects an invalid
child immediately (broadened tools/resources/expiry, a revoked or
expired parent, an issuer that isn't the parent's actual subject) — but
`chain.verify_delegation_chain` re-derives every one of those same facts
independently from persisted state on every check, never trusting that a
row currently in the table passed through that function honestly. A row
inserted by a compromised process, a bug, or direct SQL is verified with
exactly the same rigor as one issued through the CLI —
`tests/test_delegation_chain.py::test_rat6_unauthorized_issuer_denied_defensively_at_verification`
proves this directly, simulating a forged row and confirming the verifier
still denies it.

### Human root — no second identity mechanism invented

A `HumanIdentity` does not get an Ed25519 keypair in this milestone. The
root delegation (Human -> Agent A) is attributed the same way every other
human-initiated write in this codebase always has been: by requiring an
authenticated, JWT-verified `HumanIdentity` to make the call
(`grant_delegation`'s existing, unchanged ownership discipline) — not by a
new per-human signature. This is a deliberate scope boundary, not an
oversight: crypton-id's real WebAuthn flow already proves human presence
for that JWT; inventing a second, delegation-specific human signing
identity on top of it would duplicate an authentication mechanism that
already exists, for no property this milestone's RAT-6 scenario needs. A
root delegation still gets a `delegation_digest` (so a child can hash-bind
to it) — it just isn't Ed25519-*signed*, since there is no meaningful
private key to check that signature against. Recorded as a known,
deliberate limitation, not hidden: see "What v0.6 deliberately did not
build" below and `docs/CRYPTON_TECHNICAL_SOT_v1.1.md`.

### Authorization integration — one path, not two

`agent.py`'s boundary 2 now calls `chain.verify_delegation_chain` instead
of calling `authorization.authorize_tool_call` directly — but
`verify_delegation_chain`'s *first* action is calling
`authorization.authorize_tool_call` itself, unchanged, by the same
module-attribute reference agent.py always used
(`from . import authorization` / `authorization.authorize_tool_call(...)`).
There is exactly one authorization system, not a v0.4 one and a v0.6 one
running in parallel — v0.6 strictly extends what "authorization" means
(full chain verification) rather than adding a second gate beside it.
Approval (v0.5) is untouched and remains strictly after this step, for
tools that require it, exactly as before — a fully valid chain can still
require approval; a valid approval can never rescue an invalid chain,
because approval is only ever reached after chain verification has
already returned ALLOW (`tests/test_delegation_chain.py::test_valid_approval_cannot_rescue_an_invalid_chain`).

**One deliberate behavior change, not a regression:** an *unexpected
internal exception* during authorization now becomes a persisted,
evidenced DENY (`CHAIN_VERIFICATION_ERROR`) instead of an uncaught crash
propagating out of `AgentRunner.run()`. The v0.5.1 test that asserted the
old crash-shaped behavior
(`test_enforcement_boundary.py::test_authorization_error_prevents_execution`)
was updated to assert the new one — the actual security property (zero
tool execution) is unchanged either way; only the *shape* of the failure
changed, and the new shape is what this milestone's own fail-closed
invariant requires ("verifier internal error -> DENY/no execution", not
"-> crash").

### Single-hop compatibility — proven, not assumed

There is one verifier, not two authorization systems kept in parallel. A
plain v0.5-shaped `Delegation` (no `parent_delegation_id`) walks through
`chain.verify_delegation_chain` and returns after step 1 (the leaf check)
with `delegation_depth == 1` — identical behavior, identical reason codes,
identical evidence, to what `authorization.authorize_tool_call` alone
already produced before this milestone
(`tests/test_delegation_chain.py::test_root_only_delegation_verifies_identically_to_v05`).
Every pre-v0.6 test in the suite (185 of them) continues to pass unmodified
against this new code path.

### What v0.6 deliberately did not build

Per this milestone's own explicit scope boundary — not oversights, not
silently dropped, tracked here and in `docs/CRYPTON_TECHNICAL_SOT_v1.1.md`
for whoever picks up the next piece:

- **Gateway enforcement.** `crypton-gateway` remains a transparent proxy;
  the marked extension point in `main.rs` is still unimplemented. The
  RAT-6 proof lives entirely inside Agent Lab, which is sufficient to
  prove the delegation-chain *primitive* — deployment topology (where
  enforcement actually sits in a request path) is a separate decision.
- **Real Agent Lab WebAuthn wiring.** `CryptonIdWebAuthnVerifier` still
  doesn't exist (see the v0.5.1 investigation above, unchanged) —
  Approval remains simulation-only end to end. Does not weaken the
  delegation-chain proof, which is orthogonal to whether the *approval*
  gate's ceremony is real.
- **Production agent-key custody.** No KMS/HSM, no hardware attestation,
  no certificate infrastructure. Private keys are generated in-process
  and handed to the caller once — real, working Ed25519, but prototype-
  grade custody, exactly as scoped.
- **MCP, DIDs, verifiable credentials, ZK proofs, a policy DSL, multi-
  tenant infrastructure, billing, rate limiting, cloud deployment
  architecture.** None relevant to proving the delegation-chain primitive
  itself; explicitly out of scope per this milestone's own instructions.
- **Key rotation with overlap, JWKS-style key discovery.** Create/resolve/revoke
  only — see the key model above.

---

## Decisions from the 2026-08 cleanup

Judgment calls made while consolidating the repo, recorded so they don't
need to be re-litigated or accidentally reversed. See git history around
tag `pre-refactor-audit-2026-08-08` for the prior state.

## crypton-main stayed a separate app

The target layout put one frontend (`crypton-console`) under `services/`.
`crypton-main` (the public marketing site — no login, no backend calls) is
categorically different from `crypton-demo`/`crypton-admin` (which were
~70% duplicate copies of the same authenticated product). Folding a static
marketing site into the authenticated console app would have been a much
larger, riskier change for no real benefit, and the cleanup brief explicitly
said not to spend time on marketing UI. `AttackSim.js` (the one fake/scripted
element in crypton-main) was removed; the rest of the site was left alone.

## crypton-demo + crypton-admin → crypton-console, one login

Both apps had real, working device/audit/recovery pages plus their own
copies of fake pages (Sessions, RiskIntel, PolicyEngine, OrgSettings, RBAC)
and — most importantly — **two separate JWTs** (`crypton_token` and
`crypton_admin_token`) with two separate login flows. `crypton-console` has
one login. Role (admin or not) comes from `GET /me`, fetched after login,
and only controls what the sidebar *shows* — the backend's `AdminUser`
extractor is the actual enforcement point regardless of what the UI thinks
the role is.

## Admin bootstrap instead of a manual SQL step

The old code had no way to ever create the first admin — `PATCH
/users/:id/role` was a stub that updated nothing, and nothing else wrote
`users.role`. Rather than leave that gap, `POST /admin/bootstrap`
(`crypton-id/src/routes/admin.rs`) lets any authenticated user become
`super_admin`, but **only** while zero admins exist. It's a one-time
escalation path by construction, not a standing one — call it twice and the
second call always 403s.

## Actions/step-up: kept the mechanism, deleted the fake results

`/actions/execute` used to run real WebAuthn re-authentication and then
return one of three hardcoded "results" (`add_admin`, `rotate_api_key`,
`delete_resource`) that were never persisted. The real WebAuthn step-up
proof is a genuinely useful primitive — proving a human is present for a
specific intent, with an audit trail — so it was kept and simplified to
what it actually is: `{status, verified, intent}`. No business logic, no
fake results. This shape is also what agent-approval flows will need later.

## OAuth: kept, but gated behind `ENABLE_OAUTH=false`

The OAuth authorization-code implementation (`routes/oauth.rs`) is real
code with a real state machine, but it has no PKCE, a single hardcoded
client, and (until this cleanup) an insecure default client secret. Nothing
in this repo consumes it. Rather than delete real, mostly-working code or
present it as production-ready, it's unmounted by default — set
`ENABLE_OAUTH=true` to opt in — and documented as experimental everywhere
it's mentioned.

## Gateway: cut to routing + observability, not revived

~800 lines of gateway middleware (auth, policy, rate-limit, nonce, audit
ingest, metrics) existed on disk but were never wired into the router (no
`mod` declarations) and depended on crates that weren't even in
`Cargo.toml` — it could not have compiled if it had been wired in. It was
deleted rather than fixed-and-revived, because reviving it unreviewed would
mean shipping ~800 lines of security-relevant code nobody has actually
audited. The gateway is now proxy + CORS + health + request-ID/structured
logging, with a marked extension point in `main.rs` for where
agent-authorization middleware belongs later.

## Recovery: fixed the contract, not the design

The backend's recovery design (public start → trusted-device approve →
public complete with a bearer token, no email/SMS) was correct. The bug was
entirely client-side: `claimRecovery` sent `{request_id, username}` when the
backend required `{username, recovery_token}`, and the SDK's `register()`
had no parameter for the `enrollment_token` needed to re-enroll on an
existing account. Both are fixed in `libs/crypton-sdk/src/modules/recovery.ts`
and `auth.ts`; the backend (`crypton-id/src/routes/recovery.rs`) was not
changed.

## Removed outright (see git history for old code)

`services/crypton-id/src/routes/stubs.rs`'s fake sessions/risk/passkeys/
policies/org endpoints, `contact.rs` (zero callers), `services/crypton-id/
src/db/` (dead module, 0-byte files, referenced a nonexistent submodule),
the duplicate `DELETE /devices/:id` route, `AttackSim.js` (all three
copies), `graphify-out/` (a stale committed analysis artifact), and the
root `package.json`'s leftover CRA scripts/dependencies (`@fal-ai/client`,
`remotion` — there is no app at the repo root).
