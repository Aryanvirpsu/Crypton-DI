/**
 * Integration tests for @crypton/sdk
 * Tests auth flows, WebAuthn error handling, and HTTP error mapping
 */

import { CryptonClient, CryptonError, ERROR_CODES } from '../index';
import { mockWebAuthnCredential, mockWebAuthnAssertion, mockCredentials } from './setup';

describe('@crypton/sdk Integration Tests', () => {
  let client: CryptonClient;

  beforeEach(() => {
    jest.clearAllMocks();
    global.fetch = jest.fn();
    client = new CryptonClient('http://localhost:3000');
  });

  describe('Test 1: Auth success flow with mocked WebAuthn', () => {
    it('should successfully register and login with mocked navigator.credentials', async () => {
      const email = 'user@example.com';
      const mockToken = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoiMTIzIiwiaWF0IjoxNjAwMDAwMDAwfQ.mock';

      const registerStartResponse = {
        success: true,
        data: {
          challenge_id: 'challenge-123',
          publicKey: {
            challenge: 'MDEyMzQ1Njc4OWFiY2RlZg==',
            rp: { name: 'Crypton' },
            user: {
              id: 'dXNlcmFaVmhnNTI=',
              name: email,
              displayName: email,
            },
            pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
            timeout: 60000,
            attestation: 'direct',
          },
        },
      };

      const registerFinishResponse = {
        success: true,
        data: {
          user_id: 'user-123',
          token: mockToken,
        },
      };

      // Mock navigator.credentials.create for registration
      const mockCredential = mockWebAuthnCredential({});
      mockCredentials.create.mockResolvedValue(mockCredential);

      // Mock fetch for both register calls - return Promise-wrapped mock Response
      (global.fetch as jest.Mock).mockImplementation((url: string) => {
        const body = url.includes('/auth/register/start')
          ? registerStartResponse
          : registerFinishResponse;

        return Promise.resolve({
          status: 200,
          ok: true,
          json: () => Promise.resolve(body),
        } as any);
      });

      // Execute registration
      const registerResult = await client.auth.register(email);

      expect(registerResult.user_id).toBe('user-123');
      expect(registerResult.token).toBe(mockToken);
      expect(mockCredentials.create).toHaveBeenCalled();

      // Reset mocks for login test
      jest.clearAllMocks();

      const loginStartResponse = {
        success: true,
        data: {
          challenge_id: 'challenge-456',
          publicKey: {
            challenge: 'MDEyMzQ1Njc4OWFiY2RlZg==',
            timeout: 60000,
            rpId: 'localhost',
            allowCredentials: [{ type: 'public-key', id: 'Y3JlZGVudGlhbC1pZA==' }],
            userVerification: 'preferred',
          },
        },
      };

      const loginFinishResponse = {
        success: true,
        data: { token: mockToken },
      };

      // Mock navigator.credentials.get for login
      const mockAssertion = mockWebAuthnAssertion({});
      mockCredentials.get.mockResolvedValue(mockAssertion);

      // Mock fetch for both login calls
      (global.fetch as jest.Mock).mockImplementation((url: string) => {
        const body = url.includes('/auth/login/start')
          ? loginStartResponse
          : loginFinishResponse;

        return Promise.resolve({
          status: 200,
          ok: true,
          json: () => Promise.resolve(body),
        } as any);
      });

      // Execute login
      const loginResult = await client.auth.login(email);

      expect(loginResult.token).toBe(mockToken);
      expect(mockCredentials.get).toHaveBeenCalled();
    });
  });

  describe('Test 2: WebAuthn error mapping - NotAllowedError → webauthn_cancelled', () => {
    it('should map NotAllowedError DOMException to CryptonError with webauthn_cancelled code', async () => {
      const email = 'user@example.com';

      const registerStartResponse = {
        success: true,
        data: {
          challenge_id: 'challenge-123',
          publicKey: {
            challenge: 'MDEyMzQ1Njc4OWFiY2RlZg==',
            rp: { name: 'Crypton' },
            user: {
              id: 'dXNlcmFaVmhnNTI=',
              name: email,
              displayName: email,
            },
            pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
            timeout: 60000,
            attestation: 'direct',
          },
        },
      };

      // Mock WebAuthn to throw NotAllowedError (user cancels)
      mockCredentials.create.mockRejectedValue(
        new DOMException('User cancelled the operation', 'NotAllowedError')
      );

      (global.fetch as jest.Mock).mockImplementation((url: string) => {
        return Promise.resolve({
          status: 200,
          ok: true,
          json: () => Promise.resolve(registerStartResponse),
        } as any);
      });

      // Register should throw CryptonError with proper code
      try {
        await client.auth.register(email);
        throw new Error('Should have thrown CryptonError');
      } catch (err: any) {
        expect(CryptonError.is(err)).toBe(true);
        expect(err.code).toBe(ERROR_CODES.WEBAUTHN_CANCELLED);
        expect(err.message).toContain('cancelled');
      }
    });
  });

  describe('Test 3: HTTP 401 error mapping → session_expired', () => {
    it('should map HTTP 401 response to CryptonError with session_expired code', async () => {
      const email = 'user@example.com';

      const loginStartResponse = {
        success: true,
        data: {
          challenge_id: 'challenge-456',
          publicKey: {
            challenge: 'MDEyMzQ1Njc4OWFiY2RlZg==',
            timeout: 60000,
            rpId: 'localhost',
            allowCredentials: [],
            userVerification: 'preferred',
          },
        },
      };

      // Mock WebAuthn to succeed
      const mockAssertion = mockWebAuthnAssertion({});
      mockCredentials.get.mockResolvedValue(mockAssertion);

      (global.fetch as jest.Mock).mockImplementation((url: string) => {
        if (url.includes('/auth/login/start')) {
          return Promise.resolve({
            status: 200,
            ok: true,
            json: () => Promise.resolve(loginStartResponse),
          } as any);
        }
        if (url.includes('/auth/login/finish')) {
          // Return 401 Unauthorized
          return Promise.resolve({
            status: 401,
            ok: false,
            json: () => Promise.resolve({
              success: false,
              error: { code: 'session_expired', message: 'Session expired' },
            }),
          } as any);
        }
        return Promise.resolve({
          status: 404,
          ok: false,
          json: () => Promise.resolve({ success: false }),
        } as any);
      });

      // Login should throw CryptonError with session_expired
      try {
        await client.auth.login(email);
        throw new Error('Should have thrown CryptonError');
      } catch (err: any) {
        expect(CryptonError.is(err)).toBe(true);
        expect(err.code).toBe(ERROR_CODES.SESSION_EXPIRED);
        expect(err.message).toBeDefined();
      }
    });
  });
});
