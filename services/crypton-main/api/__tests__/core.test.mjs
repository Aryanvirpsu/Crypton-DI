// Unit tests: no database. Run with `npm run test:api` from services/crypton-main.
import test, { describe } from 'node:test';
import assert from 'node:assert/strict';
import { Readable } from 'node:stream';
import { randomBytes } from 'node:crypto';
import { lib, ORIGIN } from './harness.mjs';

const { crypto, config, validate, handler } = lib;
const hex = () => randomBytes(32).toString('hex');
const good = { ASSESSMENT_MASTER_KEY: hex(), ASSESSMENT_DATABASE_URL: 'postgres://assessment_writer:SECRETPW@db.example.invalid:6543/postgres', ASSESSMENT_ALLOWED_ORIGINS: ORIGIN };

describe('configuration fails closed', () => {
  test('missing everything names what is missing, and only names', () => {
    const r = config.loadConfig({});
    assert.equal(r.ok, false);
    assert.deepEqual(r.missing.sort(), ['ASSESSMENT_ALLOWED_ORIGINS', 'ASSESSMENT_DATABASE_URL', 'ASSESSMENT_MASTER_KEY']);
  });

  test('a malformed secret is rejected without being echoed back', () => {
    const r = config.loadConfig({ ...good, ASSESSMENT_MASTER_KEY: 'topsecret-not-hex', ASSESSMENT_DATABASE_URL: 'mysql://x' });
    assert.equal(r.ok, false);
    assert.ok(!JSON.stringify(r).includes('topsecret'));
    assert.deepEqual(r.missing.sort(), ['ASSESSMENT_DATABASE_URL', 'ASSESSMENT_MASTER_KEY']);
  });

  test('production accepts only https origins, and only bare origins', () => {
    const prod = { ...good, VERCEL_ENV: 'production' };
    assert.equal(config.loadConfig({ ...prod, ASSESSMENT_ALLOWED_ORIGINS: 'http://localhost:3000' }).ok, false);
    assert.equal(config.loadConfig({ ...prod, ASSESSMENT_ALLOWED_ORIGINS: 'https://cryptonid.tech/path' }).ok, false);
    const ok = config.loadConfig({ ...prod, ASSESSMENT_ALLOWED_ORIGINS: 'https://cryptonid.tech, https://www.cryptonid.tech/' });
    assert.equal(ok.ok, true);
    assert.deepEqual([...ok.origins].sort(), ['https://cryptonid.tech', 'https://www.cryptonid.tech']);
  });

  test('preview deployments may use their own generated hostnames; production may not', () => {
    const env = { ...good, ASSESSMENT_ALLOWED_ORIGINS: ORIGIN, VERCEL_URL: 'crypton-abc.vercel.app', VERCEL_BRANCH_URL: 'crypton-git-x.vercel.app' };
    assert.ok(config.loadConfig({ ...env, VERCEL_ENV: 'preview' }).origins.has('https://crypton-abc.vercel.app'));
    const prod = config.loadConfig({ ...env, VERCEL_ENV: 'production', ASSESSMENT_ALLOWED_ORIGINS: 'https://cryptonid.tech' });
    assert.ok(!prod.origins.has('https://crypton-abc.vercel.app'));
  });

  test('on Vercel a Preview is not production even though NODE_ENV is "production"', () => {
    const env = { ...good, NODE_ENV: 'production', VERCEL_URL: 'crypton-abc.vercel.app' };
    delete env.ASSESSMENT_ALLOWED_ORIGINS;
    assert.ok(config.loadConfig({ ...env, VERCEL_ENV: 'preview' }).origins.has('https://crypton-abc.vercel.app'));
    assert.equal(config.loadConfig({ ...env, VERCEL_ENV: 'production' }).ok, false, 'production still requires explicit origins');
    assert.equal(config.loadConfig({ ...env }).ok, false, 'off-platform NODE_ENV=production is still production');
  });

  test('the reason a setting failed is reported without its value', () => {
    const r = config.loadConfig({ ASSESSMENT_MASTER_KEY: 'topsecret-not-hex', ASSESSMENT_DATABASE_URL: 'mysql://x:SECRETPW@h/db' });
    assert.match(r.why.ASSESSMENT_MASTER_KEY, /^malformed\(length \d+\)$/);
    assert.equal(r.why.ASSESSMENT_DATABASE_URL, 'malformed');
    assert.equal(r.why.ASSESSMENT_ALLOWED_ORIGINS, 'unset');
    assert.ok(!JSON.stringify(r).includes('topsecret') && !JSON.stringify(r).includes('SECRETPW'));
    assert.equal(config.loadConfig({}).why.ASSESSMENT_MASTER_KEY, 'unset');
  });

  test('TLS to the database: verified by default; "disable" only for local development', () => {
    assert.deepEqual(config.loadConfig(good).ssl, { rejectUnauthorized: true });
    assert.equal(config.loadConfig({ ...good, ASSESSMENT_DATABASE_SSL: 'disable' }).ok, false, 'a remote host must not allow plaintext');
    assert.equal(config.loadConfig({ ...good, ASSESSMENT_DATABASE_SSL: 'disable', ASSESSMENT_DATABASE_URL: 'postgres://u:p@127.0.0.1:5432/db' }).ok, true);
    assert.equal(config.loadConfig({ ...good, ASSESSMENT_DATABASE_SSL: 'disable', ASSESSMENT_DATABASE_URL: 'postgres://u:p@127.0.0.1:5432/db', VERCEL_ENV: 'production', ASSESSMENT_ALLOWED_ORIGINS: 'https://cryptonid.tech' }).ok, false, 'never in production');
    assert.equal(config.loadConfig({ ...good, ASSESSMENT_DATABASE_SSL: 'nonsense' }).ok, false);
    const ca = config.loadConfig({ ...good, ASSESSMENT_DATABASE_CA: '-----BEGIN CERTIFICATE-----\\nAAA\\n-----END CERTIFICATE-----' });
    assert.match(ca.ssl.ca, /BEGIN CERTIFICATE-----\nAAA\n/);
  });
});

