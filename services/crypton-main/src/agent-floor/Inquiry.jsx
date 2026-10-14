import React, { useRef, useState } from 'react';
import { INITIAL_VALUES, INTEREST_OPTIONS, LIMITS, validateInquiry } from './inquiryModel';

/**
 * Prototype of the assessment inquiry. It validates and previews locally. It does NOT send
 * anything: there is no verified submission endpoint, storage or notification yet, so the
 * form says so and never reports a successful submission.
 */
export default function Inquiry() {
  const [values, setValues] = useState(INITIAL_VALUES);
  const [errors, setErrors] = useState({});
  const [preview, setPreview] = useState(false);
  const summaryRef = useRef(null);

  const set = (field) => (e) => {
    const value = e.target.type === 'checkbox' ? e.target.checked : e.target.value;
    setValues((v) => ({ ...v, [field]: value }));
    if (errors[field]) setErrors((er) => ({ ...er, [field]: undefined }));
    setPreview(false);
  };

  const onSubmit = (e) => {
    e.preventDefault();
    const found = validateInquiry(values);
    setErrors(found);
    const ok = Object.keys(found).length === 0;
    setPreview(ok);
    // Move focus to the error summary (or the preview) so keyboard and screen-reader users land on the result.
    setTimeout(() => summaryRef.current && summaryRef.current.focus(), 0);
  };

  const errorList = Object.entries(errors).filter(([, m]) => m);
  const field = (id) => ({
    id: `af-f-${id}`,
    'aria-invalid': errors[id] ? 'true' : undefined,
    'aria-describedby': errors[id] ? `af-e-${id}` : undefined,
  });
  const err = (id) => errors[id] && <p className="af-field-error" id={`af-e-${id}`}>{errors[id]}</p>;

  return (
    <section id="af-inquiry" className="af-section" aria-labelledby="af-inquiry-h">
      <h2 id="af-inquiry-h">Who was actually behind the action?</h2>
      <p className="af-lead">
        Tell us what you would like assessed. Principal continuity is a research-preview test family: we
        look at whether an action can be traced back to the principal that actually authorised it.
      </p>
      <p className="af-banner" role="note">
        <strong>Prototype, nothing is sent.</strong> This form is not connected to a server yet. Checking
        your answers shows a local preview only, and no inquiry is stored or delivered. A public contact
        address has not been published.
      </p>

      <form className="af-form" onSubmit={onSubmit} noValidate>
        <div className="af-field">
          <label htmlFor="af-f-name">Name</label>
          <input type="text" autoComplete="name" value={values.name} onChange={set('name')} maxLength={LIMITS.name + 20} required {...field('name')} />
          {err('name')}
        </div>
        <div className="af-field">
          <label htmlFor="af-f-email">Email</label>
          <input type="email" autoComplete="email" value={values.email} onChange={set('email')} required {...field('email')} />
          {err('email')}
        </div>
        <div className="af-field">
          <label htmlFor="af-f-organization">Organization or project <span className="af-muted">(optional)</span></label>
          <input type="text" autoComplete="organization" value={values.organization} onChange={set('organization')} {...field('organization')} />
          {err('organization')}
        </div>
        <div className="af-field">
          <label htmlFor="af-f-interest">Assessment interest</label>
          <select value={values.interest} onChange={set('interest')} {...field('interest')}>
            {INTEREST_OPTIONS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
          </select>
          {err('interest')}
        </div>
        <div className="af-field">
          <label htmlFor="af-f-context">High-level architecture context <span className="af-muted">(optional)</span></label>
          <textarea rows={4} value={values.context} onChange={set('context')} {...field('context')} />
          <p className="af-hint">A few sentences is enough. Do not include credentials, keys, customer data or anything confidential.</p>
          {err('context')}
        </div>
        <div className="af-field af-field--check">
          <input type="checkbox" checked={values.consent} onChange={set('consent')} {...field('consent')} />
          <label htmlFor="af-f-consent">I agree to be contacted about this inquiry.</label>
          {err('consent')}
        </div>

        <button type="submit" className="af-btn af-btn--primary">Check and preview (not sent)</button>

        <div ref={summaryRef} tabIndex={-1} className="af-form-result" aria-live="polite">
          {errorList.length > 0 && (
            <div role="alert" className="af-errors">
              <p><strong>Please fix {errorList.length === 1 ? 'one field' : `${errorList.length} fields`}:</strong></p>
              <ul>
                {errorList.map(([id, m]) => <li key={id}><a href={`#af-f-${id}`}>{m}</a></li>)}
              </ul>
            </div>
          )}
          {preview && (
            <div className="af-preview">
              <p><strong>Not sent.</strong> Your answers are valid, but this prototype has no way to deliver them. Preview:</p>
              <dl>
                <dt>Name</dt><dd>{values.name.trim()}</dd>
                <dt>Email</dt><dd>{values.email.trim()}</dd>
                <dt>Organization</dt><dd>{values.organization.trim() || 'Not given'}</dd>
                <dt>Interest</dt><dd>{(INTEREST_OPTIONS.find((o) => o.value === values.interest) || {}).label}</dd>
                <dt>Context</dt><dd>{values.context.trim() || 'Not given'}</dd>
              </dl>
            </div>
          )}
        </div>
      </form>
    </section>
  );
}
