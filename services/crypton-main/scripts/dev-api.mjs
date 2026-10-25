// Local stand-in for Vercel: serves the same handlers over plain HTTP so the dev server's
// /api/assessment proxy (src/setupProxy.js, port 8091) and browser end-to-end tests exercise the
// real code. Optionally also serves a production build, for same-origin end-to-end runs.
//   npm run dev:api                      (reads api/.env.local)
//   node scripts/dev-api.mjs --static build --port 8099
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFile, stat } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.ico': 'image/x-icon', '.map': 'application/json' };

export function startApiServer({ env = process.env, port = 8091, host = '127.0.0.1', staticDir } = {}) {
  const { createApi } = require('../api/_lib/handler.js');
  const api = createApi({ env });
  const root = staticDir ? resolve(staticDir) : null;
  const server = createServer(async (req, res) => {
    const path = new URL(req.url, 'http://local').pathname;
    if (path === '/api/assessment/challenge') return api.challenge(req, res);
    if (path === '/api/assessment') return api.submit(req, res);
    if (path.startsWith('/api/')) { res.writeHead(404, { 'Content-Type': 'application/json' }); return res.end('{"error":"not-found"}'); }
    if (!root) { res.writeHead(404); return res.end(); }
    let file = resolve(root, `.${decodeURIComponent(path)}`);
    if (file !== root && !file.startsWith(root + sep)) { res.writeHead(404); return res.end(); }
    try { if (!(await stat(file)).isFile()) throw new Error('not a file'); } catch { file = resolve(root, 'index.html'); }
    res.writeHead(200, { 'Content-Type': TYPES[extname(file)] || 'application/octet-stream' });
    res.end(await readFile(file));
  });
  return new Promise((done) => server.listen(port, host, () => done(server)));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const arg = (name) => { const i = process.argv.indexOf(name); return i > -1 ? process.argv[i + 1] : undefined; };
  const server = await startApiServer({ port: Number(arg('--port') || process.env.PORT || 8091), staticDir: arg('--static') });
  console.log(`assessment API (local) listening on http://127.0.0.1:${server.address().port}`);
  for (const s of ['SIGINT', 'SIGTERM']) process.on(s, () => server.close(() => process.exit(0)));
}
