import { safeBand, solveLabels } from './mobileLayout';

const stage = { width: 375, top: 60, bottom: 420 };
const tag = (l, t, w = 56, h = 22, p = 4) => ({ l, t, w, h, p });
const box = (r, o) => ({ l: r.l + o.dx, t: r.t + o.dy, w: r.w, h: r.h });
const overlaps = (a, b) => a.l < b.l + b.w && b.l < a.l + a.w && a.t < b.t + b.h && b.t < a.t + a.h;

describe('safeBand', () => {
  it('sits between the header and the story text, with breathing room', () => {
    expect(safeBand({ navBottom: 52, blockTop: 480, stageHeight: 812 })).toEqual({ top: 58, bottom: 468 });
  });
  it('never collapses, however tall the text block is', () => {
    const b = safeBand({ navBottom: 52, blockTop: 90, stageHeight: 568 });
    expect(b.bottom - b.top).toBeGreaterThanOrEqual(120);
  });
  it('never extends past the stage', () => {
    expect(safeBand({ navBottom: 52, blockTop: 5000, stageHeight: 568 }).bottom).toBe(568);
  });
});

describe('solveLabels', () => {
  it('leaves tags that do not touch exactly where they are', () => {
    const rects = [tag(40, 200), tag(240, 300)];
    const out = solveLabels(rects, stage);
    expect(out).toEqual([{ dx: 0, dy: 0, hide: false }, { dx: 0, dy: 0, hide: false }]);
  });

  it('separates two agent tags that sit on top of each other, without hiding either', () => {
    const rects = [tag(100, 200), tag(120, 206)];
    const out = solveLabels(rects, stage);
    expect(out.every((o) => !o.hide)).toBe(true);
    expect(overlaps(box(rects[0], out[0]), box(rects[1], out[1]))).toBe(false);
  });

  it('solves a cluster of three agent tags (SCOUT, RELAY, COURIER) so none overlap', () => {
    const rects = [tag(90, 200), tag(110, 204), tag(130, 208)];
    const out = solveLabels(rects, stage);
    for (let i = 0; i < rects.length; i += 1) {
      for (let j = i + 1; j < rects.length; j += 1) {
        if (!out[i].hide && !out[j].hide) expect(overlaps(box(rects[i], out[i]), box(rects[j], out[j]))).toBe(false);
      }
    }
  });

  it('moves a tag that pokes off the left or right edge back onto the stage', () => {
    const rects = [tag(-20, 200), tag(340, 300)];
    const out = solveLabels(rects, stage);
    expect(box(rects[0], out[0]).l).toBeGreaterThanOrEqual(0);
    expect(box(rects[1], out[1]).l + rects[1].w).toBeLessThanOrEqual(stage.width);
  });

  it('hides a tag whose anchor is not on the stage at all', () => {
    const out = solveLabels([tag(-200, 200), tag(500, 200), tag(100, 10), tag(100, 600)], stage);
    expect(out.map((o) => o.hide)).toEqual([true, true, true, true]);
  });

  it('lets the more important tag keep its place and moves or hides the place name', () => {
    const agent = tag(120, 200, 60, 22, 4);
    const place = tag(110, 204, 150, 12, 1);
    const out = solveLabels([place, agent], stage);
    expect(out[1]).toEqual({ dx: 0, dy: 0, hide: false });
    expect(out[0].hide || !overlaps(box(place, out[0]), box(agent, out[1]))).toBe(true);
  });

  it('is stable: solving twice gives the same answer', () => {
    const rects = [tag(100, 200), tag(120, 206), tag(300, 210, 70, 22, 3)];
    expect(solveLabels(rects, stage)).toEqual(solveLabels(rects, stage));
  });
});
