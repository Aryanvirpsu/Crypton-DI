import { CryptonTransport } from '../client';
import { fromB64url, b64url } from '../utils/base64';
import { wrapWebAuthnError } from '../webauthn-errors';

export interface ActionResult {
  status: string;
  verified: boolean;
  intent: string;
}

/**
 * ActionsModule — step-up (re-authentication) verification.
 *
 * Proves the signed-in user still holds a registered passkey right now,
 * for a caller-supplied intent label (e.g. "confirm_role_change"). This
 * does not execute any business action itself — it just gives the caller
 * a fresh, audited proof-of-presence to gate one with.
 *
 *   1. challenge(intent) → backend issues a WebAuthn assertion challenge
 *   2. user signs with registered device (navigator.credentials.get)
 *   3. execute(challengeId, intent, assertion) → backend verifies + audits
 *
 * sign(intent) is the recommended one-shot wrapper for most callers.
 * challenge() and execute() are available as escape hatches for custom UI.
 */
export class ActionsModule {
  constructor(private client: CryptonTransport) {}

  /** Step 1 — request a challenge for the given action id. */
  async challenge(action: string): Promise<{ challenge_id: string; publicKey: any }> {
    return this.client.post('/actions/challenge', { action });
  }

  /** Step 3 — submit the signed assertion. */
  async execute(
    challengeId: string,
    action: string,
    assertion: PublicKeyCredential,
  ): Promise<ActionResult> {
    const resp = assertion.response as AuthenticatorAssertionResponse;
    return this.client.post('/actions/execute', {
      challenge_id: challengeId,
      action,
      assertion: {
        id:    assertion.id,
        rawId: b64url(assertion.rawId),
        type:  assertion.type,
        response: {
          clientDataJSON:    b64url(resp.clientDataJSON),
          authenticatorData: b64url(resp.authenticatorData),
          signature:         b64url(resp.signature),
          userHandle: resp.userHandle ? b64url(resp.userHandle) : null,
        },
      },
    });
  }

  /**
   * Full one-shot flow: challenge → WebAuthn prompt → execute.
   * Set UI to "signing" state before calling; awaiting this resolves to
   * the action result or throws CryptonError (including on user cancel).
   */
  async sign(action: string): Promise<ActionResult> {
    const start = await this.challenge(action);
    const pk = start.publicKey;

    const getOpts: PublicKeyCredentialRequestOptions = {
      ...pk,
      challenge: fromB64url(pk.challenge),
      allowCredentials: (pk.allowCredentials || []).map((c: any) => ({
        ...c,
        id: fromB64url(c.id),
      })),
    };

    let assertion: PublicKeyCredential;
    try {
      assertion = await navigator.credentials.get({ publicKey: getOpts }) as PublicKeyCredential;
    } catch (err) {
      wrapWebAuthnError(err);
    }

    return this.execute(start.challenge_id, action, assertion);
  }
}
