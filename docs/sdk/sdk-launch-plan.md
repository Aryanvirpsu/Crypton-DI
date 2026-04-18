# Crypton SDK — Production Launch Plan
**Date:** 2026-04-15  
**Branch:** dev  
**Review type:** /plan-design-review  
**Scope:** SDK production launch — architecture, public surface, internal boundaries, blockers, execution

---

## TL;DR

The SDK exists and works. The WebAuthn plumbing in `auth.ts` and `actions.ts` is solid. The
`CryptonError` pattern with `.code` and `.is()` is clean. But right now the SDK is not a product —
it's a private implementation detail copied in three places, baked into a CRA singleton, and
shipping a contact form submission function alongside passkey authentication.

Seven things must happen before this is a credible v1. They are listed in order in section E.

---

## Pass 1 — Information Architecture

### Q1: Ideal SDK position in the product architecture

The correct position: the SDK is the **only** public-facing integration contract. Everything else
is internal.

```
integrator's app
  └── import { CryptonClient } from "@crypton/sdk"
           └── CryptonTransport (internal)
                    └── HTTP → crypton-gateway :8090
                              └── crypton-identity :8080
```

`crypton-admin` and `crypton-demo` become first-party integrators — they consume `@crypton/sdk`
exactly as a third party would. This is the right way to dog-food your own SDK. If something is
awkward for admin to use, it's awkward for everyone.

### Q2: One canonical source of truth

Right now the SDK lives in three places:

| Location | State |
|---|---|
| `services/crypton-admin/src/sdk/` | Full, working, current |
| `services/crypton-demo/src/sdk/` | Full, byte-for-byte identical copy |
| `services/crypton-demo/demo-site/src/sdk/` | Partial scaffold, modules empty |
| `libs/contracts/` | Empty, never populated |

`libs/contracts/` was the right instinct. It just never got populated.

The fix: create `libs/crypton-sdk/`, extract the canonical source there, wire npm workspaces,
delete the two copies. One source. One version. Two consumers.

### Q3: Public vs internal

**Public (goes into `@crypton/sdk`):**
- `CryptonClient` — the class users instantiate
- `CryptonError` — error type they need to catch
- `AuthModule` — register, login, logout
- `DevicesModule` — list, revoke, markLost
- `RecoveryModule` — both authenticated and unauthenticated paths
- `ActionsModule` — sign (full one-shot flow); challenge/execute as lower-level escape hatches
- Types: `RecoveryRequest`, `DeviceInfo`, `ActionResult`, `JwtPayload`

**Internal (not exported, stays in consuming apps):**
- `CryptonTransport` — implementation detail; should not appear in public types
- `api.js` bridge — legacy compatibility shim in `crypton-admin`
- `session.ts` utilities (`getSessionToken`, `setSessionToken`) — currently exported, but they
  leak storage internals. Consider making them internal and exposing a `crypton.session.token`
  getter only if integrators actually need it.
- `contact.ts` — not an auth SDK concern. Move to `crypton-demo` directly.

**Admin-only functionality (NOT in SDK, lives in `crypton-admin` as direct API calls):**
- Session management (`GET/DELETE /sessions`) — admin panel only
- User/role management (`GET/PATCH /users/:id/role`) — admin panel only
- Org settings (`GET/PATCH /org`) — admin panel only
- Policy engine (`GET/PATCH /policies/:id`) — admin panel only

These are backend admin operations. Putting them in a public SDK invites misuse. Keep them as
direct `api.js` calls or a separate internal `AdminClient` that is never published.

---

## Pass 2 — Interaction State Coverage

### Q4: Namespace design

The current surface is almost right. One removal, one clarification:

```typescript
const crypton = new CryptonClient("https://api.yourapp.com");

// Auth — correct
await crypton.auth.register("alice@example.com");
await crypton.auth.login("alice@example.com");
await crypton.auth.logout();

// Devices — correct
const devices = await crypton.devices.list();
await crypton.devices.revoke(deviceId);
await crypton.devices.markLost(deviceId);

// Actions — correct, but expose only sign() in v1 docs; challenge/execute are escape hatches
await crypton.actions.sign("delete_account");

// Recovery — correct but needs clearer split in docs
// User-facing (no auth required):
await crypton.recovery.requestRecovery("alice@example.com");
await crypton.recovery.getRecoveryStatus("alice@example.com");
await crypton.recovery.claimRecovery(requestId, "alice@example.com");

// Admin/trusted-device-facing (requires auth):
await crypton.recovery.start();
await crypton.recovery.current();
await crypton.recovery.approve(requestId);
await crypton.recovery.reject(requestId);
await crypton.recovery.complete(requestId);

// REMOVE:
// crypton.contact — not auth SDK scope
```

