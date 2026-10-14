/**
 * Assessment inquiry: field definitions and validation. Pure so it can be unit tested.
 * There is NO submission backend yet (see Inquiry.jsx and README); nothing here sends data.
 */

export const INTEREST_OPTIONS = [
  { value: 'principal_continuity', label: 'Principal continuity: who was actually behind the action? (research preview)' },
  { value: 'delegation_scope', label: 'Delegation and authority scope' },
  { value: 'approval_binding', label: 'Approval and execution binding' },
  { value: 'not_sure', label: 'Not sure yet' },
];

export const INITIAL_VALUES = {
  name: '',
  email: '',
  organization: '',
  interest: 'principal_continuity',
  context: '',
  consent: false,
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

  if (!email) errors.email = 'Enter your email address.';
  else if (email.length > LIMITS.email || !EMAIL.test(email)) errors.email = 'Enter a valid email address, like name@company.com.';

  if ((v.organization || '').length > LIMITS.organization) errors.organization = `Keep this under ${LIMITS.organization} characters.`;

  if (!INTEREST_OPTIONS.some((o) => o.value === v.interest)) errors.interest = 'Choose what you would like to discuss.';

  if ((v.context || '').length > LIMITS.context) errors.context = `Keep this under ${LIMITS.context} characters.`;

  if (!v.consent) errors.consent = 'Please confirm you agree to be contacted about this inquiry.';

  return errors;
}
