import { DatabaseSync } from 'node:sqlite';
import { createCipheriv, createDecipheriv, createHmac, randomBytes, randomUUID, timingSafeEqual } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

export function settings(env = process.env) {
  const production = env.NODE_ENV === 'production';
  if (production && (!env.INQUIRY_KEY || !env.INQUIRY_DB || !env.INQUIRY_ORIGIN)) throw new Error('Production requires INQUIRY_KEY, INQUIRY_DB and INQUIRY_ORIGIN.');
  const dbPath = resolve(env.INQUIRY_DB || '.private/inquiries.sqlite');
  mkdirSync(dirname(dbPath), { recursive: true, mode: 0o700 });
  let keyHex = env.INQUIRY_KEY;
  if (!keyHex && !production) {
    const file = resolve('.private/inquiry.key');
    mkdirSync(dirname(file), { recursive: true, mode: 0o700 });
    if (!existsSync(file) && existsSync(dbPath)) throw new Error('Existing inbox has no local encryption key. Restore the key; refusing to replace it.');
    if (!existsSync(file)) writeFileSync(file, randomBytes(32).toString('hex'), { flag: 'wx', mode: 0o600 });
    keyHex = readFileSync(file, 'utf8').trim();
  }
  if (!/^[a-f\d]{64}$/i.test(keyHex || '')) throw new Error('INQUIRY_KEY must be a 32-byte hex key.');
  const origin = new URL(env.INQUIRY_ORIGIN || 'http://localhost:3000');
  if (production && origin.protocol !== 'https:') throw new Error('Production origin must use HTTPS.');
  if (origin.origin !== origin.href.replace(/\/$/, '')) throw new Error('INQUIRY_ORIGIN must be an origin, without a path.');
  return { dbPath, key: Buffer.from(keyHex, 'hex'), origin: origin.origin, production };
}

export function openInbox({ dbPath, key, now = Date.now }) {
  const db = new DatabaseSync(dbPath);
  db.exec(`PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000; PRAGMA secure_delete=ON;
    CREATE TABLE IF NOT EXISTS requests (receipt TEXT PRIMARY KEY, created INTEGER NOT NULL, fingerprint TEXT NOT NULL, email_hash TEXT NOT NULL, payload TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS requests_email ON requests(email_hash, created);
    CREATE TABLE IF NOT EXISTS rate_limits (bucket TEXT PRIMARY KEY, count INTEGER NOT NULL, expires INTEGER NOT NULL);`);
  const mac = (value) => createHmac('sha256', key).update(value).digest('hex');
  const prune = () => {
    db.prepare('DELETE FROM rate_limits WHERE expires < ?').run(now());
    db.prepare('DELETE FROM requests WHERE created < ?').run(now() - 90 * 86400000);
  };
  const timer = setInterval(prune, 3600000); timer.unref();
  const encrypt = (value) => {
    const iv = randomBytes(12);
    const cipher = createCipheriv('aes-256-gcm', key, iv);
    const body = Buffer.concat([cipher.update(JSON.stringify(value), 'utf8'), cipher.final()]);
    return Buffer.concat([iv, cipher.getAuthTag(), body]).toString('base64');
  };
  const decrypt = (value) => {
    const bytes = Buffer.from(value, 'base64');
    const decipher = createDecipheriv('aes-256-gcm', key, bytes.subarray(0, 12));
    decipher.setAuthTag(bytes.subarray(12, 28));
    return JSON.parse(Buffer.concat([decipher.update(bytes.subarray(28)), decipher.final()]).toString('utf8'));
  };
  const first = db.prepare('SELECT payload FROM requests LIMIT 1').get();
  if (first) { try { decrypt(first.payload); } catch { clearInterval(timer); db.close(); throw new Error('Inbox encryption key does not match existing data.'); } }
  prune();
  function rate(ip) {
    const time = now();
    const bucket = mac(`ip:${ip}:${Math.floor(time / 60000)}`);
    const row = db.prepare(`INSERT INTO rate_limits VALUES (?, 1, ?) ON CONFLICT(bucket) DO UPDATE SET count=count+1 RETURNING count`).get(bucket, time + 120000);
    return row.count <= 60;
  }
  function challenge() {
    const body = `${now()}.${randomUUID()}`;
    return `${body}.${mac(`challenge:${body}`)}`;
  }
  function accept(input) {
    if (!input || Array.isArray(input) || typeof input !== 'object') return { code: 422, error: 'validation' };
    const limits = { name: 80, email: 254, organization: 120, context: 1000 };
    const errors = {};
    const value = {};
    for (const [field, limit] of Object.entries(limits)) {
      if (typeof input[field] !== 'string' || input[field].length > limit || /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/.test(input[field])) errors[field] = 'Check this field and try again.';
      else value[field] = input[field].trim();
    }
    if (!errors.name && value.name.length < 2) errors.name = 'Enter your name (at least 2 characters).';
    if (!errors.email && !/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(value.email)) errors.email = 'Enter a valid email address.';
    if (input.consent !== true) errors.consent = 'Please confirm we may contact you about this request.';
    if (Object.keys(errors).length) return { code: 422, error: 'validation', errors };
    if (input.source !== 'agent-floor' || input.website !== '' || typeof input.token !== 'string' || input.token.length > 200) return { code: 400, error: 'verification' };
    const parts = input.token.split('.');
    const [issued, receipt, signature] = parts;
    const body = `${issued}.${receipt}`;
    const expected = mac(`challenge:${body}`);
    if (parts.length !== 3 || !/^\d{13}$/.test(issued) || !/^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/.test(receipt || '') || !/^[a-f\d]{64}$/.test(signature || '') || !timingSafeEqual(Buffer.from(signature), Buffer.from(expected))) return { code: 400, error: 'verification' };
    const age = now() - Number(issued);
    if (age < 1500) return { code: 425, error: 'too-fast' };
    if (age > 3600000) return { code: 410, error: 'expired' };
    const payload = { ...value, consent: true, source: 'agent-floor', privacyVersion: '2026-10-14' };
    const fingerprint = mac(`payload:${JSON.stringify(payload)}`);
    const emailHash = mac(`email:${value.email.toLowerCase()}`);
    db.exec('BEGIN IMMEDIATE');
    try {
      const previous = db.prepare('SELECT fingerprint FROM requests WHERE receipt=?').get(receipt);
      if (previous) {
        db.exec('ROLLBACK');
        return previous.fingerprint === fingerprint ? { code: 200, status: 'received', receipt } : { code: 409, error: 'changed-retry' };
      }
      const recent = db.prepare('SELECT count(*) AS n FROM requests WHERE email_hash=? AND created>?').get(emailHash, now() - 3600000);
      if (recent.n >= 3) { db.exec('ROLLBACK'); return { code: 429, error: 'rate-limit' }; }
      db.prepare('INSERT INTO requests VALUES (?, ?, ?, ?, ?)').run(receipt, now(), fingerprint, emailHash, encrypt(payload));
      db.exec('COMMIT');
      return { code: 201, status: 'received', receipt };
    } catch (error) { db.exec('ROLLBACK'); throw error; }
  }
  return { rate, challenge, accept, list: () => db.prepare('SELECT receipt, created, payload FROM requests ORDER BY created DESC').all().map((r) => ({ receipt: r.receipt, created: new Date(r.created).toISOString(), ...decrypt(r.payload) })), close: () => { clearInterval(timer); db.close(); } };
}
