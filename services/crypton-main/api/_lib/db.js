'use strict';

const postgres = require('postgres');

/**
 * Database access for the web tier. It connects as `assessment_writer`, whose only
 * privilege is EXECUTE on assessment.submit_inquiry(). Even a full compromise of this
 * function's credentials cannot read, change or list a single lead.
 *
 * Serverless notes:
 *  - one connection per function instance, reused while the instance is warm;
 *  - `prepare: false` because the Supabase pooler runs in transaction mode;
 *  - short connect and idle timeouts, so a cold or abandoned instance never hoards connections.
 */

let cached = null;

function client(config) {
  const key = [config.databaseUrl, config.sslMode, config.ca].join('|');
  if (cached && cached.key === key) return cached.sql;
  if (cached) cached.sql.end({ timeout: 0 }).catch(() => {});
  const sql = postgres(config.databaseUrl, {
    max: 1,
    prepare: false,
    ssl: config.ssl,
    connect_timeout: 5,
    idle_timeout: 10,
    max_lifetime: 300,
    onnotice: () => {},
    connection: { application_name: 'crypton-assessment' },
  });
  cached = { key, sql };
  return sql;
}

function withTimeout(promise, ms) {
  let timer;
  const timeout = new Promise((_, reject) => { timer = setTimeout(() => reject(Object.assign(new Error('timeout'), { code: 'ETIMEDOUT' })), ms); });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

function createStore(config) {
  return {
    /** Resolves to { status, outcome, id }. Rejects on ANY database problem: callers must treat that as "not stored". */
    async submit(a) {
      const sql = client(config);
      const rows = await withTimeout(sql`
        select http_status, outcome, inquiry_id
        from assessment.submit_inquiry(
          ${a.idempotencyKey}::uuid, ${a.fingerprint}, ${a.emailHash}, ${a.clientHash},
          ${a.keyId}, ${a.payload}::bytea, ${a.privacyVersion}, ${a.source})`, 8000);
      const row = rows[0];
      if (!row) throw Object.assign(new Error('no result'), { code: 'NORESULT' });
      return { status: row.http_status, outcome: row.outcome, id: row.inquiry_id };
    },
    async close() { if (cached) { const { sql } = cached; cached = null; await sql.end({ timeout: 2 }); } },
  };
}

module.exports = { createStore };