**Error codes — need a registry.** Right now `CryptonTransport` emits `"session_expired"` and
`"network_error"`. WebAuthn throws raw `DOMException` errors that currently bubble up unwrapped.
Define the full error surface before v1:

| Code | When |
|---|---|
| `session_expired` | 401 from backend |
| `network_error` | Non-JSON response or fetch failure |
| `webauthn_cancelled` | `NotAllowedError` from `navigator.credentials` |
| `webauthn_already_registered` | `InvalidStateError` from `navigator.credentials.create` |
| `webauthn_security_error` | `SecurityError` from `navigator.credentials` |
| `challenge_expired` | Backend rejects stale challenge |
| `device_not_found` | 404 on revoke/markLost |

Without this registry, integrators write `catch(err) { console.error(err) }` and ship it.

---

## Pass 3 — Developer Onboarding

### Q7: Minimum docs and examples for launch

**Target:** Under 15 minutes from `npm install @crypton/sdk` to a working passkey login.

**Minimum viable:**

1. `README.md` — install + 3 examples: register, login, error handling
2. One complete integration example in `crypton-demo` (already exists, just needs to import from
   the package instead of a local copy)
3. TypeScript types exported so IDEs autocomplete — already done, just needs the package boundary

**Not required for v1:** full API reference, JSDoc site, changelog, migration guide. Ship README
first. Add JSDoc later.

Example README quickstart (exact copy-pasteable):

```typescript
import { CryptonClient, CryptonError } from "@crypton/sdk";

const crypton = new CryptonClient("https://api.yourapp.com");

// Register
try {
  await crypton.auth.register("alice@example.com");
  // user is now registered and logged in
} catch (err) {
  if (CryptonError.is(err) && err.code === "webauthn_cancelled") {
    // user dismissed the browser prompt
  }
}

// Login
try {
  await crypton.auth.login("alice@example.com");
} catch (err) {
  if (CryptonError.is(err) && err.code === "session_expired") {
    // redirect to login page
  }
}
```

That's it for v1. Three examples, one file.

---

## Pass 4 — AI Slop Risk

### Things that don't belong

**`contact.ts` in the auth SDK.** `ContactModule.submit()` POSTs a name/email/message/company
form to `/api/contact`. This is a CRM lead capture form. It has zero relationship to passkey
authentication. It is in the SDK because someone was already using `CryptonTransport` and it was
convenient. Convenience is not a good reason to pollute the public surface of an auth SDK. Remove
it. Move it to `crypton-demo`'s own `api.js` as a plain fetch call.

**`challenge()` and `execute()` exposed publicly.** The split-step pattern (challenge → sign →
execute) is correct for the implementation. But for the public SDK surface, `sign(action)` is the
90% use case. Exposing `challenge()` and `execute()` as documented public API in v1 means you have
to support them forever. Keep them, but document only `sign()`. Add a comment that the lower-level
methods exist for advanced use (e.g., custom UI between challenge and assertion).

**`transport` exposed as public field.** `CryptonClient.transport` is public, with a JSDoc comment
that says "do not use in new code; prefer typed module methods." That comment will be ignored. The
field is typed `public readonly` and will appear in autocomplete. Change it to `private` or
`protected`. The `api.js` bridge that uses it can be refactored to call module methods instead, or
kept as `// @ts-expect-error internal` for the transition period.

**Singleton export.** `export const crypton = new CryptonClient(process.env.REACT_APP_API_BASE || "")`
is a hard dependency on a CRA environment variable. In Vite it's `import.meta.env.VITE_API_BASE`.
In Next.js it's something else. In a test environment it's `""` and every request goes to
`localhost`. Do not export a pre-configured singleton. Export the class. Let callers own lifecycle.

---

## Pass 5 — TypeScript Consistency

The module structure is clean. `AuthModule`, `DevicesModule`, etc. are well-shaped classes with
private constructor injection. `CryptonError` with `.is()` static narrowing is good TypeScript
design — the consumer doesn't need to import the class to narrow the type.

