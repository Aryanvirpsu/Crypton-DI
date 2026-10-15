import { PROBLEM_FLOORS, SEAM_X, buildFloors, packetProgress, workerAt } from './ecosystem';
import { SLAB } from './choreography';

const floors = buildFloors();

describe('far floors', () => {
  it('only a minority carry the boundary signal, and most are plainly at work', () => {
    const problem = floors.filter((f) => f.problem);
    expect(problem.map((f) => f.i)).toEqual(PROBLEM_FLOORS);
    expect(problem.length / floors.length).toBeLessThanOrEqual(0.4);
    expect(floors.length - problem.length).toBeGreaterThanOrEqual(6);
  });

  it('varies the work areas, activity levels and sizes', () => {
    expect(new Set(floors.map((f) => f.variant)).size).toBe(4);
    const busy = floors.map((f) => f.busy);
    expect(Math.max(...busy) - Math.min(...busy)).toBeGreaterThan(0.6);
    expect(new Set(floors.map((f) => f.s)).size).toBeGreaterThan(4);
    expect(new Set(floors.map((f) => f.phase)).size).toBe(floors.length);
  });

  it('keeps every worker on its own slab, at every moment', () => {
    floors.forEach((f) => {
      for (let t = 0; t < 120; t += 3.7) {
        for (let k = 0; k < f.workers; k += 1) {
          const [x, , z] = workerAt(f, k, t);
          expect(Math.abs(x)).toBeLessThan(SLAB.width / 2 - 0.4);
          expect(Math.abs(z)).toBeLessThan(SLAB.depth / 2 - 0.4);
        }
      }
    });
  });

  it('keeps the furniture on the slab and clear of the seam', () => {
    floors.forEach((f) => f.furniture.forEach(({ p, size }) => {
      expect(p[0] + size[0] / 2).toBeLessThan(SEAM_X - 0.2);
      expect(Math.abs(p[2]) + size[2] / 2).toBeLessThanOrEqual(SLAB.depth / 2);
    }));
  });

  it('a request travels toward the seam and waits there: progress is monotonic to 1 and never exceeds it', () => {
    const f = floors[PROBLEM_FLOORS[0]];
    let prev = -1;
    for (let u = 0; u < 3.2; u += 0.01) {
      const p = packetProgress(f, u);
      expect(p).toBeLessThanOrEqual(1);
      expect(p).toBeGreaterThanOrEqual(0);
      if (p < 1 && prev < 1 && p >= prev) prev = p;
    }
    // it does reach the seam and stays there for part of every cycle
    const samples = Array.from({ length: 400 }, (_, i) => packetProgress(f, i * 0.05));
    expect(samples.some((p) => p === 1)).toBe(true);
    expect(samples.some((p) => p < 0.05)).toBe(true);
  });
});
