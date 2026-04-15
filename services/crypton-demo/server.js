'use strict';

/**
 * Crypton Demo Site — "Login with Crypton"
 *
 * OAuth 2.0 Authorization Code flow consumer.
 *
 * Routes:
 *   GET /          → landing page (public/index.html)
 *   GET /login     → redirect browser to Crypton /authorize
 *   GET /callback  → receive code+state, exchange for token, get userinfo, set session cookie
 *   GET /profile   → protected — shows logged-in user
 *   GET /logout    → clears session cookie
 *
 * Config (env vars):
 *   PORT            default 4000
 *   CRYPTON_URL     default http://127.0.0.1:8080   (identity service, direct — use 127.0.0.1 not localhost: Node.js 22 on Windows resolves localhost→::1 which fails when identity binds IPv4-only)
 *   CLIENT_ID       default demo-site
 *   CLIENT_SECRET   default demo-secret-change-in-prod
 *   REDIRECT_URI    default http://localhost:4000/callback
 */

const express      = require('express');
const cookieParser = require('cookie-parser');
const crypto       = require('crypto');
const path         = require('path');

const app = express();
app.use(cookieParser());
app.use(express.static(path.join(__dirname, 'public')));

const PORT         = process.env.PORT         || 4000;
const CRYPTON_URL  = process.env.CRYPTON_URL  || 'http://127.0.0.1:8080'; // 127.0.0.1 not localhost — Node fetch on Windows resolves localhost→::1
const CLIENT_ID    = process.env.CLIENT_ID    || 'demo-site';
const CLIENT_SECRET = process.env.CLIENT_SECRET || 'demo-secret-change-in-prod';
const REDIRECT_URI = process.env.REDIRECT_URI || 'http://localhost:4000/callback';

// In-memory stores (restart-safe enough for a demo)
const sessions     = new Map(); // sessionId → { username, sub, loginAt }
const pendingState = new Map(); // state     → timestamp ms

function log(msg) { console.log(`[demo] ${new Date().toISOString()} ${msg}`); }

// ── GET /login ────────────────────────────────────────────────────────────────
app.get('/login', (req, res) => {
  const state = crypto.randomBytes(16).toString('hex');
  pendingState.set(state, Date.now());

  // Expire states older than 10 min (light cleanup)
  for (const [k, ts] of pendingState) {
    if (Date.now() - ts > 600_000) pendingState.delete(k);
  }

  const params = new URLSearchParams({
    response_type: 'code',
    client_id:     CLIENT_ID,
    redirect_uri:  REDIRECT_URI,
    state,
  });

  const authUrl = `${CRYPTON_URL}/authorize?${params}`;
  log(`/login → ${authUrl}`);
  res.redirect(authUrl);
});

// ── GET /callback ─────────────────────────────────────────────────────────────
app.get('/callback', async (req, res) => {
  const { code, state, error } = req.query;

  if (error) {
    log(`/callback error from Crypton: ${error}`);
    return res.redirect(`/?error=${encodeURIComponent(error)}`);
  }
  if (!code || !state) {
    return res.status(400).send('Bad Request: missing code or state');
  }

  // Verify state
  if (!pendingState.has(state)) {
    log(`/callback invalid state=${state}`);
    return res.status(400).send('Bad Request: invalid state');
  }
  const ts = pendingState.get(state);
  pendingState.delete(state);
  if (Date.now() - ts > 600_000) {
    return res.status(400).send('Bad Request: state expired');
  }

  try {
    // 1. Exchange code for access token
    log(`/callback exchanging code for token`);
    const tokenRes = await fetch(`${CRYPTON_URL}/token`, {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({
        grant_type:    'authorization_code',
        code,
        client_id:     CLIENT_ID,
        client_secret: CLIENT_SECRET,
        redirect_uri:  REDIRECT_URI,
      }),
    });

    if (!tokenRes.ok) {
      const body = await tokenRes.text();
      log(`/callback token exchange failed: ${tokenRes.status} ${body}`);
      return res.redirect('/?error=token_exchange_failed');
    }

    const { access_token } = await tokenRes.json();

    // 2. Fetch userinfo
    const uiRes = await fetch(`${CRYPTON_URL}/userinfo`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });

    if (!uiRes.ok) {
      log(`/callback userinfo failed: ${uiRes.status}`);
      return res.redirect('/?error=userinfo_failed');
    }

    const userinfo = await uiRes.json();

    // 3. Create session
    const sessionId = crypto.randomBytes(32).toString('hex');
    sessions.set(sessionId, {
      username: userinfo.username,
      sub:      userinfo.sub,
      loginAt:  new Date().toISOString(),
    });

    log(`/callback session created for user=${userinfo.username}`);

    res.cookie('sid', sessionId, {
      httpOnly: true,
      sameSite: 'lax',
      maxAge:   3_600_000, // 1 hour
    });
    res.redirect('/profile');

  } catch (err) {
    log(`/callback internal error: ${err.message}`);
    res.redirect('/?error=internal');
  }
});

// ── GET /profile ──────────────────────────────────────────────────────────────
app.get('/profile', (req, res) => {
  const session = req.cookies.sid && sessions.get(req.cookies.sid);
  if (!session) return res.redirect('/');

  res.send(`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Crypton Demo — Profile</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: 'DM Mono', monospace, monospace;
         background: #0a0a0a; color: #f4f1ec;
         display: flex; align-items: center; justify-content: center; min-height: 100vh; }
  .card { background: #111; border: 1px solid rgba(255,255,255,.1);
          padding: 48px; width: 100%; max-width: 480px; }
  .badge { display: inline-block; background: rgba(74,222,128,.1);
           color: #4ade80; border: 1px solid rgba(74,222,128,.3);
           padding: 4px 14px; font-size: 10px; letter-spacing: .12em;
           text-transform: uppercase; margin-bottom: 28px; }
  h2 { font-size: 32px; letter-spacing: .04em; text-transform: uppercase; margin-bottom: 28px; }
  .label { font-size: 9px; letter-spacing: .12em; text-transform: uppercase;
           color: #7a7570; margin-bottom: 4px; }
  .value { font-size: 15px; color: #c8f55a; margin-bottom: 20px; word-break: break-all; }
  .value.muted { font-size: 12px; color: #7a7570; }
  a { color: #c8f55a; font-size: 11px; text-decoration: none; letter-spacing: .08em;
      text-transform: uppercase; }
  a:hover { text-decoration: underline; }
</style>
</head>
<body>
<div class="card">
  <div class="badge">✓ Authenticated via Crypton</div>
  <h2>Welcome back</h2>
  <div class="label">Username</div>
  <div class="value">${escHtml(session.username)}</div>
  <div class="label">User ID</div>
  <div class="value muted">${escHtml(session.sub)}</div>
  <div class="label">Session started</div>
  <div class="value muted">${escHtml(session.loginAt)}</div>
  <a href="/logout">↪ Sign out</a>
</div>
</body>
</html>`);
});

// ── GET /logout ───────────────────────────────────────────────────────────────
app.get('/logout', (req, res) => {
  const sid = req.cookies.sid;
  if (sid) {
    sessions.delete(sid);
    log(`/logout session=${sid.slice(0, 8)}…`);
  }
  res.clearCookie('sid');
  res.redirect('/');
});

// ── Helpers ───────────────────────────────────────────────────────────────────
function escHtml(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

app.listen(PORT, () => log(`running on http://localhost:${PORT}`));
