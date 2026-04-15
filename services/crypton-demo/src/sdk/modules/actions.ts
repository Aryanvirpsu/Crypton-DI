import { CryptonTransport } from '../client';
import { fromB64url, b64url } from '../utils/base64';

export interface ActionResult {
  status: string;
  result: Record<string, unknown>;
}

/**
 * ActionsModule — WebAuthn-protected server-side actions.
 *
 * Each action follows the pattern:
 *   1. challenge(action) → backend issues a WebAuthn assertion challenge
 *   2. user signs with registered device (navigator.credentials.get)
 *   3. execute(challengeId, action, assertion) → backend verifies + runs action
 *
 * sign(action) is a convenience wrapper for callers that manage their own
 * loading state and want the full flow in one await.
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
   * the action result or throws (including NotAllowedError on cancel).
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

    const assertion = await navigator.credentials.get({ publicKey: getOpts }) as PublicKeyCredential;
    return this.execute(start.challenge_id, action, assertion);
  }
}
