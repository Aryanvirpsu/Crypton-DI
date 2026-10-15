import { MOMENTS, STORY, momentOf } from './story';
import { STEPS, getEntity } from './scenario';

describe('visitor story', () => {
  it('has one plain line per scene, grouped into four moments in order', () => {
    expect(STORY).toHaveLength(STEPS.length);
    expect(MOMENTS.map((m) => m.label)).toEqual(['Enter', 'Follow', 'Discover', 'Act']);
    const order = STORY.map((s) => MOMENTS.findIndex((m) => m.id === s.moment));
    order.forEach((o, i) => { if (i) expect(o).toBeGreaterThanOrEqual(order[i - 1]); });
    MOMENTS.forEach((m) => expect(STORY[m.first].moment).toBe(m.id));
    STEPS.forEach((_, i) => expect(momentOf(i)).toBeDefined());
  });

  it('keeps each line short: one idea per scene', () => {
    STORY.forEach((s) => {
      expect(s.line.split(' ').length).toBeLessThanOrEqual(8);
      if (s.sub) expect(s.sub.length).toBeLessThanOrEqual(110);
    });
  });

  it('uses no internal IDs or jargon in the default story', () => {
    const text = STORY.map((s) => `${s.line} ${s.sub || ''}`).join(' ');
    expect(text).not.toMatch(/\b(GR|DL|ACT|RS|AG|PR|EV|AQ)-|envelope|principal|UNVERIFIABLE|VIOLATED|ENFORCED|manifest/i);
  });

  it('never says the export ran, was blocked, or was a confirmed violation', () => {
    // the one hedged clause ("whether it ran or was stopped") is the point, not a claim
    const text = STORY.map((s) => `${s.line} ${s.sub || ''}`).join(' ').replace(/whether it ran or was stopped/g, '');
    expect(text).not.toMatch(/\b(was|were|got) (sent|exported|blocked|stopped|prevented|denied)\b|violat|breach|leaked/i);
    expect(STORY[6].sub).toMatch(/not whether it ran or was stopped/);
  });

  it('keeps the closing and the pullback line short and free of jargon', () => {
    const last = STORY[STORY.length - 1];
    expect(last.line).toBe('Your agents have boundaries too.');
    expect(last.sub).toBe('Find out whether their actions stay within them.');
    expect(last.pullback.split(' ').length).toBeLessThanOrEqual(8);
    expect(last.pullback).not.toMatch(/\b(violat|breach|leak|exceed|attack)/i);
  });

  it('only links to details that exist', () => {
    STORY.filter((s) => s.detail).forEach((s) => {
      expect(s.detail.id === 'AQ-001' || getEntity(s.detail.id)).toBeTruthy();
    });
  });
});
