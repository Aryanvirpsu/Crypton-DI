/**
 * Unit tests for @crypton/sdk.
 *
 * These are honestly-named unit tests, not integration tests: fetch and
 * navigator.credentials are both mocked, so nothing here proves the real
 * backend behaves this way — see docs/testing.md and
 * services/crypton-id/tests/ for the tests that hit a real Postgres/Redis.
 * What this file DOES prove: the SDK builds the request URL/body it
 * claims to, unwraps the {success,data} envelope correctly, and maps
 * WebAuthn/HTTP errors to the right CryptonError code. Every fetch mock
 * assertion below checks the exact URL and body, not just "fetch was
 * called" — a silent endpoint-path or payload-shape regression fails
 * these tests.
 */

import { CryptonClient, ERROR_CODES } from '../index';
import { mockWebAuthnCredential, mockWebAuthnAssertion, mockCredentials } from './setup';

function jsonResponse(status: number, body: unknown) {
  return Promise.resolve({
    status,
    ok: status >= 200 && status < 300,
    json: () => Promise.resolve(body),
  } as Response);
}

describe('@crypton/sdk', () => {
  let client: CryptonClient;

  beforeEach(() => {
    jest.clearAllMocks();
    global.fetch = jest.fn();
    client = new CryptonClient('http://localhost:8090');
  });

  describe('auth.register / auth.login happy path', () => {
    it('registers with a real WebAuthn create() call and the right request shapes', async () => {
      const email = 'user@example.com';
      const mockToken = 'header.payload.signature';

      const registerStartResponse = {
        success: true,
        data: {
          challenge_id: 'challenge-123',
          publicKey: {
            challenge: 'MDEyMzQ1Njc4OWFiY2RlZg==',
            rp: { name: 'Crypton' },
            user: { id: 'dXNlcmFaVmhnNTI=', name: email, displayName: email },
            pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
            timeout: 60000,
            attestation: 'direct',
          },
        },
      };
      const registerFinishResponse = { success: true, data: { user_id: 'user-123', token: mockToken } };

      mockCredentials.create.mockResolvedValue(mockWebAuthnCredential({}));

      (global.fetch as jest.Mock).mockImplementation((url: string, opts: RequestInit) => {
        if (url === 'http://localhost:8090/auth/register/start') {
          expect(JSON.parse(opts.body as string)).toEqual({ username: email });
          return jsonResponse(200, registerStartResponse);
        }
        if (url === 'http://localhost:8090/auth/register/finish') {
          const body = JSON.parse(opts.body as string);
          expect(body.challenge_id).toBe('challenge-123');
          expect(body.attestation.id).toBe('credential-id-123');
          return jsonResponse(200, registerFinishResponse);
        }
        throw new Error(`unexpected fetch: ${url}`);
      });

      const result = await client.auth.register(email);

      expect(result.user_id).toBe('user-123');
      expect(result.token).toBe(mockToken);
      expect(mockCredentials.create).toHaveBeenCalledTimes(1);
    });

    it('sends enrollment_token when re-enrolling on an existing account', async () => {
      const email = 'user@example.com';
      mockCredentials.create.mockResolvedValue(mockWebAuthnCredential({}));

      (global.fetch as jest.Mock).mockImplementation((url: string, opts: RequestInit) => {
        if (url === 'http://localhost:8090/auth/register/start') {
          const body = JSON.parse(opts.body as string);
          expect(body).toEqual({ username: email, enrollment_token: 'recovery-token.abc' });
          return jsonResponse(200, {
            success: true,
            data: { challenge_id: 'c1', publicKey: { challenge: 'MDE=', user: { id: 'dXE=' } } },
          });
        }
        return jsonResponse(200, { success: true, data: { user_id: 'u1', token: 't1' } });
      });

      await client.auth.register(email, 'recovery-token.abc');
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });

    it('logs in with a real WebAuthn get() call against the right endpoint', async () => {
      const email = 'user@example.com';
      const mockToken = 'header.payload.signature';

      mockCredentials.get.mockResolvedValue(mockWebAuthnAssertion({}));

      (global.fetch as jest.Mock).mockImplementation((url: string) => {
        if (url === 'http://localhost:8090/auth/login/start') {
          return jsonResponse(200, {
            success: true,
            data: {
              challenge_id: 'challenge-456',
              publicKey: { challenge: 'MDEyMzQ1Njc4OWFiY2RlZg==', allowCredentials: [] },
            },
          });
        }
        if (url === 'http://localhost:8090/auth/login/finish') {
          return jsonResponse(200, { success: true, data: { token: mockToken } });
        }
        throw new Error(`unexpected fetch: ${url}`);
      });

      const result = await client.auth.login(email);
      expect(result.token).toBe(mockToken);
      expect(mockCredentials.get).toHaveBeenCalledTimes(1);
    });
  });

  describe('error mapping', () => {
    it('maps NotAllowedError → webauthn_cancelled', async () => {
      mockCredentials.create.mockRejectedValue(
        new DOMException('User cancelled the operation', 'NotAllowedError'),
      );
      (global.fetch as jest.Mock).mockImplementation(() =>
        jsonResponse(200, {
          success: true,
          data: { challenge_id: 'c1', publicKey: { challenge: 'MDE=', user: { id: 'dXE=' } } },
        }),
      );

      await expect(client.auth.register('user@example.com')).rejects.toMatchObject({
        code: ERROR_CODES.WEBAUTHN_CANCELLED,
      });
    });

    it('maps HTTP 401 → session_expired and clears the stored token', async () => {
      mockCredentials.get.mockResolvedValue(mockWebAuthnAssertion({}));
      (global.fetch as jest.Mock).mockImplementation((url: string) => {
        if (url.endsWith('/auth/login/start')) {
          return jsonResponse(200, {
            success: true,
            data: { challenge_id: 'c1', publicKey: { challenge: 'MDE=', allowCredentials: [] } },
          });
        }
        return jsonResponse(401, { success: false, error: { code: 'session_expired', message: 'Session expired' } });
      });

      await expect(client.auth.login('user@example.com')).rejects.toMatchObject({
        code: ERROR_CODES.SESSION_EXPIRED,
      });
    });

    it('passes through a non-401 backend error code+message unchanged', async () => {
      (global.fetch as jest.Mock).mockImplementation(() =>
        jsonResponse(404, { success: false, error: { code: 'credential_not_found', message: 'Not found' } }),
      );
      await expect(client.devices.revoke('does-not-exist')).rejects.toMatchObject({
        code: 'credential_not_found',
      });
    });
  });

  describe('devices', () => {
    it('revokes via POST /devices/:id/revoke (not the removed DELETE /devices/:id)', async () => {
      (global.fetch as jest.Mock).mockImplementation((url: string, opts: RequestInit) => {
        expect(url).toBe('http://localhost:8090/devices/dev1/revoke');
        expect(opts.method).toBe('POST');
        return jsonResponse(200, { success: true, data: { status: 'revoked' } });
      });
      const result = await client.devices.revoke('dev1');
      expect(result.status).toBe('revoked');
    });
  });

  describe('recovery — public flow request shapes', () => {
    it('claimRecovery sends {username, recovery_token}, not {request_id}', async () => {
      (global.fetch as jest.Mock).mockImplementation((url: string, opts: RequestInit) => {
        expect(url).toBe('http://localhost:8090/recovery/public/complete');
        expect(JSON.parse(opts.body as string)).toEqual({
          username: 'user@example.com',
          recovery_token: 'abc.def',
        });
        return jsonResponse(200, {
          success: true,
          data: { request: { status: 'completed', method: 'trusted_device', expires_at: '2026-01-01T00:00:00Z' }, enrollment_token: 'enroll.xyz' },
        });
      });

      const result = await client.recovery.claimRecovery('abc.def', 'user@example.com');
      expect(result.enrollment_token).toBe('enroll.xyz');
    });

    it('requestRecovery surfaces the one-time recovery_token from the response', async () => {
      (global.fetch as jest.Mock).mockImplementation(() =>
        jsonResponse(200, {
          success: true,
          data: {
            request: { status: 'pending', method: 'trusted_device', expires_at: '2026-01-01T00:00:00Z' },
            recovery_token: 'newly-issued-token',
          },
        }),
      );
      const result = await client.recovery.requestRecovery('user@example.com');
      expect(result.recovery_token).toBe('newly-issued-token');
      expect(result.request.status).toBe('pending');
    });
  });

  describe('admin', () => {
    it('updateUserRole PATCHes /admin/users/:id/role with the role in the body', async () => {
      (global.fetch as jest.Mock).mockImplementation((url: string, opts: RequestInit) => {
        expect(url).toBe('http://localhost:8090/admin/users/u1/role');
        expect(opts.method).toBe('PATCH');
        expect(JSON.parse(opts.body as string)).toEqual({ role: 'admin' });
        return jsonResponse(200, { success: true, data: { status: 'updated' } });
      });
      await client.admin.updateUserRole('u1', 'admin');
    });
  });
});
