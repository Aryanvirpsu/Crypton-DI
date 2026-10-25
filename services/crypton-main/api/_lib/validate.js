'use strict';

/**
 * Server-side validation. These are the same rules, limits, messages and order as the
 * original inbox service (services/crypton-inquiries/store.mjs), so the public contract the
 * form already handles is unchanged: 422 for field errors, 400 for failed verification.
 * The browser validates too, but nothing here trusts it.
 */

const LIMITS = { name: 80, email: 254, organization: 120, context: 1000 };
// Control characters other than tab, LF and CR.
const CONTROL = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/;
const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;
const SOURCE = 'agent-floor';

function validateSubmission(input) {
  if (!input || Array.isArray(input) || typeof input !== 'object') return { ok: false, code: 422, error: 'validation' };

  const errors = {};
  const value = {};
  for (const [field, limit] of Object.entries(LIMITS)) {
    const raw = input[field];
    if (typeof raw !== 'string' || raw.length > limit || CONTROL.test(raw)) errors[field] = 'Check this field and try again.';
    else value[field] = raw.trim();
  }
  if (!errors.name && value.name.length < 2) errors.name = 'Enter your name (at least 2 characters).';
  if (!errors.email && !EMAIL.test(value.email)) errors.email = 'Enter a valid email address.';
  if (input.consent !== true) errors.consent = 'Please confirm we may contact you about this request.';
  if (Object.keys(errors).length) return { ok: false, code: 422, error: 'validation', errors };

  // Anti-automation fields: a filled honeypot or a wrong source is not a user mistake.
  if (input.source !== SOURCE || input.website !== '' || typeof input.token !== 'string' || input.token.length > 200) {
    return { ok: false, code: 400, error: 'verification' };
  }
  return { ok: true, value, token: input.token, source: SOURCE };
}

module.exports = { LIMITS, SOURCE, validateSubmission };