Two issues before public release:

**1. `types.ts` exports `ApiResponse` as `any`-based.** The `ApiResponse<T>` shape
(`{ success: true, data: T } | { success: false, error: { code: string, message: string } }`) is
the wire protocol. Integrators who build their own wrappers need this. Export it. But also — the
`json.error.code` access at `client.ts:34` has no type safety, `json.error` could be `undefined`
if the backend returns `{ success: false }` without an error field. Add a guard.

**2. `RecoveryModule` mixes auth-required and no-auth methods.** A developer who calls
`crypton.recovery.approve(id)` without a session will get a 401 and a confusing error. The methods
are documented with comments (`// Authenticated routes`, `// Unauthenticated routes`) but the
module itself gives no signal at the type level. For v1, a comment grouping in the docs is
sufficient. For v2, consider `crypton.recovery.admin` vs `crypton.recovery` split.

---

## Pass 6 — Environment Compatibility

### Q6: Packaging and release shape for credible v1

The SDK uses `navigator.credentials` in `auth.ts` and `actions.ts`. This is a hard browser
requirement. WebAuthn does not run in Node.js, Deno, or Bun without polyfills. This is fine and
expected — WebAuthn is a browser API. But document it.

**Package format for v1:**
- TypeScript source in `src/`
- Compiled to `dist/` with `tsc` — two outputs: `dist/esm/` (ESM) and `dist/cjs/` (CommonJS)
- `package.json` exports map pointing to the right build for the consumer's module system
- `"browser": true` or `"environment": "browser"` in the package metadata to signal that Node SSR
  will fail at runtime for `auth.*` and `actions.*` calls

Minimum `package.json` for `libs/crypton-sdk`:

```json
{
  "name": "@crypton/sdk",
  "version": "0.1.0",
  "description": "Passkey-native authentication SDK",
  "main": "dist/cjs/index.js",
  "module": "dist/esm/index.js",
  "types": "dist/types/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/esm/index.js",
      "require": "./dist/cjs/index.js",
      "types": "./dist/types/index.d.ts"
    }
  },
  "files": ["dist"],
  "scripts": {
    "build": "tsc --project tsconfig.build.json",
    "typecheck": "tsc --noEmit"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  },
  "peerDependencies": {}
}
```

**npm workspace configuration** in root `package.json`:

```json
{
  "workspaces": ["libs/*", "services/*"]
}
```

This makes `@crypton/sdk` resolvable in `crypton-admin` and `crypton-demo` as a local workspace
package without publishing to npm.

---

## Pass 7 — Unresolved Decisions

### Q5: What belongs in SDK scope

**Yes, in SDK:**
- Passkey registration and login — core purpose
- Device list/revoke/markLost — user's own devices, directly tied to auth session
- Recovery — both user-facing and admin/trusted-device flows
- WebAuthn-protected actions — natural extension of the auth surface

**No, not in SDK:**
- Admin session management — killing other users' sessions is an admin privilege, not a user action
- User/role management — RBAC is a backend policy concern, not the client SDK's job
- Org settings, policy engine — same
- Contact form submission — unrelated to auth

**Borderline:** `session.ts` exports (`getSessionToken`, `setSessionToken`, `clearSessionToken`).
These give integrators direct access to the JWT. Needed for apps that want to use the token
for their own authenticated requests (e.g., fetching data not through the SDK). For v1, export
them. Document them as "low-level, only use if you need the raw token." In v2, consider a
`crypton.session.getToken()` accessor to keep the surface on the client object.

### Q8: Migration path from current structure to launch-ready

Four phases. Each is independently shippable.

**Phase 1 (2 hours): Extract SDK to package**
- Create `libs/crypton-sdk/` with `package.json`, `tsconfig.json`
- Copy source from `crypton-admin/src/sdk/` — it's the most current copy
- Remove `contact.ts`
- Remove singleton export
- Change `transport` to private
- Add npm workspaces to root `package.json`

**Phase 2 (1 hour): Wire consumers**
- Update `crypton-admin` to import from `@crypton/sdk`
- Update `crypton-demo` to import from `@crypton/sdk`
- Delete `services/crypton-admin/src/sdk/`
- Delete `services/crypton-demo/src/sdk/`
- Delete `services/crypton-demo/demo-site/src/sdk/`

