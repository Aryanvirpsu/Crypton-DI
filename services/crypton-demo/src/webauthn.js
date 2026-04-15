/** DEPRECATED — compatibility bridge only. Do not use in new code. */
import { _authRef } from './auth';
import { crypton, parseJwt } from './sdk';
import { b64url as sdkB64url, fromB64url as sdkFromB64url } from './sdk/utils/base64';

// Deprecated: use sdk utils if needed, or prefer internalizing
export const b64url = sdkB64url;
export const fromB64url = sdkFromB64url;

/** 
 * @deprecated Use `crypton.auth.register` instead 
 */
export async function doRegister(email) {
  console.warn("doRegister is deprecated. Use crypton.auth.register");
  return crypton.auth.register(email);
}

/** 
 * @deprecated Use `crypton.auth.login` instead 
 */
export async function doLogin(handle) {
  console.warn("doLogin is deprecated. Use crypton.auth.login");
  const result = await crypton.auth.login(handle);
  // Maintain legacy side effect — parseJwt returns JwtPayload | null
  const user = result.token ? parseJwt(result.token) : null;
  if (user && _authRef.setUser) {
    _authRef.setUser(user);
  }
  return result;
}

