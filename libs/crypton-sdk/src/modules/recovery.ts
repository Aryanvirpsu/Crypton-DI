import { CryptonTransport } from '../client';

export interface RecoveryRequest {
  id: string;
  user_id: string;
  status: 'pending' | 'approved' | 'rejected' | 'completed';
  method: string;
  approved_by_credential_id: string | null;
  created_at: string;
  expires_at: string;
}

/**
 * Public (unauthenticated) recovery status. Deliberately excludes `id` and
 * `user_id` — these endpoints are reachable by anyone who knows a username,
 * so the backend never returns internal identifiers here.
 */
export interface PublicRecoveryStatus {
  status: 'pending' | 'approved' | 'rejected' | 'completed';
  method: string;
  expires_at: string;
}

export interface PublicRecoveryStartResult {
  request: PublicRecoveryStatus;
  /**
   * Only present on the call that *creates* the request (a repeat call
   * against an already-pending request omits it). This is the recovery
   * secret — there is no email/SMS delivery in this system, so the caller
   * is responsible for showing it to the user (e.g. "write this down, then
   * approve from a trusted device") and passing it back to claimRecovery().
   */
  recovery_token?: string;
}

export interface PublicRecoveryClaimResult {
  request: PublicRecoveryStatus;
  /** Pass this to auth.register(username, enrollmentToken) to enroll a new
   *  passkey on the existing account. */
  enrollment_token: string;
}

/**
 * Lost-device account recovery.
 *
 * The full flow (see docs/recovery-flow.md):
 *   1. On the new/lost device:  requestRecovery(username) → recovery_token
 *   2. On an existing trusted device (already logged in): start()/current()
 *      to see the pending request, then approve(requestId)
 *   3. Back on the new device: poll getRecoveryStatus(username) until
 *      status === 'approved'
 *   4. claimRecovery(recovery_token, username) → enrollment_token
 *   5. auth.register(username, enrollment_token) → new passkey enrolled
 */
export class RecoveryModule {
  constructor(private client: CryptonTransport) {}

  // ── Authenticated routes (require an existing valid session) ────────────

  start(): Promise<RecoveryRequest> {
    return this.client.post("/recovery/start", { method: "trusted_device" });
  }

  current(): Promise<{ request: RecoveryRequest | null }> {
    return this.client.get("/recovery");
  }

  approve(requestId: string): Promise<RecoveryRequest> {
    return this.client.post("/recovery/approve", { request_id: requestId });
  }

  reject(requestId: string): Promise<RecoveryRequest> {
    return this.client.post("/recovery/reject", { request_id: requestId });
  }

  complete(requestId: string): Promise<RecoveryRequest> {
    return this.client.post("/recovery/complete", { request_id: requestId });
  }

  // ── Public routes (lost-device recovery, no session) ────────────────────

  requestRecovery(username: string): Promise<PublicRecoveryStartResult> {
    return this.client.post("/recovery/public/start", { username });
  }

  getRecoveryStatus(username: string): Promise<{ request: PublicRecoveryStatus | null }> {
    return this.client.get(`/recovery/public/status?username=${encodeURIComponent(username)}`);
  }

  /**
   * @param recoveryToken the `recovery_token` returned by requestRecovery()
   * @param username the account's username
   */
  claimRecovery(recoveryToken: string, username: string): Promise<PublicRecoveryClaimResult> {
    return this.client.post("/recovery/public/complete", {
      username,
      recovery_token: recoveryToken,
    });
  }
}
