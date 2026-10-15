// Shared test helpers. The database tests run against a REAL PostgreSQL (>= 15; Supabase runs
// 15 or 17), given as ASSESSMENT_TEST_DATABASE_URL: a superuser URL to any database on that
// server. Each run creates its own throwaway database, applies the real migration file, and
// drops it afterwards. Nothing here ever touches a Supabase project.
import postgres from 'postgres';
import { readFileSync } from 'node:fs';
import { randomBytes } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
export const lib = {
  crypto: require('../_lib/crypto.js'),
  config: require('../_lib/config.js'),
  validate: require('../_lib/validate.js'),
  http: require('../_lib/http.js'),
  handler: require('../_lib/handler.js'),
};

export const MIGRATION_FILE = fileURLToPath(new URL('../../../../supabase/migrations/20261015000000_assessment_inquiries.sql', import.meta.url));
export const WORKER_FILE = fileURLToPath(new URL('./worker.mjs', import.meta.url));
export const ORIGIN = 'http://localhost:3000';
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** A fresh database with the migration applied, Supabase-like built-in roles, and role passwords for login. */
export async function createTestDatabase(adminUrl) {
  const name = `assessment_test_${randomBytes(4).toString('hex')}`;
  const admin = postgres(adminUrl, { max: 2, onnotice: () => {} });
  // Roles Supabase provides. Created here only so the migration's revokes have something to bite on.
  for (const role of ['anon', 'authenticated', 'service_role']) {
    await admin.unsafe(`do $$ begin if not exists (select from pg_roles where rolname = '${role}') then create role ${role} nologin${role === 'service_role' ? ' bypassrls' : ''}; end if; end $$`);
  }
  await admin.unsafe(`create database ${name} encoding 'UTF8' template template0`);

  const withDb = (url) => { const u = new URL(url); u.pathname = `/${name}`; return u; };
  // A migration carries its own begin/commit, so it needs a dedicated single connection, as a migration runner would use.
  const migrator = postgres(withDb(adminUrl).toString(), { max: 1, onnotice: () => {} });
  await migrator.unsafe(readFileSync(MIGRATION_FILE, 'utf8'));
  await migrator.end({ timeout: 2 });
  const owner = postgres(withDb(adminUrl).toString(), { max: 2, onnotice: () => {} });

  // The migration creates the roles NOLOGIN. Passwords are set out of band, exactly as in production.
  await admin.unsafe(`alter role assessment_writer login password 'writer-pw'`);
  await admin.unsafe(`alter role assessment_operator login password 'operator-pw'`);

  const asRole = (user, password) => { const u = withDb(adminUrl); u.username = user; u.password = password; return u.toString(); };
  const masterKey = randomBytes(32).toString('hex');
  return {
    name,
    owner,
    masterKey,
    keys: lib.crypto.deriveKeys(masterKey),
    writerUrl: asRole('assessment_writer', 'writer-pw'),
    operatorUrl: asRole('assessment_operator', 'operator-pw'),
    adminUrl: withDb(adminUrl).toString(),
    async drop() {
      await owner.end({ timeout: 2 });
      await admin.unsafe(`drop database if exists ${name} with (force)`);
      await admin.end({ timeout: 2 });
    },
  };
}

/** Environment for one API instance, as Vercel would provide it. */
export function instanceEnv(db, extra = {}) {
  return {
    ASSESSMENT_MASTER_KEY: db.masterKey,
    ASSESSMENT_DATABASE_URL: db.writerUrl,
    ASSESSMENT_DATABASE_SSL: 'disable',
    ASSESSMENT_ALLOWED_ORIGINS: ORIGIN,
    VERCEL: '1', // so the client address is read from the platform header, as in production
    ...extra,
  };
}

/** An independent instance: its own OS process, its own module state, its own database connection. */
export function startWorker(env) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [WORKER_FILE], { env: { ...process.env, ...env }, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    child.stdout.on('data', (d) => {
      out += d;
      const m = out.match(/PORT (\d+)/);
      if (m) resolve({ base: `http://127.0.0.1:${m[1]}`, pid: child.pid, stop: () => new Promise((r) => { child.once('exit', r); child.kill(); }) });
    });
    child.stderr.on('data', (d) => { out += d; });
    child.once('exit', (code) => reject(new Error(`worker exited ${code}: ${out}`)));
    setTimeout(() => reject(new Error(`worker did not start: ${out}`)), 15000);
  });
}

let ipCounter = 0;
/** A unique client address per test, so tests do not share a per-client throttle bucket. */
export const freshIp = () => `10.20.${Math.floor(++ipCounter / 250)}.${(ipCounter % 250) + 1}`;

export async function getToken(base, ip = freshIp()) {
  const res = await fetch(`${base}/api/assessment/challenge`, { headers: { Origin: ORIGIN, 'x-vercel-forwarded-for': ip } });
  return (await res.json()).token;
}

export const goodBody = (token, over = {}) => ({
  name: 'Ada Lovelace', email: `ada+${randomBytes(3).toString('hex')}@example.invalid`, organization: 'Analytical Engines', context: 'Agents that open tickets.',
  consent: true, source: 'agent-floor', website: '', token, ...over,
});

export async function post(base, body, { ip = freshIp(), origin = ORIGIN, headers = {}, raw } = {}) {
  const res = await fetch(`${base}/api/assessment`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...(origin ? { Origin: origin } : {}), 'x-vercel-forwarded-for': ip, ...headers },
    body: raw !== undefined ? raw : JSON.stringify(body),
  });
  let json = null;
  try { json = await res.json(); } catch { /* not JSON */ }
  return { status: res.status, json, headers: res.headers };
}

/** Per-client throttle buckets are per minute. Don't start a throttle test right before the minute rolls over. */
export async function waitForSafeMinuteWindow(maxSeconds = 40) {
  const seconds = new Date().getSeconds();
  if (seconds > maxSeconds) await sleep((61 - seconds) * 1000);
}
