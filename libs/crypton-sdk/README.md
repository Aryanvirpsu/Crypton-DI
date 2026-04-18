# @crypton/sdk

Passkey-native authentication SDK for browser applications. Sign users in with WebAuthn (biometric, hardware key, or PIN).

## Installation

```bash
npm install @crypton/sdk
```

## Requirements

- **Browser-only** — uses WebAuthn, which requires `navigator.credentials` API
- Target environment must have WebAuthn support (all modern browsers)
- Your backend must implement the Crypton auth API endpoints

## Quick Start

Initialize the client with your backend endpoint:

```typescript
import { CryptonClient, CryptonError } from '@crypton/sdk';

const crypton = new CryptonClient('https://your-api.example.com');
```

### Register a new user

```typescript
try {
  // Step 1: Get registration challenge from backend
  // Step 2: User creates credential with their authenticator
  // Step 3: Send credential back to backend for verification
  const result = await crypton.auth.register('user@example.com');

  // result.token is the JWT for this user
  // Save it to localStorage or a secure session store
  localStorage.setItem('auth_token', result.token);
} catch (err) {
  if (err instanceof CryptonError) {
    console.error(`Auth failed: ${err.code} — ${err.message}`);
  }
}
```

### Log in an existing user

```typescript
try {
  const result = await crypton.auth.login('user@example.com');
  localStorage.setItem('auth_token', result.token);
  // User is now authenticated
} catch (err) {
  if (CryptonError.is(err)) {
    if (err.code === 'webauthn_cancelled') {
      console.log('User dismissed the authenticator prompt');
    } else if (err.code === 'webauthn_security_error') {
      console.error('WebAuthn security check failed — wrong origin or insecure context');
    }
  }
}
```

### Error Handling

All SDK errors are `CryptonError` instances with a `.code` field:

```typescript
import { CryptonError, ERROR_CODES } from '@crypton/sdk';

try {
  await crypton.auth.login('user@example.com');
} catch (err) {
  if (CryptonError.is(err)) {
    switch (err.code) {
      case ERROR_CODES.WEBAUTHN_CANCELLED:
        // User cancelled the authenticator prompt
        break;
      case ERROR_CODES.SESSION_EXPIRED:
        // JWT has expired — ask user to log in again
        break;
      case ERROR_CODES.NETWORK_ERROR:
        // Network failure — check connectivity
        break;
      default:
        console.error(`Unexpected error: ${err.code} — ${err.message}`);
    }
  }
}
```

## API

### CryptonClient

Main entry point. Initialize with your backend URL.

#### `auth.register(email: string)`
Register a new user. Returns `{ user_id, token? }`.

#### `auth.login(email: string)`
Authenticate an existing user. Returns `{ token }`.

#### `auth.logout()`
Server-side logout (best-effort).

#### `devices.list()`
List registered authenticators for the logged-in user.

#### `devices.revoke(id: string)`
Revoke an authenticator by ID.

#### `recovery.requestRecovery(username: string)`
Initiate account recovery (unauthenticated).

## Error Codes

See `ERROR_CODES` export for the complete list:

- `webauthn_cancelled` — User dismissed the authenticator prompt
- `webauthn_already_registered` — This credential is already registered
- `webauthn_security_error` — WebAuthn security check failed (origin mismatch, insecure context)
- `session_expired` — JWT has expired or was revoked
- `network_error` — Network request failed
- `challenge_expired` — WebAuthn challenge is stale
- `device_not_found` — Authenticator ID not found

## Type-Safe Error Narrowing

Use `CryptonError.is()` to narrow error types:

```typescript
try {
  await crypton.auth.login(email);
} catch (err) {
  if (CryptonError.is(err)) {
    // err is now typed as CryptonError
    console.log(err.code, err.message);
  } else {
    // Non-SDK error (network timeout, etc.)
    throw err;
  }
}
```

## Architecture

The SDK encapsulates:
- **Auth**: WebAuthn-native registration and login flows
- **Devices**: Manage registered authenticators
- **Recovery**: Account recovery with trusted devices
- **Actions**: WebAuthn-protected server-side actions (requires `crypton.actions.sign()`)

All errors are wrapped in `CryptonError` with typed codes. WebAuthn browser prompts are handled transparently — just await the method and catch errors.

## License

MIT
