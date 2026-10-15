# Assessment API (Vercel Functions + Supabase Postgres)

Serverless intake for the Agent Floor "Request an assessment" form. It replaces the single-instance
`services/crypton-inquiries` service (encrypted local SQLite) for production; that service stays in the repo
for local development and self-hosting only.

Delivery means **a committed row in Postgres**. There is no email or other notification: check the inbox with
the operator tool (`npm run inbox`).

## Architecture decision

| Question | Decision |
|---|---|
| Runtime | Two Vercel Functions in this project (`api/assessment/index.js`, `api/assessment/challenge.js`), plain Node `(req, res)` handlers. No framework, no separate server. |
| Extra always-on hosting | **Not needed.** Nothing here needs a long-lived process, a local disk or in-process state. |
| Database access | Direct Postgres connection as a dedicated least-privilege role (`assessment_writer`) through Supabase's pooler. **Not** supabase-js, **not** the service-role key, **not** a public RPC. |
| What the web role can do | `EXECUTE` on exactly one function, `assessment.submit_inquiry(...)`. No table privileges at all. |
| Where the rules live | In that one `SECURITY DEFINER` SQL function, so idempotency, throttling and the insert are atomic across any number of concurrent function instances. |
| Encryption | Application-layer AES-256-GCM is **kept**. Supabase stores ciphertext only; the master key lives only in Vercel env and the operator's machine. |
| Lead access | Only the operator role (`assessment_operator`), over a direct connection from the operator's machine. Nothing is exposed through Supabase's REST API. |

Why not the Supabase REST API / service-role key: the service-role key bypasses RLS and is a standing
all-powerful credential in the web tier; a compromised function would read or alter every table in the
project. A login role that can only call one function limits that blast radius to "insert a lead".

## Request flow

```
browser ── GET  /api/assessment/challenge ──► signed token (HMAC, no storage)
browser ── POST /api/assessment ────────────► Vercel Function
              method → config (fail closed) → origin → content-type → 8 KiB body cap
              → JSON → validation → token (age 1.5 s .. 1 h) → encrypt + hash
              → assessment.submit_inquiry()  (one SQL call, one transaction)
              → 201 / 200 { status: "received", receipt }  only if Postgres committed
```

## Public contract (unchanged from the legacy service)

- `GET  /api/assessment/challenge` → `200 { token }`
- `POST /api/assessment` → `201 { status: "received", receipt }` (stored) or `200` (identical retry, same receipt)
- Errors: `400` verification/json, `403` origin, `405` method, `408` slow body, `409` changed retry, `410` expired token,
  `413` too large, `415` content type, `422` validation (with per-field `errors`), `425` too fast, `429` rate limit
  (with `Retry-After`), `503` anything we cannot confirm (database down, misconfiguration).
- The frontend (`src/agent-floor/leadClient.js`) accepts only JSON, `200/201`, `status === "received"` and a UUID
  receipt, so any other answer renders as an honest "couldn't confirm receipt" and keeps the visitor's fields.

## Audit: what the legacy service relied on, and what replaces it

