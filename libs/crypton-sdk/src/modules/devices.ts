import { CryptonTransport } from '../client';

export interface DeviceInfo {
  id: string;
  nickname: string | null;
  status: 'active' | 'lost' | 'revoked';
  user_agent: string | null;
  created_at: string | null;
  last_used_at: string | null;
}

export class DevicesModule {
  constructor(private client: CryptonTransport) {}

  list(): Promise<DeviceInfo[]> {
    return this.client.get("/devices");
  }

  revoke(id: string): Promise<{ status: string }> {
    return this.client.post(`/devices/${id}/revoke`);
  }

  markLost(id: string): Promise<{ status: string }> {
    return this.client.post(`/devices/${id}/mark-lost`);
  }
}
