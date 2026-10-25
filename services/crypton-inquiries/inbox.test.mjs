import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { randomBytes } from 'node:crypto';
import { openInbox, settings } from './store.mjs';
import { createInquiryServer } from './server.mjs';

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'crypton-inbox-'));
  const dbPath = join(dir, 'inquiries.sqlite');
  const key = randomBytes(32);
  let time = 1792000000000;
  let inbox = openInbox({ dbPath, key, now: () => time });
  t.after(() => { inbox.close(); rmSync(dir, { recursive: true }); });
  const input = (patch = {}) => {
    const token = inbox.challenge(); time += 2000;
    return { name: 'Verification Only', email: 'verification@example.invalid', organization: '', context: '', consent: true, source: 'agent-floor', website: '', token, ...patch };
  };
  return { key, get inbox() { return inbox; }, input, dbPath, advance: (ms) => { time += ms; }, reopen: () => { inbox.close(); inbox = openInbox({ dbPath, key, now: () => time }); } };
}

test('delivery is committed, encrypted at rest, readable privately, and durable after restart', (t) => {
  const f = fixture(t); const input = f.input(); const result = f.inbox.accept(input);
  assert.equal(result.code, 201); assert.equal(result.status, 'received');
  assert.equal(f.inbox.list()[0].receipt, result.receipt);
  for (const file of [f.dbPath, `${f.dbPath}-wal`]) assert.equal(readFileSync(file).includes(Buffer.from(input.email)), false);
  f.reopen(); assert.equal(f.inbox.list()[0].email, input.email);
});
test('identical retries return the same receipt and never insert twice', (t) => {
  const f = fixture(t); const input = f.input(); const first = f.inbox.accept(input); const retry = f.inbox.accept(input);
  assert.equal(retry.code, 200); assert.equal(retry.receipt, first.receipt); assert.equal(f.inbox.list().length, 1);
  assert.equal(f.inbox.accept({ ...input, context: 'changed' }).code, 409);
});
for (const [field, value] of [['name', 'A'], ['name', 4], ['email', 'bad'], ['email', 'a'.repeat(255)], ['context', 'x'.repeat(1001)], ['organization', {}], ['consent', 'true'], ['consent', false], ['context', '\u0000']]) {
  test(`server validation rejects ${field} ${JSON.stringify(value).slice(0, 35)}`, (t) => {
    const f = fixture(t); assert.equal(f.inbox.accept(f.input({ [field]: value })).code, 422); assert.equal(f.inbox.list().length, 0);
  });
}
test('signed tokens reject tampering, expiry, immediate sends, and honeypots without fake success', (t) => {
  const f = fixture(t);
  assert.equal(f.inbox.accept(f.input({ website: 'spam' })).code, 400);
  assert.equal(f.inbox.accept(f.input({ token: 'forged' })).code, 400);
  const input = f.input(); assert.equal(f.inbox.accept({ ...input, token: f.inbox.challenge() }).code, 425);
  f.advance(3600001); assert.equal(f.inbox.accept(input).code, 410); assert.equal(f.inbox.list().length, 0);
});
test('email rate limit survives restart and expires', (t) => {
  const f = fixture(t); for (let i=0;i<3;i++) assert.equal(f.inbox.accept(f.input()).code, 201);
  f.reopen(); assert.equal(f.inbox.accept(f.input()).code, 429);
  f.advance(3600001); assert.equal(f.inbox.accept(f.input()).code, 201);
});
test('IP attempt throttling ignores payload and persists', (t) => {
  const f = fixture(t); for (let i=0;i<60;i++) assert.equal(f.inbox.rate('127.0.0.1'), true);
  f.reopen(); assert.equal(f.inbox.rate('127.0.0.1'), false); f.advance(61000); assert.equal(f.inbox.rate('127.0.0.1'), true);
});
test('requests older than 90 days leave the active inbox', (t) => {
  const f = fixture(t); f.inbox.accept(f.input()); f.advance(91*86400000); f.reopen(); assert.equal(f.inbox.list().length, 0);
});
test('production configuration fails closed without secrets and HTTPS', () => {
  assert.throws(() => settings({ NODE_ENV: 'production' }), /requires/);
  const dir = mkdtempSync(join(tmpdir(), 'crypton-settings-'));
  try { assert.throws(() => settings({ NODE_ENV: 'production', INQUIRY_KEY: 'a'.repeat(64), INQUIRY_DB: join(dir, 'db'), INQUIRY_ORIGIN: 'http://example.com' }), /HTTPS/); }
  finally { rmSync(dir, { recursive: true }); }
});
test('real HTTP: challenge → commit → receipt; origin, media type, size and read access fail closed', async (t) => {
  const f = fixture(t); const origin = 'http://localhost:3000';
  const server = createInquiryServer({ inbox: f.inbox, origin });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise((resolve) => server.close(resolve)));
  const base = `http://127.0.0.1:${server.address().port}`;
  const headers = { Origin: origin, 'Content-Type': 'application/json' };
  let response = await fetch(`${base}/api/assessment/challenge`, { headers });
  assert.equal(response.status, 200); const { token } = await response.json(); f.advance(2000);
  const input = { ...f.input(), token };
  response = await fetch(`${base}/api/assessment`, { method: 'POST', headers, body: JSON.stringify(input) });
  assert.equal(response.status, 201); assert.equal((await response.json()).receipt, f.inbox.list()[0].receipt);
  assert.equal((await fetch(`${base}/api/assessment`, { headers })).status, 405);
  assert.equal((await fetch(`${base}/api/assessment`, { method:'POST', headers: { ...headers, Origin:'https://evil.example' }, body: '{}' })).status, 403);
  assert.equal((await fetch(`${base}/api/assessment/challenge`)).status, 403);
  assert.equal((await fetch(`${base}/api/assessment`, { method:'POST', headers: { Origin: origin, 'Content-Type':'text/plain' }, body:'{}' })).status, 415);
  assert.equal((await fetch(`${base}/api/assessment`, { method:'POST', headers, body:'x'.repeat(8193) })).status, 413);
  assert.equal((await fetch(`${base}/api/assessment`, { method:'POST', headers, body:'{' })).status, 400);
  assert.equal((await fetch(`${base}/api/inbox`, { headers })).status, 404);
  assert.equal((await fetch(`${base}/.private/inquiries.sqlite`)).status, 404);
  assert.equal(f.inbox.list().length, 1);
});
test('storage failure produces 503 and never received', async (t) => {
  const server = createInquiryServer({ inbox: { rate: () => true, accept: () => { throw new Error('disk full'); } }, origin: 'http://localhost:3000' });
  await new Promise((r) => server.listen(0, '127.0.0.1', r)); t.after(() => new Promise((r) => server.close(r)));
  const res = await fetch(`http://127.0.0.1:${server.address().port}/api/assessment`, { method:'POST', headers: { Origin:'http://localhost:3000', 'Content-Type':'application/json' }, body:'{}' });
  assert.equal(res.status, 503); assert.deepEqual(await res.json(), { error:'unavailable' });
});

