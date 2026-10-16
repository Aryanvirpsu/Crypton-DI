import React, { useEffect, useRef, useState } from 'react';
import { INITIAL_VALUES, LIMITS, validateInquiry } from './inquiryModel';
import { getAssessmentChallenge, isConfigured, submitAssessmentRequest } from './leadClient';

/**
 * The assessment request: the last scene of the Agent Floor. Three fields, one optional
 * disclosure, one button. Submission goes through leadClient.js only, and success is
 * shown only when the private inbox returns a committed storage receipt.
 */
export default function AssessmentForm({ connected = isConfigured(), submit = submitAssessmentRequest, firstFieldRef }) {
  const [values, setValues] = useState(INITIAL_VALUES);
  const [errors, setErrors] = useState({});
  const [status, setStatus] = useState('idle'); // idle | sending | sent | not-configured | error
  const resultRef = useRef(null);
  const formRef = useRef(null);
  const tokenRef = useRef('');
  const challengeRef = useRef(null);
  const sendingRef = useRef(false);
  const [failure, setFailure] = useState({});
  const [receipt, setReceipt] = useState('');
  useEffect(() => {
    let active = true;
    if (connected) {
      challengeRef.current = getAssessmentChallenge();
      challengeRef.current.then((token) => { if (active) tokenRef.current = token; }).catch(() => { challengeRef.current = null; });
    }
    return () => { active = false; };
  }, [connected]);

  const set = (field) => (e) => {
    const value = e.target.type === 'checkbox' ? e.target.checked : e.target.value;
    setValues((v) => ({ ...v, [field]: value }));
    if (errors[field]) setErrors((er) => ({ ...er, [field]: undefined }));
    if (status !== 'sending' && status !== 'sent') setStatus('idle');
  };

  const focusResult = () => setTimeout(() => resultRef.current && resultRef.current.focus(), 0);

  const onSubmit = async (e) => {
    e.preventDefault();
    if (sendingRef.current || status === 'sent') return;
    const found = validateInquiry(values);
    setErrors(found);
    if (Object.keys(found).length) {
      setStatus('idle');
      const first = formRef.current?.querySelector(`#af-f-${Object.keys(found)[0]}`);
      const disclosure = first?.closest('details');
      if (disclosure) disclosure.open = true;
      first?.focus();
      return;
    }
    if (!connected) { setStatus('not-configured'); focusResult(); return; }
    sendingRef.current = true;
    setStatus('sending');
    try {
      if (connected && !tokenRef.current && submit === submitAssessmentRequest) tokenRef.current = await (challengeRef.current || getAssessmentChallenge());
      const res = await submit({ ...values, token: tokenRef.current });
      if (res.status === 'sent' && /^[a-f\d]{8}-([a-f\d]{4}-){3}[a-f\d]{12}$/i.test(res.receipt || '')) {
        setReceipt(res.receipt); setStatus('sent');
      } else {
        setStatus(res.status === 'not-configured' ? 'not-configured' : 'error');
        setFailure(res);
        if (res.code === 410 || res.code === 409) { tokenRef.current = ''; challengeRef.current = null; }
        if (res.errors) setErrors(res.errors);
      }
    } catch { setStatus('error'); setFailure({}); }
    finally { sendingRef.current = false; focusResult(); }
  };

  const errorList = Object.entries(errors).filter(([, m]) => m);
  const field = (id) => ({
    id: `af-f-${id}`,
    'aria-invalid': errors[id] ? 'true' : undefined,
    'aria-describedby': errors[id] ? `af-e-${id}` : undefined,
  });
  const err = (id) => errors[id] && <p className="af-field-error" id={`af-e-${id}`}>{errors[id]}</p>;

  if (status === 'sent') {
    return (
      <div className="af-form-done" ref={resultRef} tabIndex={-1} role="status">
        <p className="af-form-done-title">Request received.</p>
        <p>Thanks, {values.name.trim()}. Your request is saved in Crypton’s private inbox. We’ll reply to {values.email.trim()} to discuss the workflow.</p>
        <p className="af-hint">Reference: {receipt}</p>
      </div>
    );
  }

  return (
    <form ref={formRef} className="af-form af-form--assess" onSubmit={onSubmit} noValidate aria-labelledby="af-form-h">
      <h3 id="af-form-h" className="af-form-title">Get an assessment</h3>
      <p className="af-form-lead">Tell us where your agents operate. We’ll follow up to discuss an assessment.</p>

      <div className="af-field">
        <label htmlFor="af-f-name">Name</label>
        <input ref={firstFieldRef} type="text" autoComplete="name" placeholder="Your name" value={values.name} onChange={set('name')} name="name" maxLength={LIMITS.name} required {...field('name')} />
        {err('name')}
      </div>
      <div className="af-field">
        <label htmlFor="af-f-email">Work email</label>
        <input type="email" name="email" maxLength={LIMITS.email} spellCheck={false} autoComplete="email" inputMode="email" placeholder="you@company.com" value={values.email} onChange={set('email')} required {...field('email')} />
        {err('email')}
      </div>
      <div className="af-field">
        <label htmlFor="af-f-organization">Company or project <span className="af-muted">(optional)</span></label>
        <input type="text" name="organization" maxLength={LIMITS.organization} autoComplete="organization" placeholder="Company name" value={values.organization} onChange={set('organization')} {...field('organization')} />
        {err('organization')}
      </div>
      <details className="af-details af-form-more">
        <summary>Tell us about your agent system <span className="af-muted">(optional)</span></summary>
        <div className="af-field">
          <label htmlFor="af-f-context">What do your agents do, and what can they touch?</label>
          <textarea name="context" maxLength={LIMITS.context} onInput={(e) => { e.currentTarget.style.height = "auto"; e.currentTarget.style.height = `${e.currentTarget.scrollHeight}px`; }} rows={3} value={values.context} onChange={set('context')} {...field('context')} />
          <p className="af-hint">A few sentences is plenty. Please don’t include credentials, keys or customer data.</p>
          {err('context')}
        </div>
      </details>
      <div className="af-field af-field--check">
        <input type="checkbox" name="consent" checked={values.consent} onChange={set('consent')} {...field('consent')} />
        <label htmlFor="af-f-consent">You may contact me about this request.</label>
        {err('consent')}
      </div>

      <div className="af-honeypot" aria-hidden="true" inert>
        <label htmlFor="af-website">Leave this field empty</label>
        <input id="af-website" name="website" type="text" tabIndex={-1} autoComplete="off" value={values.website} onChange={set('website')} />
      </div>
      <button type="submit" className="af-btn af-btn--primary af-btn--wide" disabled={status === 'sending'}>
        {status === 'sending' ? 'Sending…' : <>Request an assessment <span aria-hidden="true">→</span></>}
      </button>
      {!connected && (
        <p className="af-form-note">Not connected yet: nothing is sent until a submission endpoint is configured.</p>
      )}

      <details className="af-privacy">
        <summary>How we use your details</summary>
        <p>Crypton stores this request in a private, encrypted inbox to discuss assessment fit. Requests are removed from the active inbox after 90 days. No marketing signup. Please don’t include sensitive system or customer data.</p>
      </details>
      <div ref={resultRef} tabIndex={-1} className="af-form-result" aria-live="polite">
        {errorList.length > 0 && (
          <div role="alert" className="af-errors">
            <p><strong>Please fix {errorList.length === 1 ? 'one field' : `${errorList.length} fields`}:</strong></p>
            <ul>
              {errorList.map(([id, m]) => <li key={id}><a href={`#af-f-${id}`}>{m}</a></li>)}
            </ul>
          </div>
        )}
        {status === 'not-configured' && (
          <p role="alert" className="af-form-warn">
            <strong>Nothing was sent.</strong> This form isn’t connected to a server yet, so your request wasn’t
            delivered. Your details are still here.
          </p>
        )}
        {status === 'error' && (
          <p role="alert" className="af-form-warn">
            <strong>{failure.code === 429 ? 'Please wait before trying again.' : 'We couldn’t confirm receipt.'}</strong>{' '}
            {failure.code === 429 ? 'Too many requests. Try again later.' : failure.code === 425 ? 'Please wait a moment, then try again.' : failure.code === 410 || failure.code === 409 ? 'Your form session changed or expired. Please submit again.' : 'Your details are still here. Try again to confirm delivery.'}
          </p>
        )}
      </div>
    </form>
  );
}