**Phase 3 (2 hours): Harden the surface**
- Add WebAuthn error wrapping in `auth.ts` and `actions.ts`
- Add error code registry (constants file or exported enum)
- Fix `client.ts:34` null guard on `json.error`
- Add `tsconfig.build.json` and run `tsc` to verify dist output

**Phase 4 (1 hour): Docs**
- Write `libs/crypton-sdk/README.md` with quickstart
- Add 3 integration tests using `jest-environment-jsdom` and a mocked `navigator.credentials`
- Update `libs/contracts/` README to point to `libs/crypton-sdk/`

---

## Output A — Recommended Launch Architecture

```
crypton/
  libs/
    crypton-sdk/                  ← THE SDK (new, canonical)
      package.json                  name: @crypton/sdk, version: 0.1.0
      tsconfig.json
      tsconfig.build.json
      README.md
      src/
        index.ts                    exports: CryptonClient, CryptonError, types
        client.ts                   CryptonTransport (internal, private)
        session.ts                  token storage (exported for advanced use)
        errors.ts                   CryptonError + error code constants
        modules/
          auth.ts
          devices.ts
          recovery.ts
          actions.ts
        utils/
          base64.ts
      dist/                         generated, gitignored
  services/
    crypton-admin/                ← imports @crypton/sdk (first-party consumer)
      src/
        sdk/                        DELETE (replaced by workspace package)
        api.js                      thin bridge or deleted
    crypton-demo/                 ← imports @crypton/sdk (first-party consumer)
      src/
        sdk/                        DELETE
    crypton-identity/             ← backend (unchanged)
    crypton-gateway/              ← backend (unchanged)
  package.json                    add "workspaces": ["libs/*", "services/*"]
```

---

## Output B — Public SDK Surface (v1)

```typescript
import { CryptonClient, CryptonError } from "@crypton/sdk";
import type { DeviceInfo, RecoveryRequest, ActionResult, JwtPayload } from "@crypton/sdk";

// Instantiation — caller owns lifecycle
const crypton = new CryptonClient("https://api.yourapp.com");

// ── Auth ──────────────────────────────────────────────────────────────────
await crypton.auth.register(email: string): Promise<{ user_id: string; token?: string }>
await crypton.auth.login(handle: string): Promise<{ token: string }>
await crypton.auth.logout(): Promise<void>

// ── Devices ───────────────────────────────────────────────────────────────
await crypton.devices.list(): Promise<DeviceInfo[]>
await crypton.devices.revoke(id: string): Promise<void>
await crypton.devices.markLost(id: string): Promise<void>

// ── Actions ───────────────────────────────────────────────────────────────
await crypton.actions.sign(action: string): Promise<ActionResult>
// (challenge + execute available as escape hatches, not in v1 docs)

// ── Recovery ──────────────────────────────────────────────────────────────
// User-facing (no session required):
await crypton.recovery.requestRecovery(username: string): Promise<RecoveryRequest>
await crypton.recovery.getRecoveryStatus(username: string): Promise<{ request: RecoveryRequest | null }>
await crypton.recovery.claimRecovery(requestId: string, username: string): Promise<RecoveryRequest>

// Admin/trusted-device (session required):
await crypton.recovery.start(): Promise<RecoveryRequest>
await crypton.recovery.current(): Promise<{ request: RecoveryRequest | null }>
await crypton.recovery.approve(requestId: string): Promise<RecoveryRequest>
await crypton.recovery.reject(requestId: string): Promise<RecoveryRequest>
await crypton.recovery.complete(requestId: string): Promise<RecoveryRequest>

// ── Session (low-level) ───────────────────────────────────────────────────
import { getSessionToken, setSessionToken, clearSessionToken } from "@crypton/sdk";

// ── Error handling ────────────────────────────────────────────────────────
try {
  await crypton.auth.login("alice@example.com");
} catch (err) {
  if (CryptonError.is(err)) {
    // err.code is one of the registered error codes
    switch (err.code) {
      case "session_expired":     break;  // 401 from backend
      case "webauthn_cancelled":  break;  // user dismissed browser prompt
      case "webauthn_already_registered": break;
      case "network_error":       break;
    }
  }
}
```

---

## Output C — What Stays Internal

