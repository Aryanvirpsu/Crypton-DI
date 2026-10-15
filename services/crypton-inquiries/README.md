# Crypton private assessment inbox

Dependency-free Node 24+ service for Agent Floor enquiries. This is a separate,
small public-intake boundary; the identity service and gateway remain unchanged.
No email provider, paid service, browser storage or public read endpoint is used.
Delivery means a committed record in this inbox, **not an email notification**.

## Local operation

From this directory:

```powershell
npm start
# In another terminal, privately review requests:
npm run inbox
npm test
```

The development server listens only on `127.0.0.1:8091`. Restart the marketing
frontend once after adding `src/setupProxy.js`; the start wrapper uses an explicit
localhost allowlist to avoid CRA 5's missing-LAN-address host-check bug; `/api/assessment` then proxies to
this process. The frontend now defaults to that same-origin endpoint.
`.private/inquiries.sqlite` and `.private/inquiry.key` are created locally and
ignored by Git. Never delete/regenerate the key for an existing inbox. `npm run
inbox` decrypts requests for the operator at the terminal; do not paste that
output into public logs. Check the inbox regularly: there are no notifications.
Use OS account/volume access controls; file modes alone do not set Windows ACLs.

## Contract and delivery

1. GET `/api/assessment/challenge` issues an HMAC-signed, hour-long form token.
2. POST `/api/assessment` with JSON `{ name, email, organization, context,
   consent: true, source: "agent-floor", website: "", token }`.
3. The server validates fields and consent, checks the origin/token/honeypot,
   and commits an AES-256-GCM encrypted record with SQLite FULL synchronization.
4. **Only after COMMIT** does it return `201 { status: "received", receipt }`.
   The same token + same payload retries with `200` and the original receipt;
   altered retries return `409`. The client rejects HTML fallbacks, `202`,
   missing receipts, redirects, network errors, and arbitrary `2xx` responses.

The browser never receives encryption keys. Visitor data is never logged by the
service. Email identifiers and IP throttle keys are keyed hashes. Both client
and server validate lengths, types and consent; the server rejects non-JSON,
malformed JSON, control characters and bodies over 8 KiB. Requests are limited
to 60 attempts/IP/minute and 3 accepted requests/email/hour; tokens cannot be
submitted during their first 1.5 seconds. Those controls deter basic automation,
not distributed bots. The database retains active requests for 90 days; expiry
runs hourly and on startup. Rate buckets expire too. Retention of independent
backups must be configured separately.

A lost response can follow a successful write. The UI therefore says it cannot
confirm receipt, retains the fields and token, and lets the visitor retry without
duplicating the record. No promise that “nothing was saved” is made on errors.

## Production gate — not deployed by this change

Set `NODE_ENV=production`, `INQUIRY_KEY` (64 random hex characters),
`INQUIRY_DB` (absolute path on a persistent private volume), and
`INQUIRY_ORIGIN` (the exact HTTPS marketing origin). Startup refuses missing
configuration, insecure origins and an encryption key that cannot decrypt
existing data. Store the key in the host's secret manager and back it up
separately. Keep the database out of the public build directory.

Use **one service instance** on durable storage. Back up the SQLite database
using its online backup API or while the service is stopped; copying just the
main file while WAL is active is unsafe. Verify restore with the same key.
Do not deploy this on ephemeral/serverless storage. For multiple replicas,
migrate storage and throttling to the existing Postgres/Redis infrastructure.

There are two routing options, both require deployment approval:

- Serve the frontend build with this process: `INQUIRY_BUILD_DIR` points to
  `../crypton-main/build`; set the approved host/port behind an HTTPS ingress.
- Keep the frontend on Vercel and configure a same-origin `/api/assessment`
  reverse proxy to the private-inbox service **before** the SPA fallback rewrite.
  Its current `vercel.json` is deliberately unchanged and does not provide that
  production route yet.

Do not trust arbitrary forwarded IP headers. `INQUIRY_TRUSTED_PROXY` is optional
and must equal the immediate proxy's socket address; the service then uses only
the rightmost address appended by that trusted proxy. Configure the ingress to
strip/append client-supplied forwarding headers. With no trusted proxy configured,
requests behind an ingress share its IP budget. Origin restrictions are CSRF
protection, not bot authentication. Add ingress abuse controls if traffic demands.

Before public launch: approve hosting/routing, set secrets and persistent storage,
configure backups and retention, assign an operator to check the inbox, verify a
real public submission and private readback. A local pass is not evidence of
production delivery. No production systems are touched by this implementation.
