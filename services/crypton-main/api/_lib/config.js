'use strict';

/**
 * Environment configuration for the assessment API. Server-side only: nothing in this
 * folder is part of the browser bundle (the app is built from src/; api/ is deployed as
 * Vercel Functions). Configuration is validated on every request and FAILS CLOSED: if
 * anything required is missing or unsafe the API answers 503 and stores nothing.
 *
 * Required
 *   ASSESSMENT_MASTER_KEY       64 hex characters. Root secret; per-purpose keys are derived from it.
 *   ASSESSMENT_DATABASE_URL     postgres:// connection string for the least-privilege
 *                               `assessment_writer` role (pooler URL on Vercel).
 *   ASSESSMENT_ALLOWED_ORIGINS  Comma-separated exact origins, e.g. https://cryptonid.tech,https://www.cryptonid.tech
 * Optional
 *   ASSESSMENT_DATABASE_SSL     verify (default) | require | disable (local development only)
 *   ASSESSMENT_DATABASE_CA      PEM certificate authority for `verify` (use \n for newlines)
 */

const HEX64 = /^[a-f\d]{64}$/i;
const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]', '::1']);

function parseOrigin(value, production) {
  let url;
  try { url = new URL(value); } catch { return null; }
  if (url.origin !== value.replace(/\/$/, '')) return null; // an origin, not a URL with a path
  const local = LOCAL_HOSTS.has(url.hostname);
  if (url.protocol !== 'https:' && !(local && !production)) return null;
  return url.origin;
}

function loadConfig(env = process.env) {
  const production = env.VERCEL_ENV === 'production' || env.NODE_ENV === 'production';
  const missing = [];

  const masterKey = env.ASSESSMENT_MASTER_KEY || '';
  if (!HEX64.test(masterKey)) missing.push('ASSESSMENT_MASTER_KEY');

  const databaseUrl = env.ASSESSMENT_DATABASE_URL || '';
  let host = '';
  try {
    const url = new URL(databaseUrl);
    if (!/^postgres(ql)?:$/.test(url.protocol)) throw new Error('scheme');
    host = url.hostname;
  } catch { missing.push('ASSESSMENT_DATABASE_URL'); }

  const origins = new Set();
  for (const raw of (env.ASSESSMENT_ALLOWED_ORIGINS || '').split(',').map((s) => s.trim()).filter(Boolean)) {
    const origin = parseOrigin(raw, production);
    if (origin) origins.add(origin); else missing.push('ASSESSMENT_ALLOWED_ORIGINS');
  }
  // A preview deployment is served from its own generated hostnames. Allow exactly those,
  // and never outside previews.
  if (!production) {
    for (const hostname of [env.VERCEL_URL, env.VERCEL_BRANCH_URL]) if (hostname) origins.add(`https://${hostname}`);
  }
  if (!origins.size && !missing.includes('ASSESSMENT_ALLOWED_ORIGINS')) missing.push('ASSESSMENT_ALLOWED_ORIGINS');

  const mode = (env.ASSESSMENT_DATABASE_SSL || 'verify').toLowerCase();
  let ssl;
  if (mode === 'disable') {
    if (production || !LOCAL_HOSTS.has(host)) missing.push('ASSESSMENT_DATABASE_SSL');
    ssl = false;
  } else if (mode === 'require') {
    ssl = { rejectUnauthorized: false }; // encrypted but the server is not authenticated: explicit opt-in only
  } else if (mode === 'verify') {
    const ca = env.ASSESSMENT_DATABASE_CA ? env.ASSESSMENT_DATABASE_CA.replace(/\\n/g, '\n') : undefined;
    ssl = ca ? { rejectUnauthorized: true, ca } : { rejectUnauthorized: true };
  } else {
    missing.push('ASSESSMENT_DATABASE_SSL');
  }

  if (missing.length) return { ok: false, missing: [...new Set(missing)] };
  return { ok: true, production, masterKey, databaseUrl, origins, ssl, sslMode: mode, ca: env.ASSESSMENT_DATABASE_CA || '' };
}

module.exports = { loadConfig, parseOrigin };
