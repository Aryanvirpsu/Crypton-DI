// Runtime verification against the REAL stack (infra/docker-compose.stack.yml):
// the real crypton-id and crypton-gateway binaries, real Postgres and Redis, in
// containers. This is an integration test: no source files are stubbed or
// compiled into the test. WebAuthn is driven by a software authenticator below
// (a test client; the server verifies real attestation/assertion signatures).
//
//   tests/runtime/run.sh            # builds, starts, runs this, tears down
//
// Environment: GATEWAY_HOST_PORT (default 18090), COMPOSE_PROJECT (crypton-stack).

import crypto from 'node:crypto';
import net from 'node:net';
import { execFileSync, spawnSync } from 'node:child_process';

const PORT = process.env.GATEWAY_HOST_PORT || '18090';
const GW = `http://127.0.0.1:${PORT}`;
const PROJECT = process.env.COMPOSE_PROJECT || 'crypton-stack';
const COMPOSE_FILE = 'infra/docker-compose.stack.yml';
const RP_ID = 'localhost';
const ORIGIN = 'http://localhost:3000';

// ── tiny assertion framework ─────────────────────────────────────────────────
const results = [];
let section = '';
function group(name) { section = name; console.log(`\n== ${name}`); }
function check(name, ok, detail = '') {
  results.push({ section, name, ok: !!ok, detail });
  console.log(`${ok ? '  PASS' : '  FAIL'}  ${name}${ok || !detail ? '' : `  -> ${detail}`}`);
}
const eq = (name, got, want) => check(name, JSON.stringify(got) === JSON.stringify(want), `got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);

// ── docker helpers ───────────────────────────────────────────────────────────
function docker(args, opts = {}) {
  return execFileSync('docker', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
}
function compose(args) {
  return docker(['compose', '-p', PROJECT, '-f', COMPOSE_FILE, ...args]);
}
const cname = (svc) => `${PROJECT}-${svc}-1`;
const psql = (sql) => docker(['exec', cname('postgres'), 'psql', '-U', 'crypton', '-d', 'crypton', '-tAc', sql]);
const redis = (...args) => docker(['exec', cname('redis'), 'redis-cli', ...args]);

// ── http through the gateway ─────────────────────────────────────────────────
let reqCount = 0;
// crypton-id limits /auth/* per client IP with tower_governor (burst 20, then one
// request per 5 s), and the gateway limits sensitive paths per IP too. The gateway
// trusts X-Forwarded-For only from a private/loopback peer (here: the docker
// bridge), so each scenario presents its own client address. This exercises those
// limiters' per-client keying rather than starving later scenarios of one budget.
let clientSeq = 10;
let currentIp = '198.51.100.10';
function newClient() { currentIp = `198.51.100.${++clientSeq}`; }
async function http(method, path, { token, body, nonce, ts, headers = {}, base = GW, requestId } = {}) {
  const h = { 'x-forwarded-for': currentIp, ...headers };
  if (token) h.authorization = `Bearer ${token}`;
  if (nonce !== null) h['x-nonce'] = nonce || crypto.randomUUID();
  if (ts !== null) h['x-timestamp'] = String(ts ?? Math.floor(Date.now() / 1000));
  h['x-request-id'] = requestId || `e2e-${process.pid}-${++reqCount}-${crypto.randomUUID().slice(0, 8)}`;
  if (body !== undefined) h['content-type'] = 'application/json';
  let res;
  try {
    res = await fetch(base + path, { method, headers: h, body: body === undefined ? undefined : JSON.stringify(body), redirect: 'manual', signal: AbortSignal.timeout(20000) });
  } catch (e) {
    return { status: 0, timeout: true, text: String(e), json: undefined, requestId: h['x-request-id'] };
  }
  const text = await res.text();
  let json; try { json = JSON.parse(text); } catch { json = undefined; }
  return { status: res.status, json, text, requestId: h['x-request-id'] };
}

// ── CBOR (just enough for WebAuthn) ──────────────────────────────────────────
function head(major, n) {
  if (n < 24) return Buffer.from([(major << 5) | n]);
  if (n < 256) return Buffer.from([(major << 5) | 24, n]);
  const b = Buffer.alloc(3); b[0] = (major << 5) | 25; b.writeUInt16BE(n, 1); return b;
}
function cbor(v) {
  if (Buffer.isBuffer(v)) return Buffer.concat([head(2, v.length), v]);
  if (typeof v === 'string') { const b = Buffer.from(v); return Buffer.concat([head(3, b.length), b]); }
  if (typeof v === 'number') return v >= 0 ? head(0, v) : head(1, -1 - v);
  if (v instanceof Map) return Buffer.concat([head(5, v.size), ...[...v].flatMap(([k, x]) => [cbor(k), cbor(x)])]);
  throw new Error('cbor: unsupported');
}
const b64u = (b) => Buffer.from(b).toString('base64url');
const sha256 = (b) => crypto.createHash('sha256').update(b).digest();

// ── software authenticator (ES256, "none" attestation) ───────────────────────
class SoftAuthenticator {
  constructor() {
    this.credId = crypto.randomBytes(32);
    this.key = crypto.generateKeyPairSync('ec', { namedCurve: 'P-256' });
    this.counter = 0;
  }
  create(publicKey) {
    const jwk = this.key.publicKey.export({ format: 'jwk' });
    const cose = new Map([[1, 2], [3, -7], [-1, 1], [-2, Buffer.from(jwk.x, 'base64url')], [-3, Buffer.from(jwk.y, 'base64url')]]);
    const len = Buffer.alloc(2); len.writeUInt16BE(this.credId.length);
    const authData = Buffer.concat([sha256(RP_ID), Buffer.from([0x45]), Buffer.alloc(4), Buffer.alloc(16), len, this.credId, cbor(cose)]);
    const clientData = Buffer.from(JSON.stringify({ type: 'webauthn.create', challenge: publicKey.challenge, origin: ORIGIN, crossOrigin: false }));
    const attObj = cbor(new Map([['fmt', 'none'], ['attStmt', new Map()], ['authData', authData]]));
    return { id: b64u(this.credId), rawId: b64u(this.credId), type: 'public-key', extensions: {},
      response: { attestationObject: b64u(attObj), clientDataJSON: b64u(clientData), transports: ['internal'] } };
  }
  get(publicKey, { corrupt = false } = {}) {
    this.counter += 1;
    const ctr = Buffer.alloc(4); ctr.writeUInt32BE(this.counter);
    const authData = Buffer.concat([sha256(RP_ID), Buffer.from([0x05]), ctr]);
    const clientData = Buffer.from(JSON.stringify({ type: 'webauthn.get', challenge: publicKey.challenge, origin: ORIGIN, crossOrigin: false }));
    let sig = crypto.sign('sha256', Buffer.concat([authData, sha256(clientData)]), this.key.privateKey);
    if (corrupt) { sig = Buffer.from(sig); sig[sig.length - 1] ^= 0xff; }
    return { id: b64u(this.credId), rawId: b64u(this.credId), type: 'public-key', extensions: {},
      response: { authenticatorData: b64u(authData), clientDataJSON: b64u(clientData), signature: b64u(sig) } };
  }
}

// ── flows through the gateway ────────────────────────────────────────────────
const claimsOf = (t) => JSON.parse(Buffer.from(t.split('.')[1], 'base64url').toString());
const headerOf = (t) => JSON.parse(Buffer.from(t.split('.')[0], 'base64url').toString());

async function register(username, auth = new SoftAuthenticator()) {
  const s = await http('POST', '/auth/register/start', { body: { username } });
  if (s.status !== 200) { console.log('  (register/start failed)', s.status, s.text.slice(0, 160)); return { fail: s }; }
  const f = await http('POST', '/auth/register/finish', { body: { challenge_id: s.json.data.challenge_id, attestation: auth.create(s.json.data.publicKey) } });
  if (f.status !== 200) { console.log('  (register/finish failed)', f.status, f.text.slice(0, 160)); return { fail: f }; }
  return { auth, token: f.json.data.token, userId: f.json.data.user_id, username, start: s, finish: f };
}
async function loginStart(username) { return http('POST', '/auth/login/start', { body: { username } }); }
async function login(username, auth, opts = {}) {
  const s = await loginStart(username);
  if (s.status !== 200) return { fail: s };
  const f = await http('POST', '/auth/login/finish', { body: { challenge_id: s.json.data.challenge_id, assertion: auth.get(s.json.data.publicKey, opts) } });
  if (!opts.corrupt && f.status !== 200) console.log('  (login/finish failed)', f.status, f.text.slice(0, 160));
  return { start: s, finish: f, token: f.json?.data?.token };
}

// The gateway limits /auth/* to 100 requests/min per client IP, and every request
// in this run comes from one docker-bridge address. Clear those counters between
// phases so earlier phases cannot starve later ones (the limiter itself is
// covered by the gateway router tests).
function resetRateLimits() {
  const keys = redis('--scan', '--pattern', 'ratelimit:*').split(String.fromCharCode(10)).map((k) => k.trim()).filter(Boolean);
  if (keys.length) redis('DEL', ...keys);
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function waitFor(fn, what, tries = 60, ms = 1000) {
  for (let i = 0; i < tries; i++) { try { if (await fn()) return true; } catch { /* retry */ } await sleep(ms); }
  throw new Error(`timed out waiting for ${what}`);
}
const portOpen = (host, port, timeout = 1500) => new Promise((res) => {
  const s = net.connect({ host, port, timeout });
  s.on('connect', () => { s.destroy(); res(true); });
  s.on('timeout', () => { s.destroy(); res(false); });
  s.on('error', () => res(false));
});

// ═════════════════════════════════════════════════════════════════════════════
async function main() {
  group('0. stack is up');
  newClient();
  resetRateLimits();
  await waitFor(async () => (await http('GET', '/health', { nonce: null, ts: null })).status === 200, 'gateway /health');
  await waitFor(async () => (await http('POST', '/auth/register/start', { body: { username: `probe-${crypto.randomUUID().slice(0, 8)}` } })).status === 200, 'identity via gateway');
  check('gateway healthy and real identity answering a public route through it', true);

  const uname = (p) => `${p}-${crypto.randomUUID().slice(0, 8)}`;

  newClient();
  group('1. public registration + login against the real identity');
  const a = await register(uname('alice'));
  check('register/start + register/finish succeed through the gateway without a token', !a.fail, JSON.stringify(a.fail));
  eq('issued token header is EdDSA / kid v1', [headerOf(a.token).alg, headerOf(a.token).kid], ['EdDSA', 'v1']);
  const me = await http('GET', '/me', { token: a.token });
  eq('GET /me with the real token through the gateway', [me.status, me.json?.data?.user_id, me.json?.data?.role], [200, a.userId, 'user']);
  const l1 = await login(a.username, a.auth);
  check('login/start + login/finish succeed with a real WebAuthn assertion', l1.finish?.status === 200 && !!l1.token, JSON.stringify(l1.finish?.json));
  eq('login-issued token is accepted', (await http('GET', '/me', { token: l1.token })).status, 200);
  eq('login for an unknown user gets identity error, not a gateway 401', (await loginStart('no-such-user-xyz')).status, 400);

  newClient();
  group('2. forged / tampered / unauthenticated requests fail closed');
  const [h, p, s] = a.token.split('.');
  const pl = claimsOf(a.token); pl.sub = crypto.randomUUID();
  const tampered = `${h}.${Buffer.from(JSON.stringify(pl)).toString('base64url')}.${s}`;
  eq('no token -> 401', (await http('GET', '/me')).status, 401);
  eq('tampered payload -> 401', (await http('GET', '/me', { token: tampered })).status, 401);
  const hs = `${Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT', kid: 'v1' })).toString('base64url')}.${p}.${crypto.createHmac('sha256', 'x').update('y').digest('base64url')}`;
  eq('HS256 forgery -> 401', (await http('GET', '/me', { token: hs })).status, 401);
  eq('replayed nonce -> 409', await (async () => { const n = crypto.randomUUID(); await http('GET', '/me', { token: a.token, nonce: n }); return (await http('GET', '/me', { token: a.token, nonce: n })).status; })(), 409);
  eq('missing nonce -> 400', (await http('GET', '/me', { token: a.token, nonce: null })).status, 400);

  group('3. failed logins, account lock, signals');
  const b = await register(uname('bob'));
  const uidB = b.userId;
  for (let i = 1; i <= 6; i++) {
    const r = await login(b.username, b.auth, { corrupt: true });
    if (i === 1) eq('bad assertion -> 400 webauthn_verification_failed', r.finish ? [r.finish.status, r.finish.json?.error?.code] : ['login/start failed', r.fail.status, r.fail.text.slice(0, 120)], [400, 'webauthn_verification_failed']);
  }
  eq('login:locked set by identity after >5 failures', redis('EXISTS', `login:locked:${uidB}`), '1');
  check('velocity:failed counter written by identity', Number(redis('GET', `velocity:failed:${uidB}:1h`)) >= 6, redis('GET', `velocity:failed:${uidB}:1h`));
  check('login attempt velocity counters written', Number(redis('GET', `velocity:login:${uidB}:5m`)) >= 6);
  const locked = await loginStart(b.username);
  eq('login/start while locked -> 429 account_locked', [locked.status, locked.json?.error?.code], [429, 'account_locked']);
  const lockedMe = await http('GET', '/me', { token: b.token });
  check('already-issued token is denied by gateway policy while the account is locked', lockedMe.status === 401 || lockedMe.status === 403, `status ${lockedMe.status}`);
  check('identity recorded login_failed in audit_logs', Number(psql(`select count(*) from audit_logs where user_id='${uidB}' and action='login_failed'`)) >= 6);
  redis('DEL', `login:locked:${uidB}`, `login:failed:${uidB}`);
  newClient();
  const unlocked = await login(b.username, b.auth);
  check('after the lock expires/clears a correct login works again', unlocked.finish?.status === 200, JSON.stringify(unlocked.finish?.json));
  eq('successful login clears the failure counter', redis('EXISTS', `login:failed:${uidB}`), '0');

  resetRateLimits();
  newClient();
  group('4. credential revocation');
  const c = await register(uname('carol'));
  const creds = await http('GET', '/devices', { token: c.token });
  const credId = creds.json?.data?.devices?.[0]?.id || creds.json?.data?.[0]?.id;
  check('device list available', !!credId, creds.text.slice(0, 200));
  const rv = await http('POST', `/devices/${credId}/revoke`, { token: c.token, body: {} });
  eq('revoke own credential -> 200', rv.status, 200);
  const after = await http('GET', '/me', { token: c.token, requestId: `rev-${crypto.randomUUID()}` });
  eq('token for the revoked credential is rejected by the gateway', after.status, 401);
  const dec = psql(`select detail->>'reason' from audit_logs where actor='gateway' and detail->>'request_id'='${after.requestId}'`);
  eq('gateway decision trail records the reason', dec, 'credential_not_active');
  check('revocation velocity signal written by identity', Number(redis('GET', `velocity:revocations:${c.userId}:1h`)) >= 1);

  newClient();
  group('5. logout');
  const d = await register(uname('dave'));
  eq('logout through the gateway -> 200', (await http('POST', '/auth/logout', { token: d.token, body: {} })).status, 200);
  const dd = await http('GET', '/me', { token: d.token, requestId: `out-${crypto.randomUUID()}` });
  eq('logged-out token rejected', dd.status, 401);
  eq('reason recorded: token_revoked', psql(`select detail->>'reason' from audit_logs where actor='gateway' and detail->>'request_id'='${dd.requestId}'`), 'token_revoked');

  newClient();
  group('6. step-up completion is recorded only by server-side verification');
  const e = await register(uname('erin'));
  const iat = claimsOf(e.token).iat;
  const stepKey = `stepup:completed:${e.userId}:${iat}`;
  eq('no step-up key before completion', redis('EXISTS', stepKey), '0');
  const ch1 = await http('POST', '/actions/challenge', { token: e.token, body: { action: 'rotate_keys' } });
  eq('step-up challenge issued', ch1.status, 200);
  const bad = await http('POST', '/actions/execute', { token: e.token, body: { challenge_id: ch1.json.data.challenge_id, action: 'rotate_keys', assertion: e.auth.get(ch1.json.data.publicKey, { corrupt: true }) } });
  eq('invalid assertion -> 400, not verified', [bad.status, bad.json?.data?.verified], [400, undefined]);
  eq('failed step-up leaves no completion key', redis('EXISTS', stepKey), '0');
  const ch2 = await http('POST', '/actions/challenge', { token: e.token, body: { action: 'rotate_keys' } });
  const ok = await http('POST', '/actions/execute', { token: e.token, body: { challenge_id: ch2.json.data.challenge_id, action: 'rotate_keys', assertion: e.auth.get(ch2.json.data.publicKey) } });
  eq('valid assertion -> verified', [ok.status, ok.json?.data?.verified], [200, true]);
  eq('completion key now exists (written by identity)', redis('EXISTS', stepKey), '1');
  const ttl = Number(redis('TTL', stepKey));
  check('completion expires (TTL 1..300s)', ttl >= 1 && ttl <= 300, String(ttl));
  const other = await register(uname('frank'));
  eq('another user/token is unaffected (key is per user + token)', redis('EXISTS', `stepup:completed:${other.userId}:${claimsOf(other.token).iat}`), '0');
  // A client cannot create completion by claiming it.
  const claim = await http('POST', '/actions/execute', { token: other.token, body: { challenge_id: crypto.randomUUID(), action: 'rotate_keys', assertion: {}, verified: true, completed: true } });
  check('client-supplied completion flag is ignored', claim.status >= 400 && redis('EXISTS', `stepup:completed:${other.userId}:${claimsOf(other.token).iat}`) === '0', `status ${claim.status}`);

  newClient();
  group('6b. step-up changes the gateway decision (server-verified completion only)');
  newClient(); resetRateLimits();
  const y = await register(uname('yara'));
  await sleep(1100); // a second token for the same user with a different iat
  const y2 = await login(y.username, y.auth);
  const T1 = y.token; const T2 = y2.token;
  check('two distinct session tokens for one user', !!T2 && claimsOf(T1).iat !== claimsOf(T2).iat);
  // Two recent revocations push a DeviceRevoke into the risk engine's Challenge band.
  redis('SET', `velocity:revocations:${y.userId}:1h`, '3', 'EX', '600');
  const revoke = (tok) => http('POST', `/devices/${crypto.randomUUID()}/revoke`, { token: tok, body: {} });
  const c1 = await revoke(T1);
  eq('sensitive action is challenged before step-up (T1)', c1.status, 401);
  eq('...decision reason policy_challenge', psql(`select detail->>'reason' from audit_logs where actor='gateway' and detail->>'request_id'='${c1.requestId}'`), 'policy_challenge');
  eq('...and challenged for the same user on the other token (T2)', (await revoke(T2)).status, 401);
  redis('SET', `velocity:revocations:${y.userId}:1h`, '0', 'EX', '600');
  const sc = await http('POST', '/actions/challenge', { token: T1, body: { action: 'revoke_device' } });
  const se = await http('POST', '/actions/execute', { token: T1, body: { challenge_id: sc.json.data.challenge_id, action: 'revoke_device', assertion: y.auth.get(sc.json.data.publicKey) } });
  eq('step-up completed on T1 (WebAuthn verified by identity)', [se.status, se.json?.data?.verified], [200, true]);
  redis('SET', `velocity:revocations:${y.userId}:1h`, '3', 'EX', '600');
  const a1 = await revoke(T1);
  eq('same action on T1 now passes the gateway policy (identity answers 404 for the unknown id)', a1.status, 404);
  eq('...recorded as an allow', psql(`select outcome from audit_logs where actor='gateway' and detail->>'request_id'='${a1.requestId}'`), 'allow');
  redis('DEL', `stepup:completed:${y.userId}:${claimsOf(T2).iat}`);
  const c3 = await revoke(T2);
  check('T2 (no step-up on that token) is still not allowed through', c3.status === 401 || c3.status === 202, `status ${c3.status}`);
  redis('SET', `velocity:revocations:${y.userId}:1h`, '6', 'EX', '600');
  const hi = await revoke(T1);
  check('step-up never downgrades a Hold/Deny (T1, high revocation velocity + repeated challenges)', hi.status === 403 || hi.status === 202, `status ${hi.status}`);

  group('7. audit persistence');
  const actions = psql(`select string_agg(distinct action, ',' order by action) from audit_logs where actor <> 'gateway'`).split(',');
  for (const act of ['register', 'login', 'login_failed', 'device_revoke', 'logout', 'step_up_verified'])
    check(`identity audit_logs has '${act}'`, actions.includes(act), actions.join(','));
  const allow = Number(psql(`select count(*) from audit_logs where actor='gateway' and outcome='allow'`));
  const deny = Number(psql(`select count(*) from audit_logs where actor='gateway' and outcome='deny'`));
  check('gateway decision trail has allow and deny rows in the same table', allow > 0 && deny > 0, `allow ${allow} deny ${deny}`);
  eq('no audit_events table exists (single audit table)', psql(`select to_regclass('audit_events') is null`), 't');

  newClient();
  group('8. gateway bypass attempts');
  const published = JSON.parse(`[${docker(['ps', '--filter', `label=com.docker.compose.project=${PROJECT}`, '--format', '{{json .}}']).split('\n').filter(Boolean).join(',')}]`);
  const ports = Object.fromEntries(published.map((c) => [c.Names.replace(`${PROJECT}-`, '').replace(/-1$/, ''), c.Ports]));
  check('only the gateway publishes a port', Object.entries(ports).every(([n, p]) => (n === 'gateway' ? /127\.0\.0\.1:\d+->8090/.test(p) : !/->/.test(p))), JSON.stringify(ports));
  check('gateway internal listener (9090) is not published or exposed', !/9090/.test(ports.gateway || ''));
  for (const port of [8080, 9090, 5432, 6379]) {
    // Host ports may be taken by unrelated local services; assert via the stack's own mapping instead.
    check(`host:${port} is not mapped to a stack container`, !Object.values(ports).some((p) => p.includes(`:${port}->`)));
  }
  const edgeNet = `${PROJECT}_edge`; const internalNet = `${PROJECT}_internal`;
  eq('internal network is marked internal (no host or internet route)', docker(['network', 'inspect', internalNet, '--format', '{{.Internal}}']), 'true');
  eq('edge network is not internal', docker(['network', 'inspect', edgeNet, '--format', '{{.Internal}}']), 'false');
  const members = (net_) => docker(['network', 'inspect', net_, '--format', '{{range .Containers}}{{.Name}} {{end}}']).split(/\s+/).filter(Boolean).sort();
  eq('only the gateway is on the edge network', members(edgeNet), [cname('gateway')]);
  const probe = (netName, host, port) => spawnSync('docker', ['run', '--rm', '--network', netName, '--entrypoint', 'sh', 'redis:7-alpine', '-c', `nc -z -w 2 ${host} ${port}`], { encoding: 'utf8' }).status === 0;
  check('from the edge network: identity:8080 unreachable (not resolvable)', !probe(edgeNet, 'identity', 8080));
  check('from the edge network: postgres:5432 unreachable', !probe(edgeNet, 'postgres', 5432));
  check('from the edge network: redis:6379 unreachable', !probe(edgeNet, 'redis', 6379));
  const gwIp = docker(['inspect', cname('gateway'), '--format', `{{(index .NetworkSettings.Networks "${edgeNet}").IPAddress}}`]);
  check('from the edge network: gateway :8090 reachable (the entry point)', probe(edgeNet, gwIp, 8090));
  check('from the edge network: gateway internal :9090 refused', !probe(edgeNet, gwIp, 9090));
  // A hostile neighbour already on the internal network: network reachability alone is not trusted.
  const direct = (path, hdr = '') => spawnSync('docker', ['run', '--rm', '--network', internalNet, '--entrypoint', 'sh', 'redis:7-alpine', '-c', `wget -q -O- -S ${hdr} http://identity:8080${path} 2>&1 | head -1`], { encoding: 'utf8' }).stdout.trim();
  check('internal-network neighbour calling identity directly WITHOUT the gateway secret is refused (403)', /403/.test(direct('/me')), direct('/me'));
  check('...even with a perfectly valid token', /403/.test(direct('/me', `--header "Authorization: Bearer ${a.token}"`)), '');
  check('...but identity /health (platform probe) stays reachable', /200/.test(direct('/health')), direct('/health'));
  // Through the gateway, internal paths are not special.
  for (const path of ['/internal/metrics', '/internal/audit/ingest', '/internal/diagnostics/user/x']) {
    const r = await http('GET', path, { token: a.token });
    check(`gateway external ${path} does not expose gateway-internal data`, !/gateway_auth|# TYPE/.test(r.text) && r.status !== 200, `status ${r.status}`);
  }
  const spoof = await http('GET', '/me', { token: a.token, headers: { 'x-gateway-secret': 'guess', 'x-forwarded-for': '10.0.0.1' } });
  eq('client-supplied x-gateway-secret is stripped; real request still works', spoof.status, 200);

  newClient();
  group('9. infrastructure failures fail closed');
  compose(['stop', 'redis']);
  await sleep(1500);
  const r503 = await http('GET', '/me', { token: a.token });
  eq('Redis down: authenticated request -> 503', r503.status, 503);
  const rl = await loginStart(a.username);
  check('Redis down: login/start fails fast with a server error (no hang)', rl.status >= 500 && !rl.timeout, `status ${rl.status} timeout=${!!rl.timeout}`);
  const pub = await http('POST', '/auth/register/start', { body: { username: uname('x') } });
  check('Redis down: public route not silently allowed (nonce store unavailable)', pub.status === 503, `status ${pub.status}`);
  compose(['start', 'redis']);
  await waitFor(async () => (await http('GET', '/me', { token: a.token })).status === 200, 'recovery after Redis restart', 60, 1000);
  check('Redis back: service recovers', true);
  compose(['stop', 'postgres']);
  await sleep(1500);
  const p503 = await http('GET', '/me', { token: a.token });
  eq('Postgres down: authenticated request -> 503', p503.status, 503);
  compose(['start', 'postgres']);
  await waitFor(async () => (await http('GET', '/me', { token: a.token })).status === 200, 'recovery after Postgres restart', 90, 1000);
  check('Postgres back: service recovers', true);

  // ── summary ────────────────────────────────────────────────────────────────
  const failed = results.filter((r) => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
  if (failed.length) { console.log('FAILED:'); failed.forEach((f) => console.log(`  [${f.section}] ${f.name} :: ${f.detail}`)); }
  process.exit(failed.length ? 1 : 0);
}

main().catch((e) => { console.error('runtime test aborted:', e); process.exit(2); });