| Property | Legacy mechanism | Why it fails on serverless | Replacement |
|---|---|---|---|
| Durability | SQLite file on local disk, WAL, `synchronous=FULL` | Function filesystem is ephemeral and per-instance | Supabase Postgres; the success response is sent only after the commit returns |
| Encryption key | `.private/inquiry.key` file (mode 0600), generated on first run | No persistent disk; a regenerated key would orphan every lead | `ASSESSMENT_MASTER_KEY` env var (Vercel, Sensitive), validated as 64 hex on every request; fails closed when absent |
| Idempotency | Receipt row looked up in SQLite inside `BEGIN IMMEDIATE` (single writer) | No single writer across instances | Unique `idempotency_key` (the token's UUID) + request fingerprint; unique-constraint insert with the lost race re-read; identical retry → 200 + original receipt, altered retry → 409 |
| Per-email throttle (3/hour) | `count(*)` inside `BEGIN IMMEDIATE` | Serialisation came from the single process | `pg_advisory_xact_lock` per email hash, then count, inside the function. Proven by a forced-overlap test (see Tests) |
| Per-client throttle | `rate_limits` rows keyed by client IP, in SQLite | Same file; `trustedProxy` header logic assumed one proxy | `assessment.rate_limits` buckets (60/min per keyed-hash of client IP); client IP taken from Vercel's platform header **only when `VERCEL=1`** |
| Token signing key | Derived from the key file | n/a | Derived (HKDF) from `ASSESSMENT_MASTER_KEY`; tokens are stateless, so any instance verifies them |
| Pruning | `setInterval` hourly | No long-lived process | Opportunistic pruning inside the function (2% of calls, bounded to 500 rows); 90-day retention; optional `pg_cron` |
| Request limits / timeouts | `http.createServer` limits | Platform-managed | Streamed 8 KiB body cap, 8 s body timeout, `maxDuration: 10` in `vercel.json`, 8 s DB timeout |
| Confidentiality of the store | File permissions + location outside the build dir | n/a | RLS on, no grants to `anon`/`authenticated`/`service_role`/`PUBLIC`, ciphertext-only payload |

Nothing was removed without a replacement. Per-IP rate limiting is weaker than a CDN edge limit: see "Compromises".

## Database

`supabase/migrations/20261015000000_assessment_inquiries.sql` (version controlled; see `supabase/README.md`).

- Private schema `assessment` (not exposed by PostgREST). Tables `assessment_inquiries` and `rate_limits`, RLS enabled.
- `assessment_inquiries`: `id uuid` (random), `idempotency_key uuid unique`, `request_fingerprint`, `status`
  (`new`, `reviewing`, `contacted`, `qualified`, `closed`, `spam`), `payload bytea` (ciphertext), `key_id`, `email_hash`,
  `consent` (must be true), `privacy_version`, `source`, `created_at`, `updated_at` (trigger).
- Contact details and the optional system description are inside the encrypted `payload`; only keyed hashes
  and non-sensitive metadata are queryable.
- Roles (created `NOLOGIN`; **passwords are set out of band**, never in the migration):
  - `assessment_writer`: web tier. `EXECUTE` on `submit_inquiry` only. Own limits: `statement_timeout 5s`,
    `lock_timeout 2s`, `idle_in_transaction_session_timeout 10s`, `connection limit 30`.
  - `assessment_operator`: `SELECT`, and `UPDATE (status)` only, through RLS policies.
- Function owner is the migration role; `SECURITY DEFINER` with `set search_path = ''`.

## Environment variables

| Name | Where | Notes |
|---|---|---|
| `ASSESSMENT_MASTER_KEY` | Vercel (Sensitive), operator machine | 64 hex chars. Generate with `node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"`. Losing it makes stored leads unreadable. |
| `ASSESSMENT_DATABASE_URL` | Vercel (Sensitive) | `assessment_writer` connection string via the Supabase pooler (transaction mode). |
| `ASSESSMENT_ALLOWED_ORIGINS` | Vercel | Comma-separated exact origins, e.g. `https://cryptonid.tech,https://www.cryptonid.tech`. Preview deployments additionally allow their own `VERCEL_URL`/`VERCEL_BRANCH_URL`, never in production. |
| `ASSESSMENT_DATABASE_SSL` | Vercel, optional | `verify` (default), `require`, `disable` (only for local hosts outside production). |
| `ASSESSMENT_DATABASE_CA` | Vercel, optional | PEM CA for `verify` (use `\n` for newlines). |
| `ASSESSMENT_OPERATOR_DATABASE_URL` | operator machine only | `assessment_operator` direct connection, used by `npm run inbox`. |
| `ASSESSMENT_MASTER_KEY_PREVIOUS` | operator machine, optional | Old key, so the tool can read leads written before a rotation (`key_id` selects the key). |
| `ASSESSMENT_TEST_DATABASE_URL` | tests only | Superuser URL of a throwaway Postgres (>= 15). |

No variable is `REACT_APP_*`; none is read by anything under `src/`, so none can enter the browser bundle.
`REACT_APP_ASSESSMENT_ENDPOINT` (optional, public) only overrides the endpoint path.

## Abuse controls

| Control | Where | Notes |
|---|---|---|
| Origin check (+ `Sec-Fetch-Site`) | function | Exact-match allowlist |
| Body cap 8 KiB, slow-body timeout, JSON-only | function | Junk is rejected before any query |
| Field validation with the legacy limits and messages | function | |
| Signed challenge token, min age 1.5 s, max 1 h | function | Stateless HMAC; defeats instant scripted posts and replays beyond idempotency |
| Honeypot field | function | |
| Per-email 3/hour, per-client 60/min | database | Atomic across instances |
| Idempotency + altered-retry detection | database | |
| Safe errors, names-only logs | function | Logs carry an error *code* only, never a value, SQL or stack |
| Platform rate limit | **Vercel Firewall (not configured yet)** | Recommended: rate-limit rule on `POST /api/assessment` (for example 10 per minute per IP). Needs a dashboard step; see "Outstanding configuration". |
| Bot challenge (Turnstile / BotID) | **evaluated, not implemented** | Would add a third-party dependency and a client change; hold until real abuse appears. The signed token, honeypot and DB throttles are the interim controls. |

## Compromises (stated plainly)

- The per-client throttle lives in Postgres, so every abusive request still costs one pooled query. An edge rule
  (Vercel Firewall) is the right first line; the database throttle is the backstop.
- Client IP is trusted from Vercel's platform header only when running on Vercel (`VERCEL=1`); elsewhere the socket
  address is used.
- App-layer encryption means Supabase's dashboard cannot show lead contents. That is intentional (a database leak
  or an over-permissive dashboard user exposes only ciphertext) but means all reading goes through the operator tool.
