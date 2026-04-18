export const TOKEN_KEY = "crypton_token";
export const getSessionToken = (): string | null => localStorage.getItem(TOKEN_KEY);
export const setSessionToken = (t: string) => localStorage.setItem(TOKEN_KEY, t);
export const clearSessionToken = () => localStorage.removeItem(TOKEN_KEY);

/** Shape issued by crypton-identity's issue_jwt — sub = user UUID */
export interface JwtPayload {
  sub: string;
  username: string;
  cred_id: string;
  iat: number;
  exp: number;
}

export function parseJwt(token: string): JwtPayload | null {
  try {
    return JSON.parse(atob(token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/'))) as JwtPayload;
  } catch {
    return null;
  }
}
