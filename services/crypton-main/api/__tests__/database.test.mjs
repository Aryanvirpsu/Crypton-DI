// Integration tests against a REAL PostgreSQL. They apply the real migration file and run the
// real handlers: in-process, and as independent OS processes that share only the database.
//
//   ASSESSMENT_TEST_DATABASE_URL=postgres://postgres:<pw>@127.0.0.1:5432/postgres npm run test:api
//
// Without that variable these tests are SKIPPED (and reported as skipped, not passed).
import test, { describe, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import postgres from 'postgres';
import { startApiServer } from '../../scripts/dev-api.mjs';
import { createTestDatabase, getToken, goodBody, freshIp, instanceEnv, lib, ORIGIN, post, sleep, startWorker, waitForSafeMinuteWindow } from './harness.mjs';

const ADMIN = process.env.ASSESSMENT_TEST_DATABASE_URL;
const TOOL = fileURLToPath(new URL('../../scripts/inbox.mjs', import.meta.url));
const UUID = /^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/;
const MINT_WAIT = 1700; // a token cannot be spent for 1.5 s
const FUNCTION_CALL = `select * from assessment.submit_inquiry(gen_random_uuid(), repeat('a',64), repeat('b',64), repeat('c',64), 'deadbeef', decode(repeat('00',40),'hex'), 'x', 'agent-floor')`;

describe('against a real PostgreSQL', { skip: !ADMIN && 'set ASSESSMENT_TEST_DATABASE_URL to run' }, () => {
  let db;
  let workers = [];
  const url = (i = 0) => workers[i % workers.length].base;
  const count = async (where = 'true') => (await db.owner.unsafe(`select count(*)::int as n from assessment.assessment_inquiries where ${where}`))[0].n;
  const throttleRows = async () => (await db.owner`select coalesce(sum(count), 0)::int as n from assessment.rate_limits`)[0].n;

  before(async () => {
    db = await createTestDatabase(ADMIN);
    workers = await Promise.all([0, 1, 2, 3].map(() => startWorker(instanceEnv(db))));
  });
  after(async () => {
    await Promise.all(workers.map((w) => w.stop()));
    await db.drop();
  });

  describe('persistence', () => {
    test('four independent processes are really independent', () => {
      assert.equal(new Set(workers.map((w) => w.pid)).size, 4);
      assert.equal(new Set(workers.map((w) => w.base)).size, 4);
    });

    test('a valid submission is stored exactly once, encrypted, and readable only with the key', async () => {
      const ip = freshIp();
      const token = await getToken(url(), ip);
      await sleep(MINT_WAIT);
      const body = goodBody(token);
      const r = await post(url(), body, { ip });
      assert.equal(r.status, 201);
      assert.equal(r.json.status, 'received');
      assert.match(r.json.receipt, UUID);

      const rows = await db.owner`select * from assessment.assessment_inquiries where idempotency_key = ${token.split('.')[1]}`;
      assert.equal(rows.length, 1, 'exactly one row');
      const row = rows[0];
      assert.equal(row.id, r.json.receipt, 'the receipt is the stored row');
      assert.equal(row.status, 'new');
      assert.equal(row.consent, true);
      assert.equal(row.source, 'agent-floor');
      assert.equal(row.key_id, db.keys.keyId);
      assert.ok(row.created_at instanceof Date && row.updated_at instanceof Date);

      const raw = Buffer.from(row.payload).toString('latin1');
      for (const secret of [body.name, body.email, body.organization, body.context]) assert.ok(!raw.includes(secret), `plaintext "${secret}" found in the database`);
      const opened = lib.crypto.decryptPayload(db.keys, row.payload, row.idempotency_key);
      assert.equal(opened.name, body.name);
      assert.equal(opened.email, body.email);
      assert.equal(opened.context, body.context);
    });

    test('retrying the same request is idempotent, on this instance and on another', async () => {
      const ip = freshIp();
      const token = await getToken(url(), ip);
      await sleep(MINT_WAIT);
      const body = goodBody(token);
      const first = await post(url(0), body, { ip });
      const again = await post(url(0), body, { ip });
      const elsewhere = await post(url(2), body, { ip });
      assert.deepEqual([first.status, again.status, elsewhere.status], [201, 200, 200]);
      assert.equal(again.json.receipt, first.json.receipt);
      assert.equal(elsewhere.json.receipt, first.json.receipt);
      assert.equal(await count(`idempotency_key = '${token.split('.')[1]}'`), 1);
    });

    test('the same key with a changed request is refused and changes nothing', async () => {
      const ip = freshIp();
      const token = await getToken(url(), ip);
      await sleep(MINT_WAIT);
      const body = goodBody(token);
      assert.equal((await post(url(), body, { ip })).status, 201);
      const altered = await post(url(1), { ...body, name: 'Someone Else' }, { ip });
      assert.deepEqual([altered.status, altered.json], [409, { error: 'changed-retry' }]);
      const [row] = await db.owner`select payload, idempotency_key from assessment.assessment_inquiries where idempotency_key = ${token.split('.')[1]}`;
      assert.equal(lib.crypto.decryptPayload(db.keys, row.payload, row.idempotency_key).name, body.name);
    });

    test('24 simultaneous duplicates across 4 independent instances store exactly one row', async () => {
      const ip = freshIp();
      const token = await getToken(url(), ip);
      await sleep(MINT_WAIT);
      const body = goodBody(token);
      const results = await Promise.all(Array.from({ length: 24 }, (_, i) => post(url(i), body, { ip })));
      const statuses = results.map((r) => r.status).sort();
      assert.equal(statuses.filter((s) => s === 201).length, 1, `statuses: ${statuses}`);
      assert.equal(statuses.filter((s) => s === 200).length, 23, `statuses: ${statuses}`);
      assert.equal(new Set(results.map((r) => r.json.receipt)).size, 1, 'everyone is told the same receipt');
      assert.equal(await count(`idempotency_key = '${token.split('.')[1]}'`), 1);
    });
  });

  describe('invalid and abusive requests', () => {
    test('are rejected before they cost a database call, and store nothing', async () => {
      const ip = freshIp();
      const token = await getToken(url(), ip);
      const early = await post(url(), goodBody(token), { ip }); // spent immediately: too fast
      await sleep(MINT_WAIT);
      const rowsBefore = await count();
      const throttleBefore = await throttleRows();

      const cases = [
        ['too fast', early.status, 425],
        ['invalid email', (await post(url(), goodBody(token, { email: 'nope' }), { ip })).status, 422],
        ['no consent', (await post(url(), goodBody(token, { consent: false }), { ip })).status, 422],
        ['name too short', (await post(url(), goodBody(token, { name: 'A' }), { ip })).status, 422],
        ['control characters', (await post(url(), goodBody(token, { context: 'a\u0007b' }), { ip })).status, 422],
        ['honeypot filled', (await post(url(), goodBody(token, { website: 'x' }), { ip })).status, 400],
        ['wrong source', (await post(url(), goodBody(token, { source: 'other' }), { ip })).status, 400],
        ['forged token', (await post(url(), goodBody(`${token.slice(0, -4)}0000`), { ip })).status, 400],
        ['no token', (await post(url(), goodBody(undefined), { ip })).status, 400],
        ['malformed JSON', (await post(url(), null, { ip, raw: '{broken' })).status, 400],
        ['not an object', (await post(url(), null, { ip, raw: '[1,2]' })).status, 422],
        ['wrong content type', (await post(url(), null, { ip, raw: 'x', headers: { 'Content-Type': 'text/plain' } })).status, 415],
        ['oversized', (await post(url(), null, { ip, raw: JSON.stringify({ context: 'x'.repeat(9000) }) })).status, 413],
        ['foreign origin', (await post(url(), goodBody(token), { ip, origin: 'https://evil.example' })).status, 403],
        ['no origin', (await post(url(), goodBody(token), { ip, origin: null })).status, 403],
      ];
      for (const [label, got, want] of cases) assert.equal(got, want, label);

      assert.equal(await count(), rowsBefore, 'nothing was stored');
      assert.equal(await throttleRows(), throttleBefore, 'the database function was never called');
    });

    test('GET on the submit route and POST on the token route are refused', async () => {
      assert.equal((await fetch(`${url()}/api/assessment`, { headers: { Origin: ORIGIN } })).status, 405);
      assert.equal((await fetch(`${url()}/api/assessment/challenge`, { method: 'POST', headers: { Origin: ORIGIN } })).status, 405);
    });
  });

  describe('throttling is shared, atomic and survives concurrency', () => {
    test('per email: 3 an hour, strictly, even when sent at the same instant from different instances', async () => {
      const email = `limit+${Date.now()}@example.invalid`;
      const tokens = await Promise.all(Array.from({ length: 8 }, () => getToken(url())));
      await sleep(MINT_WAIT);
      const results = await Promise.all(tokens.map((t, i) => post(url(i), goodBody(t, { email }))));
      const statuses = results.map((r) => r.status).sort();
      assert.deepEqual(statuses, [201, 201, 201, 429, 429, 429, 429, 429], `statuses: ${statuses}`);
      assert.equal(await count(`email_hash = '${lib.crypto.emailHash(db.keys, email)}'`), 3, 'never more than 3 stored');
      const limited = results.find((r) => r.status === 429);
      assert.equal(limited.json.error, 'rate-limit');
      assert.equal(limited.headers.get('retry-after'), '60');
    });

    test('per client: 60 a minute, shared by every instance', async () => {
      await waitForSafeMinuteWindow();
      const ip = freshIp();
      const token = await getToken(url(), ip);
      await sleep(MINT_WAIT);
      const body = goodBody(token);
      const results = await Promise.all(Array.from({ length: 65 }, (_, i) => post(url(i), body, { ip })));
      const limited = results.filter((r) => r.status === 429).length;
      const allowed = results.filter((r) => r.status === 200 || r.status === 201).length;
      assert.deepEqual([allowed, limited], [60, 5]);
      const buckets = await db.owner`select count, expires_at - now() as ttl from assessment.rate_limits where bucket like ${`ip:${lib.crypto.clientHash(db.keys, ip)}:%`}`;
      assert.equal(buckets.length, 1);
      assert.equal(buckets[0].count, 65);
    });
  });


  describe('races that real traffic cannot reliably produce, forced deterministically', () => {
    // Two database sessions as the web tier's own role. Session A keeps its transaction OPEN, so a
    // request from session B overlaps it exactly, which is what concurrent serverless instances do.
    const call = (sql, { key = randomUUID(), fingerprint = 'a'.repeat(64), emailHash, ip }) => sql`
      select http_status, outcome, inquiry_id from assessment.submit_inquiry(
        ${key}::uuid, ${fingerprint}, ${emailHash}, ${lib.crypto.clientHash(db.keys, ip)}, 'deadbeef', ${Buffer.alloc(40, 1)}::bytea, 'x', 'agent-floor')`;
    // These rows carry throwaway payloads (not real ciphertext); remove them so no later test reads them.
    after(() => db.owner.unsafe(`delete from assessment.assessment_inquiries where payload = decode(repeat('01',40),'hex')`));
    const sessions = () => [postgres(db.writerUrl, { max: 1, onnotice: () => {} }), postgres(db.writerUrl, { max: 1, onnotice: () => {} })];

    test('per email: with two requests overlapping exactly, the second waits and the limit still holds', async () => {
      const emailHash = lib.crypto.emailHash(db.keys, `overlap+${Date.now()}@example.invalid`);
      for (let i = 0; i < 2; i += 1) await call(db.owner, { emailHash, ip: freshIp() }); // two already stored (as the owner, bypassing the throttle only for set-up)
      assert.equal(await count(`email_hash = '${emailHash}'`), 2);
      const [A, B] = sessions();
      try {
        let waiting = true;
        const result = await A.begin(async (tx) => {
          const [third] = await call(tx, { emailHash, ip: freshIp() });
          const fourth = call(B, { emailHash, ip: freshIp() }).then((r) => { waiting = false; return r[0]; });
          await sleep(600);
          assert.equal(waiting, true, 'the overlapping request must wait for the first one to finish');
          return { third: third.http_status, fourth };
        });
        const fourth = await result.fourth;
        assert.deepEqual([result.third, fourth.http_status], [201, 429], 'the 4th request in the hour is refused');
        assert.equal(await count(`email_hash = '${emailHash}'`), 3, 'never more than 3');
      } finally { await Promise.all([A.end({ timeout: 2 }), B.end({ timeout: 2 })]); }
    });

    test('same idempotency key, overlapping exactly: the loser gets the winner\'s receipt (or a 409 if its request differs)', async () => {
      const key = randomUUID();
      const [A, B] = sessions();
      const C = postgres(db.writerUrl, { max: 1, onnotice: () => {} });
      try {
        let bDone = false;
        // Different email addresses, so the per-email lock does not mask the key conflict.
        const out = await A.begin(async (tx) => {
          const [first] = await call(tx, { key, emailHash: lib.crypto.emailHash(db.keys, `race-a+${key}@example.invalid`), ip: freshIp() });
          const same = call(B, { key, emailHash: lib.crypto.emailHash(db.keys, `race-b+${key}@example.invalid`), ip: freshIp() }).then((r) => { bDone = true; return r[0]; });
          const different = call(C, { key, fingerprint: 'b'.repeat(64), emailHash: lib.crypto.emailHash(db.keys, `race-c+${key}@example.invalid`), ip: freshIp() }).then((r) => r[0]);
          await sleep(600);
          assert.equal(bDone, false, 'the loser must wait on the unique key, not run ahead');
          return { first, same, different };
        });
        const [same, different] = [await out.same, await out.different];
        assert.equal(out.first.http_status, 201);
        assert.deepEqual([same.http_status, same.inquiry_id], [200, out.first.inquiry_id]);
        assert.equal(different.http_status, 409);
        assert.equal(await count(`idempotency_key = '${key}'`), 1);
      } finally { await Promise.all([A.end({ timeout: 2 }), B.end({ timeout: 2 }), C.end({ timeout: 2 })]); }
    });
  });

  describe('when the database is not available', () => {
    const outcomes = async (env) => {
      const server = await startApiServer({ env: instanceEnv(db, env), port: 0 });
      try {
        const base = `http://127.0.0.1:${server.address().port}`;
        const ip = freshIp();
        const token = await getToken(base, ip);
        await sleep(MINT_WAIT);
        return await post(base, goodBody(token), { ip });
      } finally { await new Promise((r) => server.close(r)); }
    };
    const rowCount = () => count();

    test('nothing listening: 503, and never "received"', async () => {
      const n = await rowCount();
      const r = await outcomes({ ASSESSMENT_DATABASE_URL: 'postgres://assessment_writer:writer-pw@127.0.0.1:1/postgres' });
      assert.deepEqual([r.status, r.json], [503, { error: 'unavailable' }]);
      assert.equal(await count(), n);
    });

    test('wrong credentials: 503 and no leak of why', async () => {
      const r = await outcomes({ ASSESSMENT_DATABASE_URL: db.writerUrl.replace('writer-pw', 'wrong-password') });
      assert.deepEqual([r.status, r.json], [503, { error: 'unavailable' }]);
    });

    test('database reachable but not migrated: 503', async () => {
      const u = new URL(db.writerUrl); u.pathname = '/postgres';
      const r = await outcomes({ ASSESSMENT_DATABASE_URL: u.toString() });
      assert.deepEqual([r.status, r.json], [503, { error: 'unavailable' }]);
    });

    test('incomplete configuration: 503', async () => {
      const r = await outcomes({ ASSESSMENT_MASTER_KEY: 'short' });
      assert.deepEqual([r.status, r.json], [503, { error: 'unavailable' }]);
    });
  });

  describe('anonymous callers cannot reach leads', () => {
    // Runs one statement as a Supabase built-in role (via SET ROLE, as a superuser may) and reports what happened.
    const as = async (role, statement) => {
      try {
        return await db.owner.begin(async (tx) => { await tx.unsafe(`set local role ${role}`); return { rows: await tx.unsafe(statement) }; });
      } catch (error) { return { error: error.code, message: error.message }; }
    };

    for (const role of ['anon', 'authenticated', 'service_role']) {
      test(`${role}: every read, write and call is denied`, async () => {
        assert.ok((await count()) > 0, 'there are leads to protect');
        for (const statement of [
          'select * from assessment.assessment_inquiries',
          'select count(*) from assessment.assessment_inquiries',
          'select * from assessment.rate_limits',
          `insert into assessment.assessment_inquiries (idempotency_key, request_fingerprint, payload, key_id, email_hash, consent, privacy_version, source) values (gen_random_uuid(), repeat('a',64), decode(repeat('00',40),'hex'), 'deadbeef', repeat('b',64), true, 'x', 'agent-floor')`,
          `update assessment.assessment_inquiries set status = 'spam'`,
          'delete from assessment.assessment_inquiries',
          'truncate assessment.assessment_inquiries',
          FUNCTION_CALL,
          'select assessment.prune_expired()',
        ]) {
          const r = await as(role, statement);
          assert.equal(r.error, '42501', `${role} was NOT denied: ${statement.slice(0, 60)} -> ${r.error || 'allowed'}`);
        }
      });
    }

    test('row level security denies on its own, even if a grant were ever added by mistake', async () => {
      const outcome = await db.owner.begin(async (tx) => {
        await tx.unsafe('grant usage on schema assessment to anon, authenticated');
        await tx.unsafe('grant select, update, delete on assessment.assessment_inquiries to anon, authenticated');
        const seen = {};
        for (const role of ['anon', 'authenticated']) {
          await tx.unsafe(`set local role ${role}`);
          seen[role] = {
            visible: (await tx.unsafe('select count(*)::int as n from assessment.assessment_inquiries'))[0].n,
            updated: (await tx.unsafe(`update assessment.assessment_inquiries set status = 'spam' returning id`)).length,
            deleted: (await tx.unsafe('delete from assessment.assessment_inquiries returning id')).length,
          };
          await tx.unsafe('reset role');
        }
        seen.real = (await tx.unsafe('select count(*)::int as n from assessment.assessment_inquiries'))[0].n;
        throw Object.assign(new Error('rollback'), { seen });
      }).catch((e) => e.seen);
      assert.ok(outcome.real > 0);
      for (const role of ['anon', 'authenticated']) assert.deepEqual(outcome[role], { visible: 0, updated: 0, deleted: 0 }, role);
    });

    test('privilege matrix', async () => {
      const q = async (sql, ...v) => (await db.owner.unsafe(sql, v))[0].ok;
      const fn = 'assessment.submit_inquiry(uuid,text,text,text,text,bytea,text,text)';
      for (const role of ['anon', 'authenticated', 'service_role']) {
        assert.equal(await q(`select has_schema_privilege('${role}', 'assessment', 'usage') as ok`), false, `${role} schema usage`);
        for (const t of ['assessment_inquiries', 'rate_limits']) for (const p of ['select', 'insert', 'update', 'delete', 'truncate']) {
          assert.equal(await q(`select has_table_privilege('${role}', 'assessment.${t}', '${p}') as ok`), false, `${role} ${p} ${t}`);
        }
        assert.equal(await q(`select has_function_privilege('${role}', '${fn}', 'execute') as ok`), false, `${role} execute`);
      }
      assert.equal(await q(`select has_function_privilege('public', '${fn}', 'execute') as ok`), false, 'public may not execute');
      assert.equal(await q(`select has_function_privilege('assessment_writer', '${fn}', 'execute') as ok`), true);
      assert.equal(await q(`select relrowsecurity as ok from pg_class where oid = 'assessment.assessment_inquiries'::regclass`), true);
      assert.equal(await q(`select relrowsecurity as ok from pg_class where oid = 'assessment.rate_limits'::regclass`), true);
    });
  });

  describe('least privilege for the roles that do have access', () => {
    test('the web tier can call the one function and nothing else', async () => {
      const sql = postgres(db.writerUrl, { max: 1, onnotice: () => {} });
      try {
        assert.equal((await sql`show statement_timeout`)[0].statement_timeout, '5s');
        for (const statement of [
          'select * from assessment.assessment_inquiries', 'select * from assessment.rate_limits', 'select assessment.prune_expired()',
          `update assessment.assessment_inquiries set status = 'spam'`, 'delete from assessment.rate_limits',
          `insert into assessment.rate_limits values ('x', 1, now())`,
        ]) {
          await assert.rejects(sql.unsafe(statement), (e) => e.code === '42501', statement);
        }
      } finally { await sql.end({ timeout: 2 }); }
    });

    test('the operator can read leads and change their status, and nothing more', async () => {
      const sql = postgres(db.operatorUrl, { max: 1, onnotice: () => {} });
      try {
        const [lead] = await sql`select id, created_at from assessment.assessment_inquiries order by created_at limit 1`;
        assert.ok((await sql`select count(*)::int as n from assessment.assessment_inquiries`)[0].n > 0, 'policy lets the operator read');
        await sleep(20);
        const [moved] = await sql`update assessment.assessment_inquiries set status = 'contacted' where id = ${lead.id} returning status, updated_at, created_at`;
        assert.equal(moved.status, 'contacted');
        assert.ok(moved.updated_at > moved.created_at, 'updated_at moves on change');
        for (const statement of [
          `update assessment.assessment_inquiries set payload = decode(repeat('00',40),'hex')`,
          `update assessment.assessment_inquiries set email_hash = repeat('f',64)`,
          'delete from assessment.assessment_inquiries', 'truncate assessment.assessment_inquiries',
          'select * from assessment.rate_limits', 'select assessment.prune_expired()', FUNCTION_CALL,
        ]) {
          await assert.rejects(sql.unsafe(statement), (e) => e.code === '42501', statement);
        }
        await assert.rejects(sql`update assessment.assessment_inquiries set status = 'bogus' where id = ${lead.id}`, (e) => e.code === '23514');
      } finally { await sql.end({ timeout: 2 }); }
    });

    test('the operator tool decrypts on the operator\'s machine, and cannot with the wrong key', () => {
      const env = { ...process.env, ASSESSMENT_OPERATOR_DATABASE_URL: db.operatorUrl, ASSESSMENT_DATABASE_SSL: 'disable', ASSESSMENT_MASTER_KEY: db.masterKey };
      const ok = spawnSync(process.execPath, [TOOL, 'list', '--all'], { env, encoding: 'utf8' });
      assert.equal(ok.status, 0, ok.stderr);
      const leads = JSON.parse(ok.stdout);
      assert.ok(leads.length > 0 && leads.every((l) => l.email && l.name), 'every lead decrypts');
      const wrong = spawnSync(process.execPath, [TOOL, 'list', '--all'], { env: { ...env, ASSESSMENT_MASTER_KEY: 'ab'.repeat(32) }, encoding: 'utf8' });
      assert.match(wrong.stdout, /no key for key_id/);
      assert.ok(!/@example\.invalid/.test(wrong.stdout), 'no plaintext without the key');
    });
  });

  describe('integrity and housekeeping', () => {
    test('the table refuses malformed rows even from its owner', async () => {
      const row = (over = {}) => ({ idempotency_key: 'gen_random_uuid()', request_fingerprint: `repeat('a',64)`, payload: `decode(repeat('00',40),'hex')`, key_id: `'deadbeef'`, email_hash: `repeat('b',64)`, consent: 'true', privacy_version: `'x'`, source: `'agent-floor'`, ...over });
      const insert = (over) => { const r = row(over); return db.owner.unsafe(`insert into assessment.assessment_inquiries (${Object.keys(r).join(',')}) values (${Object.values(r).join(',')})`); };
      await assert.rejects(insert({ consent: 'false' }), (e) => e.code === '23514', 'consent must be true');
      await assert.rejects(insert({ source: `'other'` }), (e) => e.code === '23514', 'source');
      await assert.rejects(insert({ payload: `decode('00','hex')` }), (e) => e.code === '23514', 'payload too small to be an envelope');
      await assert.rejects(insert({ key_id: `'NOT-HEX!'` }), (e) => e.code === '23514', 'key id format');
      await assert.rejects(insert({ email_hash: `'plain@example.invalid'` }), (e) => e.code === '23514', 'an email address cannot be stored as the hash');
      await assert.rejects(insert({ status: `'bogus'` }), (e) => e.code === '23514', 'status');
    });

    test('retention: requests older than 90 days and expired throttle rows are removed; recent ones stay', async () => {
      await db.owner.unsafe(`insert into assessment.assessment_inquiries (idempotency_key, request_fingerprint, payload, key_id, email_hash, consent, privacy_version, source, created_at)
        values (gen_random_uuid(), repeat('a',64), decode(repeat('00',40),'hex'), 'deadbeef', repeat('c',64), true, 'x', 'agent-floor', now() - interval '100 days')`);
      await db.owner.unsafe(`insert into assessment.rate_limits values ('expired-bucket', 3, now() - interval '1 hour')`);
      const recent = await count(`created_at > now() - interval '1 day'`);
      const [{ prune_expired: removed }] = await db.owner`select assessment.prune_expired() as prune_expired`;
      assert.ok(removed >= 1);
      assert.equal(await count(`email_hash = repeat('c',64)`), 0, 'old request removed');
      assert.equal(await count(`created_at > now() - interval '1 day'`), recent, 'recent requests kept');
      assert.equal((await db.owner`select count(*)::int as n from assessment.rate_limits where bucket = 'expired-bucket'`)[0].n, 0);
    });
  });
});

