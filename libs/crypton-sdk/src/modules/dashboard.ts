import { CryptonTransport } from '../client';

export interface DashboardStats {
  activeDevices: number;
  authEvents24h: number;
  securityScore: number;
}

export interface ActivityItem {
  id: string;
  ico: string;
  type: string;
  title: string;
  meta: string;
  time: string;
  link: string;
}

/** The signed-in user's own dashboard — real data, computed from
 *  their devices and audit_logs rows (see crypton-id routes/admin.rs). */
export class DashboardModule {
  constructor(private client: CryptonTransport) {}

  stats(): Promise<DashboardStats> {
    return this.client.get('/dashboard/stats');
  }

  activity(): Promise<ActivityItem[]> {
    return this.client.get('/dashboard/activity');
  }
}