describe('cryptography', () => {
  const keys = crypto.deriveKeys(hex());

  test('every purpose gets its own key', () => {
    const all = [keys.challenge, keys.encrypt, keys.email, keys.fingerprint, keys.ip].map((k) => k.toString('hex'));
    assert.equal(new Set(all).size, 5);
    assert.notEqual(crypto.deriveKeys(hex()).keyId, keys.keyId);
  });

  test('challenge tokens: too early, valid, expired', () => {
    const t0 = 1_800_000_000_000;
    const token = crypto.issueChallenge(keys, t0);
    assert.deepEqual(crypto.verifyChallenge(keys, token, t0 + 100), { ok: false, code: 425, error: 'too-fast' });
    const ok = crypto.verifyChallenge(keys, token, t0 + 2000);
    assert.equal(ok.ok, true);
    assert.match(ok.id, /^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/);
    assert.deepEqual(crypto.verifyChallenge(keys, token, t0 + 61 * 60 * 1000), { ok: false, code: 410, error: 'expired' });
    assert.equal(crypto.verifyChallenge(keys, token, t0 - 5000).code, 425, 'a token from the future is not valid');
  });

  test('challenge tokens cannot be forged, edited, or borrowed from another key', () => {
    const t0 = 1_800_000_000_000;
    const token = crypto.issueChallenge(keys, t0);
    const [issued, id, sig] = token.split('.');
    for (const bad of [`${issued}.${id}.${'0'.repeat(64)}`, `${Number(issued) + 1}.${id}.${sig}`, `${issued}.${crypto.issueChallenge(keys, t0).split('.')[1]}.${sig}`, 'x', '', `${token}.extra`, 'a'.repeat(300), 5, null, undefined, {}]) {
      assert.equal(crypto.verifyChallenge(keys, bad, t0 + 3000).code, 400, String(bad).slice(0, 30));
    }
    assert.equal(crypto.verifyChallenge(crypto.deriveKeys(hex()), token, t0 + 3000).code, 400);
  });

  test('payload encryption: round trip, fresh IV, no plaintext, bound to its row', () => {
    const payload = { name: 'Ada Lovelace', email: 'ada@example.invalid', organization: 'AE', context: 'secret system notes', consent: true };
    const id = '11111111-1111-4111-8111-111111111111';
    const a = crypto.encryptPayload(keys, payload, id);
    const b = crypto.encryptPayload(keys, payload, id);
    assert.notDeepEqual(a, b);
    assert.ok(!a.toString('latin1').includes('Ada') && !a.toString('latin1').includes('example.invalid') && !a.toString('latin1').includes('secret'));
    assert.deepEqual(crypto.decryptPayload(keys, a, id), payload);
    assert.throws(() => crypto.decryptPayload(keys, a, '22222222-2222-4222-8222-222222222222'), 'moved to another row');
    const flipped = Buffer.from(a); flipped[40] ^= 1;
    assert.throws(() => crypto.decryptPayload(keys, flipped, id), 'tampered');
    assert.throws(() => crypto.decryptPayload(crypto.deriveKeys(hex()), a, id), 'wrong key');
  });

  test('fingerprints and hashes: stable, sensitive to every field, email case-insensitive', () => {
    const v = { name: 'Ada', email: 'Ada@Example.invalid', organization: 'x', context: 'y' };
    assert.equal(crypto.fingerprint(keys, v), crypto.fingerprint(keys, { ...v }));
    for (const f of Object.keys(v)) assert.notEqual(crypto.fingerprint(keys, { ...v, [f]: `${v[f]}!` }), crypto.fingerprint(keys, v), f);
    assert.equal(crypto.emailHash(keys, 'ADA@example.invalid'), crypto.emailHash(keys, 'ada@example.invalid'));
    assert.notEqual(crypto.clientHash(keys, '1.2.3.4'), crypto.clientHash(keys, '1.2.3.5'));
  });
});

