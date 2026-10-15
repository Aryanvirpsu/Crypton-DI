// Operator tool: read assessment requests and move their status along. Run locally, by a person.
//   npm run inbox                         list new requests, decrypted
//   npm run inbox -- list --all           every request
//   npm run inbox -- status <id> contacted
//
// Connects as `assessment_operator` (ASSESSMENT_OPERATOR_DATABASE_URL), a role that can read
// inquiries and change their status, and nothing else. Decrypts on this machine with the root
// key(s); the database and the web tier never see plaintext. Output contains personal data:
// do not paste it into tickets or chat.
import { createRequire } from 'node:module';
import postgres from 'postgres';

const require = createRequire(import.meta.url);
const { deriveKeys, decryptPayload } = require('../api/_lib/crypto.js');

const STATUSES = ['new', 'reviewing', 'contacted', 'qualified', 'closed', 'spam'];
const url = process.env.ASSESSMENT_OPERATOR_DATABASE_URL;
const masters = [process.env.ASSESSMENT_MASTER_KEY, process.env.ASSESSMENT_MASTER_KEY_PREVIOUS].filter((k) => /^[a-f\d]{64}$/i.test(k || ''));
if (!url || !masters.length) {
  console.error('Set ASSESSMENT_OPERATOR_DATABASE_URL and ASSESSMENT_MASTER_KEY (optionally ASSESSMENT_MASTER_KEY_PREVIOUS).');
  process.exit(2);
}
const keysById = new Map(masters.map((k) => { const keys = deriveKeys(k); return [keys.keyId, keys]; }));
const sql = postgres(url, { max: 1, prepare: false, connect_timeout: 8, onnotice: () => {}, ssl: process.env.ASSESSMENT_DATABASE_SSL === 'disable' ? false : { rejectUnauthorized: process.env.ASSESSMENT_DATABASE_SSL !== 'require', ca: process.env.ASSESSMENT_DATABASE_CA || undefined } });

try {
  const [command = 'list', ...rest] = process.argv.slice(2);
  if (command === 'status') {
    const [id, status] = rest;
    if (!STATUSES.includes(status)) throw new Error(`status must be one of: ${STATUSES.join(', ')}`);
    const rows = await sql`update assessment.assessment_inquiries set status = ${status} where id = ${id}::uuid returning id, status`;
    console.log(rows.length ? `${rows[0].id} -> ${rows[0].status}` : 'no such request');
  } else {
    const all = rest.includes('--all');
    const rows = all
      ? await sql`select id, idempotency_key, status, key_id, payload, created_at from assessment.assessment_inquiries order by created_at desc limit 200`
      : await sql`select id, idempotency_key, status, key_id, payload, created_at from assessment.assessment_inquiries where status = 'new' order by created_at desc limit 200`;
    const out = rows.map((r) => {
      const keys = keysById.get(r.key_id);
      if (!keys) return { id: r.id, status: r.status, created: r.created_at, error: `no key for key_id ${r.key_id}` };
      const p = decryptPayload(keys, r.payload, r.idempotency_key);
      return { id: r.id, status: r.status, created: r.created_at, name: p.name, email: p.email, organization: p.organization, context: p.context, privacyVersion: p.privacyVersion };
    });
    console.log(JSON.stringify(out, null, 2));
  }
} catch (error) {
  console.error(`Failed: ${error.code || error.name}`);
  process.exitCode = 1;
} finally {
  await sql.end({ timeout: 2 });
}
