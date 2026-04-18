import { getToken, setToken, clearToken, parseJwt, getValidToken } from './auth';

// Regression: ISSUE-001 — go() / popstate / mount guard passed expired tokens
// Found by /qa on 2026-04-15
// Report: .gstack/qa-reports/qa-report-crypton-admin-2026-04-15.md

const TOKEN_KEY = 'crypton_token';

function makeJwt(payload) {
  const enc = (str) =>
    btoa(str).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  return `${enc('{"alg":"HS256"}')}.${enc(JSON.stringify(payload))}.fakesig`;
}

const VALID_PAYLOAD = {
  sub: 'user-1',
  username: 'alice',
  cred_id: 'cred-1',
  iat: Math.floor(Date.now() / 1000),
  exp: Math.floor(Date.now() / 1000) + 3600,
};

const EXPIRED_PAYLOAD = {
  ...VALID_PAYLOAD,
  exp: Math.floor(Date.now() / 1000) - 60, // expired 1 min ago
};

describe('getValidToken — expiry-aware token check', () => {
  beforeEach(() => localStorage.removeItem(TOKEN_KEY));
  afterEach(() => localStorage.removeItem(TOKEN_KEY));

  it('returns null when no token is stored', () => {
    expect(getValidToken()).toBeNull();
  });

  it('returns the raw token when a valid (non-expired) token is stored', () => {
    const tok = makeJwt(VALID_PAYLOAD);
    localStorage.setItem(TOKEN_KEY, tok);
    expect(getValidToken()).toBe(tok);
    // token should still be in storage
    expect(localStorage.getItem(TOKEN_KEY)).toBe(tok);
  });

  it('returns null and clears storage when the token is expired', () => {
    const tok = makeJwt(EXPIRED_PAYLOAD);
    localStorage.setItem(TOKEN_KEY, tok);
    expect(getValidToken()).toBeNull();
    // must be cleared — otherwise go() and the mount guard let the expired holder through
    expect(localStorage.getItem(TOKEN_KEY)).toBeNull();
  });

  it('returns null and clears storage when the token is malformed', () => {
    localStorage.setItem(TOKEN_KEY, 'not.a.real.jwt');
    expect(getValidToken()).toBeNull();
    expect(localStorage.getItem(TOKEN_KEY)).toBeNull();
  });

  it('returns null and clears storage when the token has no exp field', () => {
    const noExpPayload = { sub: 'user-1', username: 'alice', cred_id: 'cred-1', iat: 1000 };
    localStorage.setItem(TOKEN_KEY, makeJwt(noExpPayload));
    expect(getValidToken()).toBeNull();
    expect(localStorage.getItem(TOKEN_KEY)).toBeNull();
  });

  it('returns null for a token expiring exactly at now (boundary)', () => {
    const nowPayload = { ...VALID_PAYLOAD, exp: Math.floor(Date.now() / 1000) };
    localStorage.setItem(TOKEN_KEY, makeJwt(nowPayload));
    // exp * 1000 === Date.now() → not strictly greater → treated as expired
    expect(getValidToken()).toBeNull();
    expect(localStorage.getItem(TOKEN_KEY)).toBeNull();
  });
});
