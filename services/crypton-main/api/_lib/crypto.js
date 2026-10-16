'use strict';

const {
  createCipheriv, createDecipheriv, createHash, createHmac, hkdfSync, randomBytes, randomUUID, timingSafeEqual,
} = require('node:crypto');

/**
 * Cryptography for the assessment API. One root secret (ASSESSMENT_MASTER_KEY) is split into
 * independent per-purpose keys with HKDF, so a key used to sign challenges is never also the
 * key that encrypts leads (the earlier single-key design reused one key for everything).
 *
 *   challenge   signs form tokens                 encrypt      AES-256-GCM for lead payloads
 *   email       keyed hash for throttle/dedupe    fingerprint  detects an altered retry
 *   ip          keyed hash of the client address (never stored in the clear)
 */

const MIN_AGE_MS = 1500; // a token cannot be spent the instant it is issued (basic automation speed bump)
const MAX_AGE_MS = 60 * 60 * 1000;
const UUID = /^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/;

function deriveKeys(masterHex) {
  const master = Buffer.from(masterHex, 'hex');
  const derive = (label) => Buffer.from(hkdfSync('sha256', master, Buffer.from('crypton-assessment-v1'), Buffer.from(label), 32));
  return {
    challenge: derive('challenge'),
    encrypt: derive('encrypt'),
    email: derive('email'),
    fingerprint: derive('fingerprint'),
    ip: derive('client-ip'),
    // Identifies which root key encrypted a row, so keys can be rotated. Not secret.
    keyId: createHash('sha256').update(master).digest('hex').slice(0, 8),
  };
}

const mac = (key, value) => createHmac('sha256', key).update(value).digest('hex');

function issueChallenge(keys, now = Date.now()) {
  const body = `${now}.${randomUUID()}`;
  return `${body}.${mac(keys.challenge, `challenge:${body}`)}`;
}

/** Stateless: everything needed to check a token is in the token and the key. */
function verifyChallenge(keys, token, now = Date.now()) {
  const bad = { ok: false, code: 400, error: 'verification' };
  if (typeof token !== 'string' || token.length > 200) return bad;
  const parts = token.split('.');
  if (parts.length !== 3) return bad;
  const [issued, id, signature] = parts;
  if (!/^\d{13}$/.test(issued) || !UUID.test(id) || !/^[a-f\d]{64}$/.test(signature)) return bad;
  const expected = mac(keys.challenge, `challenge:${issued}.${id}`);
  if (!timingSafeEqual(Buffer.from(signature), Buffer.from(expected))) return bad;
  const age = now - Number(issued);
  if (age < MIN_AGE_MS) return { ok: false, code: 425, error: 'too-fast' };
  if (age > MAX_AGE_MS) return { ok: false, code: 410, error: 'expired' };
  return { ok: true, id };
}

/** AES-256-GCM envelope: 12-byte IV | 16-byte tag | ciphertext. */
function encryptPayload(keys, payload, idempotencyKey) {
  const iv = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', keys.encrypt, iv);
  // Bind the ciphertext to its row and key: it cannot be copied onto another inquiry.
  cipher.setAAD(Buffer.from(`${keys.keyId}:${idempotencyKey}`));
  const body = Buffer.concat([cipher.update(JSON.stringify(payload), 'utf8'), cipher.final()]);
  return Buffer.concat([iv, cipher.getAuthTag(), body]);
}

function decryptPayload(keys, envelope, idempotencyKey) {
  const bytes = Buffer.from(envelope);
  const decipher = createDecipheriv('aes-256-gcm', keys.encrypt, bytes.subarray(0, 12));
  decipher.setAAD(Buffer.from(`${keys.keyId}:${idempotencyKey}`));
  decipher.setAuthTag(bytes.subarray(12, 28));
  return JSON.parse(Buffer.concat([decipher.update(bytes.subarray(28)), decipher.final()]).toString('utf8'));
}

/** Identical requests have identical fingerprints; an altered retry does not. */
const fingerprint = (keys, v) => mac(keys.fingerprint, `payload:${JSON.stringify([v.name, v.email, v.organization, v.context])}`);
const emailHash = (keys, email) => mac(keys.email, email.toLowerCase());
const clientHash = (keys, ip) => mac(keys.ip, String(ip));

module.exports = {
  MIN_AGE_MS, MAX_AGE_MS, deriveKeys, issueChallenge, verifyChallenge, encryptPayload, decryptPayload, fingerprint, emailHash, clientHash,
};
