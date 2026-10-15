'use strict';

const { loadConfig } = require('./config');
const crypto = require('./crypto');
const { validateSubmission } = require('./validate');
const { sendJson, readBody, originAllowed, clientAddress } = require('./http');
const { createStore } = require('./db');

const PRIVACY_VERSION = '2026-10-14';
const RECEIPT = /^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/i;

/** Only a short, data-free code ever reaches the logs: never a message, a value or a SQL fragment. */
const safeCode = (error) => String((error && (error.code || error.name)) || 'unknown').slice(0, 40);

/**
 * The assessment API as two plain Node request handlers, so the same code runs as Vercel
 * Functions, behind the local dev adapter, and in tests. Nothing here keeps state between
 * requests: tokens are signed, throttles and idempotency live in Postgres.
 *
 * Contract (unchanged from the original inbox service):
 *   GET  /api/assessment/challenge  -> 200 { token }
 *   POST /api/assessment            -> 201 { status: "received", receipt }   stored
 *                                      200 { status: "received", receipt }   identical retry
 *                                      400 verification | json    403 origin    405 method
 *                                      409 changed-retry          410 expired   413 too-large
 *                                      415 content-type           422 validation (+ errors)
 *                                      425 too-fast               429 rate-limit
 *                                      503 unavailable            (anything we cannot confirm)
 * "received" is only ever sent after the database has committed the row.
 */
function createApi({ env = process.env, store: injectedStore, now = () => Date.now(), log = console } = {}) {
  let memo = null;
  function setup() {
    const config = loadConfig(env);
    if (!config.ok) return { ok: false, missing: config.missing, why: config.why };
    if (!memo || memo.masterKey !== config.masterKey || memo.databaseUrl !== config.databaseUrl) {
      memo = { masterKey: config.masterKey, databaseUrl: config.databaseUrl, keys: crypto.deriveKeys(config.masterKey), store: injectedStore || createStore(config) };
    }
    return { ok: true, config, keys: memo.keys, store: memo.store };
  }

  function unavailable(res, missing, why = {}) {
    // Names of missing settings and whether each is unset or malformed, never their values.
    if (missing) log.error('assessment: configuration incomplete', missing.map((name) => (why[name] ? `${name}=${why[name]}` : name)).join(','));
    if (!res.headersSent) sendJson(res, 503, { error: 'unavailable' });
  }

  async function challenge(req, res) {
    try {
      if (req.method !== 'GET') return sendJson(res, 405, { error: 'method' }, { Allow: 'GET' });
      const ctx = setup();
      if (!ctx.ok) return unavailable(res, ctx.missing, ctx.why);
      if (!originAllowed(req, ctx.config.origins)) return sendJson(res, 403, { error: 'origin' });
      return sendJson(res, 200, { token: crypto.issueChallenge(ctx.keys, now()) });
    } catch (error) {
      log.error('assessment: challenge failed', safeCode(error));
      return unavailable(res);
    }
  }

  async function submit(req, res) {
    try {
      if (req.method !== 'POST') return sendJson(res, 405, { error: 'method' }, { Allow: 'POST' });
      const ctx = setup();
      if (!ctx.ok) return unavailable(res, ctx.missing, ctx.why);
      const { config, keys, store } = ctx;

      if (!originAllowed(req, config.origins)) return sendJson(res, 403, { error: 'origin' });
      if (!/^application\/json(?:\s*;|$)/i.test(req.headers['content-type'] || '')) return sendJson(res, 415, { error: 'content-type' });

      let raw;
      try { raw = await readBody(req); } catch (e) { return sendJson(res, e.code || 400, { error: e.error || 'request' }); }
      let input;
      try { input = JSON.parse(raw.toString('utf8')); } catch { return sendJson(res, 400, { error: 'json' }); }

      // Everything below up to the database call is cheap and stateless: junk never costs a query.
      const checked = validateSubmission(input);
      if (!checked.ok) return sendJson(res, checked.code, { error: checked.error, ...(checked.errors ? { errors: checked.errors } : {}) });
      const token = crypto.verifyChallenge(keys, checked.token, now());
      if (!token.ok) return sendJson(res, token.code, { error: token.error });

      const payload = { ...checked.value, consent: true, source: checked.source, privacyVersion: PRIVACY_VERSION };
      const result = await store.submit({
        idempotencyKey: token.id,
        fingerprint: crypto.fingerprint(keys, checked.value),
        emailHash: crypto.emailHash(keys, checked.value.email),
        clientHash: crypto.clientHash(keys, clientAddress(req, env)),
        keyId: keys.keyId,
        payload: crypto.encryptPayload(keys, payload, token.id),
        privacyVersion: PRIVACY_VERSION,
        source: checked.source,
      });

      if ((result.status === 201 || result.status === 200) && RECEIPT.test(String(result.id || ''))) {
        return sendJson(res, result.status, { status: 'received', receipt: result.id });
      }
      if (result.status === 409) return sendJson(res, 409, { error: 'changed-retry' });
      if (result.status === 429) return sendJson(res, 429, { error: 'rate-limit' }, { 'Retry-After': '60' });
      // Any other answer is something we cannot vouch for: never turn it into success.
      log.error('assessment: unexpected storage result', String(result.status));
      return unavailable(res);
    } catch (error) {
      log.error('assessment: storage unavailable', safeCode(error));
      return unavailable(res);
    }
  }

  return { challenge, submit };
}

module.exports = { createApi, PRIVACY_VERSION };
