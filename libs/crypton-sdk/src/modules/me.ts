import { CryptonTransport } from '../client';

export interface Me {
  user_id: string;
  username: string;
  role: 'user' | 'admin' | 'super_admin';
}

/**
 * The current caller's own identity + role, decoded server-side from the
 * same JWT used for every other request. This is the one source of truth
 * a frontend should use to decide whether to show admin UI — there is no
 * separate "admin token" in this system.
 */
export class MeModule {
  constructor(private client: CryptonTransport) {}

  get(): Promise<Me> {
    return this.client.get('/me');
  }
}
