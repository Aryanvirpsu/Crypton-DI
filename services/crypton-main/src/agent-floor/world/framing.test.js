import { STEPS } from '../scenario';
import { SLAB } from './choreography';
import { FRAMING, fitFrame, framingPoints } from './framing';

const VIEWPORTS = [
  { name: '320x568', width: 320, height: 568, safe: { top: 58, bottom: 250 } },
  { name: '375x667', width: 375, height: 667, safe: { top: 58, bottom: 330 } },
  { name: '393x852', width: 393, height: 852, safe: { top: 60, bottom: 470 } },
  { name: '430x932', width: 430, height: 932, safe: { top: 62, bottom: 520 } },
];

const fit = (step, vp, view = 'floor') => {
  const defaultView = STEPS[step].defaultView;
  const points = framingPoints(step, view, defaultView);
  return { points, ...fitFrame({ points, step, view, defaultView, width: vp.width, height: vp.height, safe: vp.safe }) };
};

describe('framing points', () => {
  it('names a cast for every scene except the final pullback', () => {
    expect(FRAMING).toHaveLength(STEPS.length);
    STEPS.forEach((s, step) => {
      const pts = framingPoints(step, s.defaultView, s.defaultView);
      if (step === STEPS.length - 1) expect(pts).toBeNull();
      else {
        expect(pts.length).toBeGreaterThanOrEqual(4);
        pts.forEach((p) => { expect(p).toHaveLength(3); p.forEach((n) => expect(Number.isFinite(n)).toBe(true)); });
      }
    });
  });

  it('keeps every point on or above the floor slab and inside the world', () => {
    STEPS.forEach((s, step) => {
      (framingPoints(step, s.defaultView, s.defaultView) || []).forEach(([x, y, z]) => {
        expect(y).toBeGreaterThanOrEqual(0);
        expect(Math.abs(z)).toBeLessThanOrEqual(SLAB.depth);
        expect(Math.abs(x)).toBeLessThan(14);
      });
    });
  });

  it('uses the whole-floor cast when a scene is shown in its other view', () => {
    const all = framingPoints(1, 'atlas', 'floor');
    expect(all.length).toBeGreaterThan(framingPoints(1, 'floor', 'floor').length);
  });
});

describe('fitFrame', () => {
  VIEWPORTS.forEach((vp) => {
    it(`fits every scene inside the safe band at ${vp.name}`, () => {
      STEPS.slice(0, -1).forEach((_, step) => {
        const { bbox, scale } = fit(step, vp);
        const padX = 44;
        // unless it hit the farthest allowed camera, the cast fits the width and the band
        if (scale < 3.39) {
          expect(bbox.maxX - bbox.minX).toBeLessThanOrEqual(vp.width - 2 * padX + 0.5);
          expect(bbox.maxY - bbox.minY).toBeLessThanOrEqual(vp.safe.bottom - vp.safe.top - 2 * 18 + 0.5);
        }
        expect(bbox.behind).toBe(false);
      });
    });

    it(`centres the cast in the safe band at ${vp.name}`, () => {
      STEPS.slice(0, -1).forEach((_, step) => {
        const { bbox, shift } = fit(step, vp);
        const cx = (bbox.minX + bbox.maxX) / 2 + shift[0];
        const cy = (bbox.minY + bbox.maxY) / 2 + shift[1];
        expect(cx).toBeCloseTo(vp.width / 2, 3);
        expect(cy).toBeCloseTo((vp.safe.top + vp.safe.bottom) / 2, 3);
      });
    });
  });

  it('never comes closer than the closest allowed camera, even for a tiny subject', () => {
    const vp = VIEWPORTS[2];
    const { scale } = fitFrame({ points: [[-0.6, 1, 0.4], [-0.5, 1, 0.4]], step: 3, view: 'floor', defaultView: 'floor', width: vp.width, height: vp.height, safe: vp.safe });
    expect(scale).toBeCloseTo(0.9, 5);
  });

  it('pulls back at least as far on a narrower screen, all else equal', () => {
    const safe = { top: 60, bottom: 470 };
    STEPS.slice(0, -1).forEach((_, step) => {
      const narrow = fit(step, { width: 320, height: 852, safe });
      const wide = fit(step, { width: 430, height: 852, safe });
      expect(narrow.scale).toBeGreaterThanOrEqual(wide.scale - 1e-6);
    });
  });

  it('pulls back at least as far when the safe band is shorter, all else equal', () => {
    STEPS.slice(0, -1).forEach((_, step) => {
      const tall = fit(step, { width: 393, height: 852, safe: { top: 60, bottom: 520 } });
      const short = fit(step, { width: 393, height: 852, safe: { top: 60, bottom: 300 } });
      expect(short.scale).toBeGreaterThanOrEqual(tall.scale - 1e-6);
    });
  });

  it('is deterministic', () => {
    expect(fit(4, VIEWPORTS[1]).scale).toBe(fit(4, VIEWPORTS[1]).scale);
  });
});
