/**
 * Assessment request: field definitions and validation. Pure so it can be unit tested.
 * Sending lives in leadClient.js; nothing here sends data.
 */

export const INITIAL_VALUES = {
  name: '',
  email: '',
  organization: '',
  context: '',
  consent: false,
  website: '',
};

export const LIMITS = { name: 80, email: 254, organization: 120, context: 1000 };

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;

/** Returns an object of field -> message. Empty object means valid. */
export function validateInquiry(v) {
  const errors = {};
  const name = (v.name || '').trim();
  const email = (v.email || '').trim();

  if (name.length < 2) errors.name = 'Enter your name (at least 2 characters).';
  else if (name.length > LIMITS.name) errors.name = `Keep your name under ${LIMITS.name} characters.`;

  if (!email) errors.email = 'Enter your work email.';
  else if (email.length > LIMITS.email || !EMAIL.test(email)) errors.email = 'Enter a valid email address, like you@company.com.';

  if ((v.organization || '').length > LIMITS.organization) errors.organization = `Keep this under ${LIMITS.organization} characters.`;

  if ((v.context || '').length > LIMITS.context) errors.context = `Keep this under ${LIMITS.context} characters.`;

  if (!v.consent) errors.consent = 'Please confirm we may contact you about this request.';

  return errors;
}
