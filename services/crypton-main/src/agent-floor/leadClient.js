/** Single submission boundary. Receipt means the private inbox committed the request. */
export const ASSESSMENT_ENDPOINT = process.env.REACT_APP_ASSESSMENT_ENDPOINT || '/api/assessment';
export function isConfigured(endpoint = ASSESSMENT_ENDPOINT) {
  return /^https:\/\/\S+$/.test(endpoint) || /^\/[^/]\S*$/.test(endpoint) || /^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?\//.test(endpoint);
}
export function toPayload(values) {
  return {
    name: values.name.trim(), email: values.email.trim(), organization: values.organization.trim(),
    context: values.context.trim(), consent: values.consent === true, source: 'agent-floor',
    website: values.website || '', token: values.token || '',
  };
}
const receiptPattern = /^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/i;
export async function getAssessmentChallenge({ endpoint = ASSESSMENT_ENDPOINT, fetchImpl = fetch } = {}) {
  const response = await fetchImpl(`${endpoint}/challenge`, { headers: { Accept: 'application/json' }, cache: 'no-store', credentials: 'same-origin', signal: AbortSignal.timeout(10000) });
  if (!response.ok || !/application\/json/i.test(response.headers.get('content-type') || '')) throw new Error('unavailable');
  const data = await response.json();
  if (typeof data.token !== 'string' || data.token.length > 200) throw new Error('unavailable');
  return data.token;
}
/** Never turn a timeout, redirect, HTML fallback, or arbitrary 2xx into success. */
export async function submitAssessmentRequest(values, { endpoint = ASSESSMENT_ENDPOINT, fetchImpl } = {}) {
  if (!isConfigured(endpoint)) return { status: 'not-configured' };
  const doFetch = fetchImpl || (typeof fetch === 'function' ? fetch : null);
  if (!doFetch) return { status: 'error' };
  try {
    const response = await doFetch(endpoint, {
      method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      credentials: 'same-origin', redirect: 'error', signal: AbortSignal.timeout(15000), body: JSON.stringify(toPayload(values)),
    });
    const data = /application\/json/i.test(response.headers?.get('content-type') || '') ? await response.json() : {};
    if (response.ok && [200, 201].includes(response.status) && data.status === 'received' && receiptPattern.test(data.receipt || '')) return { status: 'sent', receipt: data.receipt };
    return { status: 'error', code: response.status, errors: data.errors, reason: data.error };
  } catch { return { status: 'error' }; }
}