- Single `ASSESSMENT_MASTER_KEY`. Rotation is supported by `key_id` + `ASSESSMENT_MASTER_KEY_PREVIOUS` for the
  operator tool; it is not automated.
- New dependency: `postgres` 3.4.9 (zero transitive dependencies, Unlicense). Used with `prepare: false` for the
  transaction pooler.

## Operating

```bash
npm run dev:api -w crypton-main     # local adapter on :8091 (reads api/.env.local), proxied by src/setupProxy.js
npm run test:api -w crypton-main    # core tests always; database tests when ASSESSMENT_TEST_DATABASE_URL is set
npm run inbox -w crypton-main list            # new leads (decrypted on your machine)
npm run inbox -w crypton-main list -- --all
npm run inbox -w crypton-main status <id> <new|reviewing|contacted|qualified|closed|spam>
```

## Tests

`api/__tests__/core.test.mjs` (no database) and `database.test.mjs` (real PostgreSQL, throwaway database per
run, applies the real migration, drops it afterwards): 62 tests, 0 skipped when `ASSESSMENT_TEST_DATABASE_URL` is set. Covered: valid submission persisted exactly once and
encrypted; duplicates and concurrent duplicates idempotent across independent OS-process instances; altered
retries; invalid input rejected without touching the database; database unavailable (refused connection, slow
query, role without privileges) → 503 and no success; anonymous roles cannot read or write; privilege matrix of
every role; retention; and two forced-overlap tests that hold one transaction open while a second request arrives,
which is the only deterministic way to prove the per-email lock and the idempotency-key race.

The migration was also mutation-tested: granting `anon` read, omitting RLS, and removing the per-email advisory
lock each make the suite fail.

## Outstanding configuration (requires owner approval)

See the deployment checklist in `supabase/README.md`. Nothing here has been applied to any Supabase project or
Vercel project.
