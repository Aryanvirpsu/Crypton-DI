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

### Known tension against these invariants (tracked, not fixed here)

- **Invariant 5 vs. `labs/agent-lab`'s JWT handling.** `crypton-id`
  signs JWTs with HS256 — a *symmetric* algorithm, so the same
  `JWT_SECRET` that lets a service verify a token also lets it mint one.
  `labs/agent-lab/src/agentlab/identity.py`'s `verify_human_identity`
  only needs to verify, but because it holds the shared secret to do so,
  it is also capable of signing arbitrary tokens — and its own
  `issue_dev_token` helper does exactly that, deliberately, for local
  dev/test convenience (see `labs/agent-lab/LEARNING.md` §10). This is a
  real, currently-existing gap against invariant 5, not hypothetical: as
  long as JWT signing is symmetric, any verifier is a de facto signer.
  Fully closing this needs asymmetric signing (RS256/EdDSA — `crypton-id`
  holds the private key, verifiers get only the public key), which is a
  `crypton-id`/`crypton-sdk` change out of scope for this pass. Until
  then, the mitigation is operational, not structural: per-environment
  secrets (already the convention — see
  `services/crypton-id/.env.example` vs. `.env.staging.example` vs.
  `.env.production.example`) mean a lab pointed at a dev secret cannot
  mint tokens valid against production, but nothing in code currently
  *enforces* that a lab can't be misconfigured to point at a production
  secret. `issue_dev_token` is also unused outside local dev/test paths
  in the current code — flagging it here so it doesn't quietly grow a
  production caller.

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

Known, deliberately-deferred gaps: the `JWT_SECRET` tension is unchanged
(same as v0.4); `webauthn.FakeWebAuthnVerifier` is a Lab-only simulation
with no live crypton-id integration yet; `Approval.instance_id`/`task_id`
are optional bindings, not mandatory ones (mirroring `Delegation.task_id`'s
existing precedent).

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
