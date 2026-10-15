import { createServer } from 'node:http';
import { isIP } from 'node:net';
import { readFile, stat } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openInbox, settings } from './store.mjs';

export function createInquiryServer({ inbox, origin, buildDir, trustedProxy = '' }) {
  const json = (res, code, body) => {
    res.writeHead(code, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff', ...(code === 429 ? { 'Retry-After': '60' } : {}) });
    res.end(JSON.stringify(body));
  };
  const server = createServer(async (req, res) => {
    try {
      const path = new URL(req.url, origin).pathname;
      if (path.startsWith('/api/assessment')) {
        if (req.headers.origin && req.headers.origin !== origin) return json(res, 403, { error: 'origin' });
        if (!req.headers.origin && !(req.method === 'GET' && req.headers['sec-fetch-site'] === 'same-origin')) return json(res, 403, { error: 'origin' });
        if (req.headers['sec-fetch-site'] === 'cross-site') return json(res, 403, { error: 'origin' });
        let ip = req.socket.remoteAddress;
        // Only trust an explicitly configured immediate proxy; use its rightmost appended address.
        if (trustedProxy && ip === trustedProxy) {
          const forwarded = String(req.headers['x-forwarded-for'] || '').split(',').pop().trim();
          if (isIP(forwarded)) ip = forwarded;
        }
        if (!inbox.rate(ip)) return json(res, 429, { error: 'rate-limit' });
        if (path === '/api/assessment/challenge' && req.method === 'GET') return json(res, 200, { token: inbox.challenge() });
        if (path !== '/api/assessment') return json(res, 404, { error: 'not-found' });
        if (req.method !== 'POST') { res.setHeader('Allow', 'POST'); return json(res, 405, { error: 'method' }); }
        if (!/^application\/json(?:\s*;|$)/i.test(req.headers['content-type'] || '')) return json(res, 415, { error: 'content-type' });
        if (Number(req.headers['content-length'] || 0) > 8192) return json(res, 413, { error: 'too-large' });
        let length = 0; const chunks = [];
        for await (const chunk of req) {
          length += chunk.length;
          if (length > 8192) return json(res, 413, { error: 'too-large' });
          chunks.push(chunk);
        }
        let input;
        try { input = JSON.parse(Buffer.concat(chunks).toString('utf8')); }
        catch { return json(res, 400, { error: 'json' }); }
        const { code, ...body } = inbox.accept(input);
        return json(res, code, body);
      }
      if (path.startsWith('/api/')) return json(res, 404, { error: 'not-found' });
      if (req.method === 'GET' && path === '/health') return json(res, 200, { status: 'ok' });
      if (buildDir && (req.method === 'GET' || req.method === 'HEAD')) {
        const base = resolve(buildDir);
        let file = resolve(base, `.${decodeURIComponent(path)}`);
        if (file !== base && !file.startsWith(base + sep)) return json(res, 404, { error: 'not-found' });
        try { if (!(await stat(file)).isFile()) throw new Error('not-file'); }
        catch { if (path === '/' || path === '/agent-floor') file = resolve(base, 'index.html'); else return json(res, 404, { error: 'not-found' }); }
        const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.ico': 'image/x-icon', '.json': 'application/json', '.woff2': 'font/woff2', '.txt': 'text/plain' };
        const body = await readFile(file);
        res.writeHead(200, { 'Content-Type': types[extname(file)] || 'application/octet-stream', 'X-Content-Type-Options': 'nosniff', 'Referrer-Policy': 'strict-origin-when-cross-origin', 'Cache-Control': extname(file) === '.html' ? 'no-cache' : 'public, max-age=3600' });
        return res.end(req.method === 'HEAD' ? undefined : body);
      }
      return json(res, 404, { error: 'not-found' });
    } catch {
      // Never log visitor details or database/crypto errors to a public response.
      console.error('Inquiry request failed; check private storage availability.');
      if (!res.headersSent) json(res, 503, { error: 'unavailable' });
      else res.end();
    }
  });
  server.requestTimeout = 15000;
  server.headersTimeout = 10000;
  return server;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const config = settings();
  if (process.env.INQUIRY_BUILD_DIR && config.dbPath.startsWith(resolve(process.env.INQUIRY_BUILD_DIR) + sep)) throw new Error('Private inbox must be outside the public build directory.');
  const inbox = openInbox(config);
  const server = createInquiryServer({ inbox, origin: config.origin, buildDir: process.env.INQUIRY_BUILD_DIR, trustedProxy: process.env.INQUIRY_TRUSTED_PROXY });
  const port = Number(process.env.PORT || 8091);
  server.listen(port, process.env.HOST || '127.0.0.1', () => console.log(`Private inquiry service listening on port ${port}; inbox persistence enabled.`));
  for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close(() => { inbox.close(); process.exit(0); }));
}