test('a wrong encryption key cannot silently open an existing populated inbox', (t) => {
  const f = fixture(t); f.inbox.accept(f.input());
  assert.throws(() => openInbox({ dbPath: f.dbPath, key: randomBytes(32), now: () => 1792000000000 }), /key does not match/);
  assert.equal(f.inbox.list().length, 1);
});

test('optional production static hosting serves both entry routes and never falls back for APIs/private paths', async (t) => {
  const f=fixture(t), buildDir=join(dirname(f.dbPath), 'build'); mkdirSync(buildDir);
  writeFileSync(join(buildDir, 'index.html'), '<!doctype html><title>Route verification</title>');
  const server=createInquiryServer({inbox:f.inbox,origin:'http://localhost:3000',buildDir});
  await new Promise(r=>server.listen(0,'127.0.0.1',r)); t.after(()=>new Promise(r=>server.close(r)));
  const base=`http://127.0.0.1:${server.address().port}`;
  for (const path of ['/', '/agent-floor']) { const res=await fetch(base+path); assert.equal(res.status,200); assert.match(res.headers.get('content-type'), /text\/html/); }
  for (const path of ['/api/missing','/.private/inquiries.sqlite','/missing.js','/%2e%2e/inquiries.sqlite']) assert.equal((await fetch(base+path)).status,404);
});
