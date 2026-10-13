import { CryptonError } from './errors';

/** Maps a browser WebAuthn DOMException to a CryptonError with a stable code. */
export function wrapWebAuthnError(err: unknown): never {
  if (err instanceof DOMException) {
    const code =
      err.name === 'NotAllowedError' ? 'webauthn_cancelled' :
      err.name === 'InvalidStateError' ? 'webauthn_already_registered' :
      'webauthn_security_error';
    throw new CryptonError(code, err.message);
  }
  throw err;
}
