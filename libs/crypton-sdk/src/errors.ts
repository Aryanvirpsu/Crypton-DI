export class CryptonError extends Error {
  public readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'CryptonError';
    this.code = code;
    // Restores correct prototype chain when transpiled to ES5
    Object.setPrototypeOf(this, new.target.prototype);
  }

  /** Type-safe narrowing without needing to import the class */
  static is(err: unknown): err is CryptonError {
    return err instanceof CryptonError;
  }
}

/** Registry of all error codes the SDK can throw. */
export const ERROR_CODES = {
  /** 401 from backend — session token expired or revoked. */
  SESSION_EXPIRED: "session_expired",
  /** Non-JSON response or network-level fetch failure. */
  NETWORK_ERROR: "network_error",
  /** User dismissed the browser WebAuthn prompt (NotAllowedError). */
  WEBAUTHN_CANCELLED: "webauthn_cancelled",
  /** Credential already registered on this device (InvalidStateError). */
  WEBAUTHN_ALREADY_REGISTERED: "webauthn_already_registered",
  /** Origin mismatch or insecure context (SecurityError). */
  WEBAUTHN_SECURITY_ERROR: "webauthn_security_error",
  /** Backend rejected a stale or replayed challenge. */
  CHALLENGE_EXPIRED: "challenge_expired",
  /** Device ID not found (404 on revoke/markLost). */
  DEVICE_NOT_FOUND: "device_not_found",
} as const;

export type ErrorCode = typeof ERROR_CODES[keyof typeof ERROR_CODES];
