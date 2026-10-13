/**
 * crypton-gateway rejects any request without a fresh x-nonce and a current
 * x-timestamp (unix seconds). The SDK must send both on every request, with a
 * different nonce each time, or no client built on it can reach the gateway.
 */

import { CryptonClient } from '../index';

function ok(body: unknown) {
  return Promise.resolve({
    status: 200,
    ok: true,
    json: () => Promise.resolve(body),
  } as Response);
}

describe('gateway replay-protection headers', () => {
  beforeEach(() => {
    global.fetch = jest.fn(() => ok({ success: true, data: {} })) as unknown as typeof fetch;
  });

  it('sends a fresh nonce and a current timestamp on every request', async () => {
    const client = new CryptonClient('http://localhost:8090');
    const before = Math.floor(Date.now() / 1000);
    await client.me.get();
    await client.me.get();
    const after = Math.floor(Date.now() / 1000);

    const calls = (global.fetch as jest.Mock).mock.calls;
    expect(calls).toHaveLength(2);
    const [h1, h2] = calls.map(([, opts]) => opts.headers as Headers);

    for (const h of [h1, h2]) {
      expect(h.get('x-nonce')).toMatch(/^[A-Za-z0-9._:-]{1,128}$/);
      const ts = Number(h.get('x-timestamp'));
      expect(Number.isInteger(ts)).toBe(true);
      expect(ts).toBeGreaterThanOrEqual(before);
      expect(ts).toBeLessThanOrEqual(after);
    }
    expect(h1.get('x-nonce')).not.toEqual(h2.get('x-nonce'));
  });
});
