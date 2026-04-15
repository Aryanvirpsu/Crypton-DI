import { CryptonTransport } from './client';
import { AuthModule } from './modules/auth';
import { DevicesModule } from './modules/devices';
import { RecoveryModule } from './modules/recovery';
import { ContactModule } from './modules/contact';
import { ActionsModule } from './modules/actions';

export { CryptonError } from './errors';
export { parseJwt, getSessionToken, setSessionToken, clearSessionToken } from './session';
export type { JwtPayload } from './session';
export type { DeviceInfo } from './modules/devices';
export type { RecoveryRequest } from './modules/recovery';
export type { ActionResult } from './modules/actions';

export class CryptonClient {
  public auth: AuthModule;
  public devices: DevicesModule;
  public recovery: RecoveryModule;
  public contact: ContactModule;
  public actions: ActionsModule;

  /** Exposed for legacy api.js bridge — do not use in new code; prefer typed module methods. */
  public readonly transport: CryptonTransport;

  constructor(endpoint: string) {
    this.transport = new CryptonTransport(endpoint);
    this.auth = new AuthModule(this.transport);
    this.devices = new DevicesModule(this.transport);
    this.recovery = new RecoveryModule(this.transport);
    this.contact = new ContactModule(this.transport);
    this.actions = new ActionsModule(this.transport);
  }
}

// Global singleton for React injection
export const crypton = new CryptonClient(process.env.REACT_APP_API_BASE || "");