| Concern | Location | Why not in SDK |
|---|---|---|
| `api.js` bridge | `crypton-admin/src/api.js` | Legacy compatibility shim; consuming apps only |
| Session management (kill/killAll) | `crypton-admin/src/Sessions.js` | Admin privilege, not user action |
| User/role management | `crypton-admin/src/RBAC.js` | Backend policy, not client SDK scope |
| Org settings | `crypton-admin/src/OrgSettings.js` | Admin-only configuration |
| Policy engine | `crypton-admin/src/PolicyEngine.js` | Admin-only toggle |
| `contact.submit()` | Move to `crypton-demo` | CRM form, not auth |
| `CryptonTransport` | SDK-internal | Implementation detail; make field private |
| Singleton (`export const crypton`) | Remove from SDK | CRA-coupled, non-portable |

---

## Output D — Structural Blockers

These must be resolved before calling this a public SDK. Each is a blocking issue — not "nice to
have."

**BLOCKER-1: Three SDK copies, no package boundary.**
The SDK source exists in three places and has no `package.json`. You cannot `npm install` it.
You cannot version it. A bug fixed in one copy is not fixed in the others.
Fix: extract to `libs/crypton-sdk/`, wire workspaces, delete copies.

**BLOCKER-2: Singleton baked to CRA environment variable.**
`process.env.REACT_APP_API_BASE` is a CRA-specific env variable convention. This singleton will
silently use `""` as the endpoint in any non-CRA environment (Vite, Next.js, test runner, plain
Node). Every request will go to the wrong place and fail without a clear error.
Fix: remove the singleton export. Callers pass the endpoint.

**BLOCKER-3: `contact.ts` in the auth SDK.**
Shipping a CRM form submission function as part of an authentication SDK is a credibility issue,
not just a code smell. Anyone evaluating the SDK will see `crypton.contact.submit()` and
correctly conclude that the surface wasn't designed carefully.
Fix: delete from SDK, move to `crypton-demo`.

**BLOCKER-4: WebAuthn errors not wrapped.**
`navigator.credentials.create()` and `.get()` throw raw `DOMException` errors
(`NotAllowedError`, `InvalidStateError`, `SecurityError`). These currently bypass `CryptonError`
entirely. Integrators have to catch both `CryptonError` and `DOMException` with different type
guards, or they miss errors.
Fix: wrap in try/catch in `auth.ts` and `actions.ts`, throw `CryptonError` with standard codes.

**BLOCKER-5: `transport` field is public.**
`CryptonClient.transport` is typed `public readonly`. It will appear in IDE autocomplete. The
comment says "do not use in new code" — that comment will be ignored. Make it private.
Fix: `private readonly transport: CryptonTransport` and refactor `api.js` bridge to use module
methods.

**BLOCKER-6: npm workspaces not configured.**
Root `package.json` has no `workspaces` field. The monorepo structure is scaffolded but
the package graph is not wired. Workspace packages are not resolvable by name.
Fix: add `"workspaces": ["libs/*", "services/*"]` to root `package.json`.

---

## Output E — Execution Plan

Ordered by dependency. Each step is independently committable.

### Phase 1 — Package Extraction (2 hours)

**Step 1.1** — Create `libs/crypton-sdk/` with the following files:
- `package.json` (see Pass 6 above for content)
- `tsconfig.json` — base TypeScript config
- `tsconfig.build.json` — build config that emits to `dist/`
- `src/` — copy from `services/crypton-admin/src/sdk/`

**Step 1.2** — Remove `contact.ts` and its import from `index.ts`.

**Step 1.3** — Remove the singleton export from `index.ts`:
```diff
-export const crypton = new CryptonClient(process.env.REACT_APP_API_BASE || "");
```

**Step 1.4** — Make `transport` private in `CryptonClient`:
```diff
-  public readonly transport: CryptonTransport;
+  private readonly transport: CryptonTransport;
```
(The `api.js` bridge uses this field. Annotate the field access in `api.js` with `// @ts-ignore — internal bridge, remove when api.js is refactored`.)

