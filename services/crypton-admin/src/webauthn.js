import { api } from './api';
import { setToken, parseJwt, _authRef } from './auth';

export function b64url(buf) {
  return btoa(String.fromCharCode(...new Uint8Array(buf)))
    .replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

export function fromB64url(str) {
  const s = str.replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(s);
  const buf = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) buf[i] = raw.charCodeAt(i);
  return buf.buffer;
}

export async function doRegister(email) {
  const start = await api.post("/auth/register/start", { username: email });
  const pk = start.publicKey;

  const createOpts = {
    ...pk,
    challenge: fromB64url(pk.challenge),
    user: { ...pk.user, id: fromB64url(pk.user.id) },
    excludeCredentials: (pk.excludeCredentials || []).map(c => ({ ...c, id: fromB64url(c.id) })),
  };

  const cred = await navigator.credentials.create({ publicKey: createOpts });

  const result = await api.post("/auth/register/finish", {
    challenge_id: start.challenge_id,
    attestation: {
      id: cred.id,
      rawId: b64url(cred.rawId),
      type: cred.type,
      response: {
        clientDataJSON:    b64url(cred.response.clientDataJSON),
        attestationObject: b64url(cred.response.attestationObject),
      },
    },
  });
  if (result.token) setToken(result.token);
  return result;
}

export async function doLogin(handle) {
  const username = handle.includes("@") ? handle : `${handle}@crypton.local`;
  const start = await api.post("/auth/login/start", { username });
  const pk = start.publicKey;

  const getOpts = {
    ...pk,
    challenge: fromB64url(pk.challenge),
    allowCredentials: (pk.allowCredentials || []).map(c => ({ ...c, id: fromB64url(c.id) })),
  };

  const assertion = await navigator.credentials.get({ publicKey: getOpts });

  const result = await api.post("/auth/login/finish", {
    challenge_id: start.challenge_id,
    assertion: {
      id: assertion.id,
      rawId: b64url(assertion.rawId),
      type: assertion.type,
      response: {
        clientDataJSON:    b64url(assertion.response.clientDataJSON),
        authenticatorData: b64url(assertion.response.authenticatorData),
        signature:         b64url(assertion.response.signature),
        userHandle: assertion.response.userHandle ? b64url(assertion.response.userHandle) : null,
      },
    },
  });
  if (result.token) {
    setToken(result.token);
    if (_authRef.setUser) _authRef.setUser(parseJwt(result.token));
  }
  return result;
}
