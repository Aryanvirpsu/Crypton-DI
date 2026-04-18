import { CryptonTransport } from '../client';
import { CryptonError } from '../errors';
import { fromB64url, b64url } from '../utils/base64';
import { setSessionToken, clearSessionToken } from '../session';

function wrapWebAuthnError(err: unknown): never {
  if (err instanceof DOMException) {
    const code =
      err.name === 'NotAllowedError' ? 'webauthn_cancelled' :
      err.name === 'InvalidStateError' ? 'webauthn_already_registered' :
      'webauthn_security_error';
    throw new CryptonError(code, err.message);
  }
  throw err;
}

export class AuthModule {
  constructor(private client: CryptonTransport) {}

  async register(email: string): Promise<{ user_id: string, token?: string }> {
    const start: any = await this.client.post("/auth/register/start", { username: email });
    const pk = start.publicKey;

    const createOpts: PublicKeyCredentialCreationOptions = {
      ...pk,
      challenge: fromB64url(pk.challenge),
      user: { ...pk.user, id: fromB64url(pk.user.id) },
      excludeCredentials: (pk.excludeCredentials || []).map((c: any) => ({ ...c, id: fromB64url(c.id) })),
    };

    let cred: PublicKeyCredential;
    try {
      cred = await navigator.credentials.create({ publicKey: createOpts }) as PublicKeyCredential;
    } catch (err) {
      wrapWebAuthnError(err);
    }

    const result: any = await this.client.post("/auth/register/finish", {
      challenge_id: start.challenge_id,
      attestation: {
        id: cred.id,
        rawId: b64url(cred.rawId),
        type: cred.type,
        response: {
          clientDataJSON: b64url((cred.response as AuthenticatorAttestationResponse).clientDataJSON),
          attestationObject: b64url((cred.response as AuthenticatorAttestationResponse).attestationObject),
        },
      },
    });

    if (result.token) setSessionToken(result.token);
    return result;
  }

  async login(handle: string): Promise<{ token: string }> {
    const username = handle.includes("@") ? handle : `${handle}@crypton.local`;
    const start: any = await this.client.post("/auth/login/start", { username });
    const pk = start.publicKey;

    const getOpts: PublicKeyCredentialRequestOptions = {
      ...pk,
      challenge: fromB64url(pk.challenge),
      allowCredentials: (pk.allowCredentials || []).map((c: any) => ({ ...c, id: fromB64url(c.id) })),
    };

    let assertion: PublicKeyCredential;
    try {
      assertion = await navigator.credentials.get({ publicKey: getOpts }) as PublicKeyCredential;
    } catch (err) {
      wrapWebAuthnError(err);
    }

    const result: any = await this.client.post("/auth/login/finish", {
      challenge_id: start.challenge_id,
      assertion: {
        id: assertion.id,
        rawId: b64url(assertion.rawId),
        type: assertion.type,
        response: {
          clientDataJSON: b64url((assertion.response as AuthenticatorAssertionResponse).clientDataJSON),
          authenticatorData: b64url((assertion.response as AuthenticatorAssertionResponse).authenticatorData),
          signature: b64url((assertion.response as AuthenticatorAssertionResponse).signature),
          userHandle: (assertion.response as AuthenticatorAssertionResponse).userHandle
            ? b64url((assertion.response as AuthenticatorAssertionResponse).userHandle as ArrayBuffer)
            : null,
        },
      },
    });

    if (result.token) setSessionToken(result.token);
    return result;
  }

  /** Server-side logout: denylists the JWT in Redis, then clears local token. */
  async logout(): Promise<void> {
    try {
      await this.client.post("/auth/logout");
    } catch {
      // Best-effort: if server is unreachable, still clear local state
    }
    clearSessionToken();
  }
}