describe('validation matches the original service exactly', async () => {
  let accept;
  try {
    const { openInbox } = await import('../../../crypton-inquiries/store.mjs');
    accept = openInbox({ dbPath: ':memory:', key: Buffer.alloc(32, 7) }).accept;
  } catch { /* the original service is optional; parity is checked while it exists */ }
  const base = { name: 'Ada Lovelace', email: 'ada@example.invalid', organization: '', context: '', consent: true, source: 'agent-floor', website: '', token: 'not-a-real-token' };
  const vectors = [
    ['null', null], ['array', []], ['string', 'x'], ['empty object', {}],
    ['short name', { ...base, name: 'A' }], ['numeric name', { ...base, name: 4 }], ['bad email', { ...base, email: 'bad' }],
    ['long email', { ...base, email: `${'a'.repeat(250)}@x.invalid` }], ['long context', { ...base, context: 'x'.repeat(1001) }],
    ['object organization', { ...base, organization: {} }], ['string consent', { ...base, consent: 'true' }], ['no consent', { ...base, consent: false }],
    ['control character', { ...base, context: 'a\u0007b' }], ['null byte', { ...base, name: 'Ad\u0000a' }],
    ['filled honeypot', { ...base, website: 'http://spam.example' }], ['wrong source', { ...base, source: 'other' }], ['missing token', { ...base, token: undefined }],
    ['valid fields, bad token', base],
  ];
  for (const [label, input] of vectors) {
    test(label, { skip: !accept && 'original service not present' }, () => {
      const mine = validate.validateSubmission(input);
      const theirs = accept(input);
      if (mine.ok) { assert.equal(theirs.code, 400); assert.equal(theirs.error, 'verification'); return; }
      assert.equal(mine.code, theirs.code);
      assert.equal(mine.error, theirs.error);
      assert.deepEqual(mine.errors, theirs.errors);
    });
  }
});

// ── the handler, with a store we control ─────────────────────────────────────────────────
function mockReq({ method = 'POST', headers = {}, body, chunks, remote = '203.0.113.9' } = {}) {
  const data = chunks || (body === undefined ? [] : [Buffer.from(typeof body === 'string' ? body : JSON.stringify(body))]);
  const req = Readable.from(data);
  req.method = method;
  req.headers = { origin: ORIGIN, 'content-type': 'application/json', ...headers };
  req.socket = { remoteAddress: remote };
  return req;
}
function mockRes() {
  return { statusCode: 0, headers: {}, body: '', headersSent: false, writeHead(c, h) { this.statusCode = c; this.headers = h; this.headersSent = true; }, end(b) { this.body = b || ''; } };
}
const run = async (fn, req) => { const res = mockRes(); await fn(req, res); return { status: res.statusCode, json: res.body ? JSON.parse(res.body) : null, headers: res.headers, raw: res.body }; };

