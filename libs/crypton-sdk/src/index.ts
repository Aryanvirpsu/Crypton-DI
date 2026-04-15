import { CryptonTransport } from './client';
import { AuthModule } from './modules/auth';
import { DevicesModule } from './modules/devices';
import { RecoveryModule } from './modules/recovery';
import { ActionsModule } from './modules/actions';

export { CryptonError, ERROR_CODES } from './errors';
export type { ErrorCode } from './errors';
export { parseJwt, getSessionToken, setSessionToken, clearSessionToken } from './session';
export type { JwtPayload } from './session';
export type { DeviceInfo } from './modules/devices';
export type { RecoveryRequest } from './modules/recovery';
export type { ActionResult } from './modules/actions';

export class CryptonClient {
  public auth: AuthModule;
  public devices: DevicesModule;
  public recovery: RecoveryModule;
  public actions: ActionsModule;

  /** Internal transport — not part of the public API. */
  private readonly transport: CryptonTransport;

  constructor(endpoint: string) {
    this.transport = new CryptonTransport(endpoint);
    this.auth = new AuthModule(this.transport);
    this.devices = new DevicesModule(this.transport);
    this.recovery = new RecoveryModule(this.transport);
    this.actions = new ActionsModule(this.transport);
  }
}
