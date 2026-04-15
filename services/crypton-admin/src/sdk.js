// Thin shim — source of truth is libs/crypton-sdk (@crypton/sdk).
// The singleton lives here so app env config is applied in one place.
import { CryptonClient, CryptonError, parseJwt, getSessionToken, setSessionToken, clearSessionToken } from '@crypton/sdk';

export const crypton = new CryptonClient(process.env.REACT_APP_API_BASE || "");
export { CryptonError, parseJwt, getSessionToken, setSessionToken, clearSessionToken };
