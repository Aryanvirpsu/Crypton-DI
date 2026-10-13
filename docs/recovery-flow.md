# Lost-device recovery

There is no email or SMS delivery anywhere in this system. The recovery
secret (`recovery_token`) is returned directly in the HTTP response and the
UI's job is to make that obvious, not to pretend it was delivered
out-of-band. Security comes from requiring approval by an *already
logged-in* device, not from the secrecy of a delivery channel.

## The flow, both sides

**On the lost/new device (no session) — `crypton-console`'s `/login` page:**

1. `client.recovery.requestRecovery(username)` →
   `POST /recovery/public/start` → creates a `recovery_requests` row
   (`status = 'pending'`) and returns `{ request: {status, method,
   expires_at}, recovery_token }`. The token is only returned on the call
   that *creates* the request — a repeat call while one is already pending
   omits it.
2. The UI shows the token to the user and tells them to approve from a
   trusted device, then come back.
3. `client.recovery.getRecoveryStatus(username)` →
   `GET /recovery/public/status?username=...` to poll/re-check.

**On a trusted device (already logged in) — `crypton-console`'s
`/recovery` page:**

4. `client.recovery.current()` → `GET /recovery` shows the pending request.
5. `client.recovery.approve(requestId)` → `POST /recovery/approve` flips it
   to `status = 'approved'`. (`reject` is also available.)

**Back on the lost/new device:**

6. `client.recovery.claimRecovery(recoveryToken, username)` →
   `POST /recovery/public/complete` with `{username, recovery_token}`.
   The backend verifies the token hash, flips the request to `completed`,
   and returns `enrollment_token` (a short-lived Redis-backed grant).
7. `client.auth.register(username, enrollmentToken)` →
   `POST /auth/register/start` with `enrollment_token` set. The backend
   recognizes this is re-enrollment on an *existing* account (normally
   blocked — see `existing_account_requires_authorized_enrollment` in
   `auth_webauthn.rs`) and lets a new passkey be added.

## Why this used to be broken

The backend design above was always correct. The bug was entirely in the
client: the SDK's `claimRecovery` sent `{request_id, username}` — a field
the backend's `PublicCompleteReq` doesn't have — while the backend requires
`{username, recovery_token}`, and `recovery_token` was never captured from
step 1 in the first place. Separately, `register()` had no parameter for
`enrollment_token`, so step 7 could never succeed either. Both are fixed in
`libs/crypton-sdk/src/modules/recovery.ts` and `auth.ts`. See `DECISIONS.md`.