**Step 1.5** — Add WebAuthn error wrapping to `auth.ts`:
```typescript
try {
  const cred = await navigator.credentials.create({ publicKey: createOpts });
} catch (err) {
  if (err instanceof DOMException) {
    const code = err.name === "NotAllowedError" ? "webauthn_cancelled"
      : err.name === "InvalidStateError" ? "webauthn_already_registered"
      : "webauthn_security_error";
    throw new CryptonError(code, err.message);
  }
  throw err;
}
```
Same pattern in `actions.ts` for `navigator.credentials.get`.

**Step 1.6** — Wire workspaces in root `package.json`:
```json
{
  "workspaces": ["libs/*", "services/*"]
}
```

**Commit:** `feat(sdk): extract @crypton/sdk to libs/crypton-sdk`

---

### Phase 2 — Consumer Wiring (1 hour)

**Step 2.1** — Update `services/crypton-admin/package.json`:
```json
{
  "dependencies": {
    "@crypton/sdk": "*"
  }
}
```

**Step 2.2** — Update all `crypton-admin` imports:
```diff
-import { crypton, getSessionToken as getToken, CryptonError } from './sdk';
+import { CryptonClient, getSessionToken as getToken, CryptonError } from '@crypton/sdk';
+
+const crypton = new CryptonClient(process.env.REACT_APP_API_BASE || "");
```
(The singleton now lives in the app, not the SDK.)

**Step 2.3** — Same for `crypton-demo`.

**Step 2.4** — Delete:
- `services/crypton-admin/src/sdk/` (entire directory)
- `services/crypton-demo/src/sdk/` (entire directory)
- `services/crypton-demo/demo-site/src/sdk/` (partial scaffold)

**Step 2.5** — Run `npm install` from repo root to link workspaces. Verify admin still compiles.

**Commit:** `feat(sdk): wire @crypton/sdk workspace in admin and demo`

---

### Phase 3 — Surface Hardening (1 hour)

**Step 3.1** — Add error code constants to `errors.ts`:
```typescript
export const ERROR_CODES = {
  SESSION_EXPIRED: "session_expired",
  NETWORK_ERROR: "network_error",
  WEBAUTHN_CANCELLED: "webauthn_cancelled",
  WEBAUTHN_ALREADY_REGISTERED: "webauthn_already_registered",
  WEBAUTHN_SECURITY_ERROR: "webauthn_security_error",
  CHALLENGE_EXPIRED: "challenge_expired",
  DEVICE_NOT_FOUND: "device_not_found",
} as const;

export type ErrorCode = typeof ERROR_CODES[keyof typeof ERROR_CODES];
```

**Step 3.2** — Add null guard in `client.ts:34`:
```diff
-    throw new CryptonError(json.error.code, json.error.message);
+    throw new CryptonError(json.error?.code ?? "unknown_error", json.error?.message ?? "An error occurred");
```

**Step 3.3** — Run `tsc --noEmit` in `libs/crypton-sdk/`. Fix any type errors.

**Commit:** `fix(sdk): harden error surface — wrap WebAuthn DOMExceptions, add error code registry`

---

### Phase 4 — Docs (1 hour)

**Step 4.1** — Write `libs/crypton-sdk/README.md`:
- One-line description
- Install command
- 3 copy-pasteable examples: register, login, error handling
- Browser requirement note

**Step 4.2** — Add 3 integration tests in `libs/crypton-sdk/src/__tests__/`:
- `auth.test.ts` — mock `navigator.credentials`, verify token is stored on successful login
- `error-wrapping.test.ts` — mock `navigator.credentials` to throw `NotAllowedError`, verify
  `CryptonError` with code `"webauthn_cancelled"` is thrown
- `transport.test.ts` — mock fetch returning 401, verify `session_expired` error code

**Commit:** `docs(sdk): add README quickstart and integration tests`

---

## Health Assessment

| Dimension | Before | After E |
|---|---|---|
| Package boundary | 0/10 | 9/10 |
| Public surface design | 5/10 | 8/10 |
| Error handling | 4/10 | 8/10 |
| Documentation | 0/10 | 6/10 |
| Browser safety | 6/10 | 9/10 |
| Monorepo structure | 3/10 | 9/10 |
| **Overall SDK readiness** | **2/10** | **8/10** |

**Estimated total effort:** ~5 hours of focused engineering work.  
**Minimum for credible v1:** Phases 1-3 (BLOCKER-1 through BLOCKER-6 resolved). Phase 4 is
required before showing the SDK to anyone external.

---

*Generated by /plan-design-review on 2026-04-15*
