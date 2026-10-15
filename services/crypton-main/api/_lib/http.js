'use strict';

const { isIP } = require('node:net');

const BODY_LIMIT = 8192;
const BODY_TIMEOUT_MS = 8000;

function sendJson(res, code, body, extra = {}) {
  res.writeHead(code, {
    'Content-Type': 'application/json; charset=utf-8',
    'Cache-Control': 'no-store',
    'X-Content-Type-Options': 'nosniff',
    ...extra,
  });
  res.end(JSON.stringify(body));
}

/**
 * Reads at most `limit` bytes. The body is read straight from the stream (never through a
 * platform body parser), so the cap is enforced before anything is parsed, and a slow sender
 * cannot hold the function open.
 */
function readBody(req, limit = BODY_LIMIT, timeoutMs = BODY_TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    const declared = Number(req.headers['content-length'] || 0);
    if (declared > limit) return reject({ code: 413, error: 'too-large' });
    const chunks = [];
    let length = 0;
    const timer = setTimeout(() => { req.destroy(); reject({ code: 408, error: 'timeout' }); }, timeoutMs);
    req.on('data', (chunk) => {
      length += chunk.length;
      if (length > limit) { clearTimeout(timer); req.destroy(); reject({ code: 413, error: 'too-large' }); return; }
      chunks.push(chunk);
    });
    req.on('end', () => { clearTimeout(timer); resolve(Buffer.concat(chunks)); });
    req.on('error', () => { clearTimeout(timer); reject({ code: 400, error: 'request' }); });
  });
}

/**
 * Cross-origin requests are refused. A browser POST always carries Origin, so it must be one
 * of ours. With no Origin the only thing allowed is a same-origin GET (the form token), which
 * browsers mark with Sec-Fetch-Site.
 */
function originAllowed(req, allowed) {
  const origin = req.headers.origin;
  const site = req.headers['sec-fetch-site'];
  if (site === 'cross-site') return false;
  if (origin) return allowed.has(origin);
  return req.method === 'GET' && site === 'same-origin';
}

/**
 * The client's address. On Vercel the platform sets these headers and discards any the
 * client sent, so they can be trusted there. Anywhere else only the socket address is used,
 * so a caller cannot choose its own throttle bucket by sending a header.
 */
function clientAddress(req, env = process.env) {
  if (env.VERCEL === '1') {
    for (const name of ['x-vercel-forwarded-for', 'x-real-ip', 'x-forwarded-for']) {
      const first = String(req.headers[name] || '').split(',')[0].trim();
      if (isIP(first)) return first;
    }
  }
  return (req.socket && req.socket.remoteAddress) || 'unknown';
}

module.exports = { BODY_LIMIT, sendJson, readBody, originAllowed, clientAddress };