describe('the handler tells the truth', () => {
  const T0 = 1_800_000_000_000;
  const env = { ...good };
  const keys = crypto.deriveKeys(env.ASSESSMENT_MASTER_KEY);
  const setup = (storeImpl, extraEnv = {}) => {
    const calls = [];
    const logs = [];
    const store = { async submit(args) { calls.push(args); return storeImpl(args); } };
    const api = handler.createApi({ env: { ...env, ...extraEnv }, store, now: () => T0 + 5000, log: { error: (...a) => logs.push(a.join(' ')) } });
    return { api, calls, logs };
  };
  const body = (over = {}) => ({ name: 'Ada Lovelace', email: 'ada@example.invalid', organization: 'AE', context: 'private notes', consent: true, source: 'agent-floor', website: '', token: crypto.issueChallenge(keys, T0), ...over });
  const RECEIPT = '33333333-3333-4333-8333-333333333333';

  test('success is reported only after the store confirms, and the store only ever sees ciphertext', async () => {
    const { api, calls } = setup(async () => ({ status: 201, outcome: 'received', id: RECEIPT }));
    const b = body();
    const r = await run(api.submit, mockReq({ body: b }));
    assert.equal(r.status, 201);
    assert.deepEqual(r.json, { status: 'received', receipt: RECEIPT });
    assert.equal(calls.length, 1);
    assert.ok(!calls[0].payload.toString('latin1').includes('Ada') && !calls[0].payload.toString('latin1').includes('private'));
    const stored = crypto.decryptPayload(keys, calls[0].payload, calls[0].idempotencyKey);
    assert.equal(stored.email, 'ada@example.invalid');
    assert.equal(stored.privacyVersion, handler.PRIVACY_VERSION);
    assert.equal(calls[0].idempotencyKey, b.token.split('.')[1], 'idempotency key comes from the signed token, not the client');
  });

  test('an identical retry answers 200 with the same receipt', async () => {
    const { api } = setup(async () => ({ status: 200, outcome: 'received', id: RECEIPT }));
    const r = await run(api.submit, mockReq({ body: body() }));
    assert.equal(r.status, 200);
    assert.equal(r.json.receipt, RECEIPT);
  });

  test('any storage failure is a 503, never a success, and nothing sensitive is logged or returned', async () => {
    const secretError = Object.assign(new Error('connect failed for postgres://assessment_writer:SECRETPW@db for ada@example.invalid'), { code: 'ECONNREFUSED' });
    const { api, logs } = setup(async () => { throw secretError; });
    const r = await run(api.submit, mockReq({ body: body() }));
    assert.equal(r.status, 503);
    assert.deepEqual(r.json, { error: 'unavailable' });
    assert.ok(!r.raw.includes('received'));
    const text = logs.join('\n') + r.raw;
    for (const leak of ['SECRETPW', 'ada@example', 'assessment_writer', env.ASSESSMENT_MASTER_KEY, 'private notes']) assert.ok(!text.includes(leak), `leaked ${leak}`);
    assert.match(logs[0], /ECONNREFUSED/);
  });

  test('a storage answer we cannot vouch for is never turned into success', async () => {
    for (const odd of [{ status: 500, outcome: 'x', id: RECEIPT }, { status: 202, outcome: 'queued', id: RECEIPT }, { status: 201, outcome: 'received', id: 'not-a-uuid' }, { status: 201, outcome: 'received', id: null }]) {
      const { api } = setup(async () => odd);
      const r = await run(api.submit, mockReq({ body: body() }));
      assert.equal(r.status, 503, JSON.stringify(odd));
      assert.ok(!r.raw.includes('received'));
    }
  });

  test('changed retry and throttling map to their statuses', async () => {
    let r = await run(setup(async () => ({ status: 409, outcome: 'changed-retry', id: null })).api.submit, mockReq({ body: body() }));
    assert.deepEqual([r.status, r.json], [409, { error: 'changed-retry' }]);
    r = await run(setup(async () => ({ status: 429, outcome: 'rate-limit', id: null })).api.submit, mockReq({ body: body() }));
    assert.deepEqual([r.status, r.json, r.headers['Retry-After']], [429, { error: 'rate-limit' }, '60']);
  });

  test('requests that cannot succeed never reach the database', async () => {
    const cases = [
      ['invalid email', () => mockReq({ body: body({ email: 'nope' }) }), 422],
      ['no consent', () => mockReq({ body: body({ consent: false }) }), 422],
      ['honeypot', () => mockReq({ body: body({ website: 'x' }) }), 400],
      ['forged token', () => mockReq({ body: body({ token: `${T0}.${'a'.repeat(8)}-aaaa-4aaa-8aaa-${'a'.repeat(12)}.${'0'.repeat(64)}` }) }), 400],
      ['token spent too early', () => mockReq({ body: body({ token: crypto.issueChallenge(keys, T0 + 4900) }) }), 425],
      ['expired token', () => mockReq({ body: body({ token: crypto.issueChallenge(keys, T0 - 3_700_000) }) }), 410],
      ['malformed JSON', () => mockReq({ body: '{nope' }), 400],
      ['wrong content type', () => mockReq({ headers: { 'content-type': 'text/plain' }, body: body() }), 415],
      ['declared too large', () => mockReq({ headers: { 'content-length': '9000' }, body: body() }), 413],
      ['streamed too large, no length', () => mockReq({ chunks: [Buffer.alloc(5000, 97), Buffer.alloc(5000, 97)] }), 413],
      ['no Origin', () => mockReq({ headers: { origin: undefined }, body: body() }), 403],
      ['foreign Origin', () => mockReq({ headers: { origin: 'https://evil.example' }, body: body() }), 403],
      ['cross-site fetch', () => mockReq({ headers: { 'sec-fetch-site': 'cross-site' }, body: body() }), 403],
      ['GET', () => mockReq({ method: 'GET' }), 405],
    ];
    for (const [label, makeReq, expected] of cases) {
      const { api, calls } = setup(async () => ({ status: 201, outcome: 'received', id: RECEIPT }));
      const r = await run(api.submit, makeReq());
      assert.equal(r.status, expected, label);
      assert.equal(calls.length, 0, `${label}: reached the database`);
      assert.ok(!(r.raw || '').includes('received'), label);
    }
  });

  test('missing configuration fails closed and logs names only', async () => {
    const logs = [];
    const api = handler.createApi({ env: { ASSESSMENT_DATABASE_URL: 'postgres://u:SECRETPW@h/db' }, log: { error: (...a) => logs.push(a.join(' ')) } });
    const r = await run(api.submit, mockReq({ body: body() }));
    assert.deepEqual([r.status, r.json], [503, { error: 'unavailable' }]);
    assert.match(logs[0], /ASSESSMENT_MASTER_KEY=unset/);
    assert.ok(!logs.join().includes('SECRETPW'));
  });

  test('the client address is only trusted from platform headers, and only on Vercel', async () => {
    const spoofed = { 'x-vercel-forwarded-for': '198.51.100.7' };
    const off = setup(async () => ({ status: 201, outcome: 'received', id: RECEIPT }));
    await run(off.api.submit, mockReq({ headers: spoofed, body: body() }));
    assert.equal(off.calls[0].clientHash, crypto.clientHash(keys, '203.0.113.9'), 'header ignored off-platform');
    const on = setup(async () => ({ status: 201, outcome: 'received', id: RECEIPT }), { VERCEL: '1' });
    await run(on.api.submit, mockReq({ headers: spoofed, body: body() }));
    assert.equal(on.calls[0].clientHash, crypto.clientHash(keys, '198.51.100.7'));
  });

  test('the form token endpoint is stateless, GET-only and origin-checked', async () => {
    const { api, calls } = setup(async () => assert.fail('must not touch the database'));
    const ok = await run(api.challenge, mockReq({ method: 'GET', body: undefined, headers: { 'content-type': undefined } }));
    assert.equal(ok.status, 200);
    assert.equal(crypto.verifyChallenge(keys, ok.json.token, T0 + 8000).ok, true);
    assert.equal((await run(api.challenge, mockReq({ method: 'POST' }))).status, 405);
    assert.equal((await run(api.challenge, mockReq({ method: 'GET', headers: { origin: 'https://evil.example' } }))).status, 403);
    assert.equal((await run(api.challenge, mockReq({ method: 'GET', headers: { origin: undefined, 'sec-fetch-site': 'same-origin' } }))).status, 200);
    assert.equal(calls.length, 0);
  });
});
