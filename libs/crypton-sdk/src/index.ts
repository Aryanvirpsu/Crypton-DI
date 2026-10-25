import { CryptonTransport } from './client';
import { AuthModule } from './modules/auth';
import { DevicesModule } from './modules/devices';
import { RecoveryModule } from './modules/recovery';
import { ActionsModule } from './modules/actions';
import { DashboardModule } from './modules/dashboard';
import { MeModule } from './modules/me';
import { AdminModule } from './modules/admin';

export { CryptonError, ERROR_CODES } from './errors';
export type { ErrorCode } from './errors';
export { parseJwt, getSessionToken, setSessionToken, clearSessionToken, TOKEN_KEY } from './session';
export type { JwtPayload } from './session';
export type { DeviceInfo } from './modules/devices';
export type { RecoveryRequest, PublicRecoveryStatus, PublicRecoveryStartResult, PublicRecoveryClaimResult } from './modules/recovery';
export type { ActionResult } from './modules/actions';
export type { DashboardStats, ActivityItem } from './modules/dashboard';
export type { Me } from './modules/me';
export type { AdminDashboardStats, UserSummary, AuditLogEntry } from './modules/admin';

export class CryptonClient {
  public auth: AuthModule;
  public devices: DevicesModule;
  public recovery: RecoveryModule;
  public actions: ActionsModule;
  public dashboard: DashboardModule;
  public me: MeModule;
  public admin: AdminModule;

  /** Internal transport — not part of the public API. */
  private readonly transport: CryptonTransport;

  constructor(endpoint: string) {
    this.transport = new CryptonTransport(endpoint);
    this.auth = new AuthModule(this.transport);
    this.devices = new DevicesModule(this.transport);
    this.recovery = new RecoveryModule(this.transport);
    this.actions = new ActionsModule(this.transport);
    this.dashboard = new DashboardModule(this.transport);
    this.me = new MeModule(this.transport);
    this.admin = new AdminModule(this.transport);
  }
}
