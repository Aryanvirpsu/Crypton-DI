import { CryptonTransport } from '../client';
import { ActivityItem } from './dashboard';

export interface AdminDashboardStats {
  totalUsers: number;
  activeCredentials: number;
  lostCredentials: number;
  revokedCredentials: number;
  authEvents24h: number;
  failedAuths24h: number;
}

export interface UserSummary {
  id: string;
  username: string;
  role: 'user' | 'admin' | 'super_admin';
  created_at: string | null;
  device_count: number;
}

export interface AuditLogEntry {
  id: string;
  actor: string | null;
  event_type: string;
  status: string;
  credential_id: string | null;
  metadata: unknown;
  created_at: string | null;
}

/**
 * Admin-only surface. Every method here 403s unless the caller's role is
 * `admin` or `super_admin` (crypton-id/src/jwt.rs AdminUser). There is no
 * client-side role gate anywhere in the SDK by design — check `me.get()`
 * to decide what UI to show, but the server is what actually enforces it.
 */
export class AdminModule {
  constructor(private client: CryptonTransport) {}

  stats(): Promise<AdminDashboardStats> {
    return this.client.get('/admin/dashboard/stats');
  }

  activity(): Promise<ActivityItem[]> {
    return this.client.get('/admin/dashboard/activity');
  }

  listUsers(): Promise<UserSummary[]> {
    return this.client.get('/admin/users');
  }

  updateUserRole(id: string, role: 'user' | 'admin' | 'super_admin'): Promise<{ status: string }> {
    return this.client.patch(`/admin/users/${id}/role`, { role });
  }

  /**
   * Promotes the *caller* to super_admin. Only succeeds while the
   * deployment has zero admins — this is how a fresh install gets its
   * first admin without manual SQL. Returns 403 once any admin exists.
   */
  bootstrap(): Promise<{ status: string }> {
    return this.client.post('/admin/bootstrap');
  }

  auditLogs(): Promise<{ logs: AuditLogEntry[] }> {
    return this.client.get('/audit-logs');
  }

  /** Real CSV export — returns the raw Response so the caller can .blob() it. */
  exportAuditLogsCsv(): Promise<Response> {
    return this.client.rawFetch('/export/audit-logs');
  }
}
