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

export class RecoveryModule {
  constructor(private client: CryptonTransport) {}

  // ── Authenticated routes (require JWT) ──────────────────────────────────

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

  // ── Unauthenticated routes (no JWT required) ────────────────────────────

  requestRecovery(username: string): Promise<RecoveryRequest> {
    return this.client.post("/recovery/public/start", { username });
  }

  getRecoveryStatus(username: string): Promise<{ request: RecoveryRequest | null }> {
    return this.client.get(`/recovery/public/status?username=${encodeURIComponent(username)}`);
  }

  claimRecovery(requestId: string, username: string): Promise<RecoveryRequest> {
    return this.client.post("/recovery/public/complete", { request_id: requestId, username });
  }
}
