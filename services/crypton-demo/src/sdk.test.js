import { crypton, parseJwt, CryptonError } from './sdk';

// Minimal regression protection for SDK
describe('SDK Regression Protection', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
    // mock for navigator.credentials (auth modules)
    Object.defineProperty(global.navigator, 'credentials', {
      value: { get: jest.fn(), create: jest.fn() },
      configurable: true
    });
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  const mockResponse = (data) => {
    global.fetch.mockResolvedValueOnce({
      status: 200,
      json: async () => ({ success: true, data })
    });
  };

  it('login flow invokes correct endpoint and returns token', async () => {
    // 1. Mock the /auth/login/start call
    global.fetch.mockResolvedValueOnce({
      status: 200,
      json: async () => ({
        success: true,
        data: { challenge_id: "123", publicKey: { challenge: "mock", allowCredentials: [] } }
      })
    });

    // 2. Mock navigator.credentials.get
    global.navigator.credentials.get.mockResolvedValueOnce({
      id: "cred-id",
      rawId: new ArrayBuffer(8),
      type: "public-key",
      response: {
        clientDataJSON: new ArrayBuffer(8),
        authenticatorData: new ArrayBuffer(8),
        signature: new ArrayBuffer(8),
        userHandle: new ArrayBuffer(8),
      }
    });

    // 3. Mock the /auth/login/finish call
    global.fetch.mockResolvedValueOnce({
      status: 200,
      json: async () => ({ success: true, data: { token: "mock.jwt.token" } })
    });

    const result = await crypton.auth.login("test@test.local");
    expect(result.token).toBe("mock.jwt.token");
    expect(global.fetch).toHaveBeenCalledTimes(2);
  });

  it('devices list fetches array of devices', async () => {
    mockResponse([{ id: "dev1" }]);
    const items = await crypton.devices.list();
    expect(items.length).toBe(1);
    expect(items[0].id).toBe("dev1");
  });

  it('devices revoke calls post on revoke endpoint', async () => {
    mockResponse({ status: "revoked" });
    await crypton.devices.revoke("dev1");
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining("/devices/dev1/revoke"),
      expect.objectContaining({ method: "POST" })
    );
  });

  it('recovery current fetches active recovery', async () => {
    mockResponse({ request: { id: "req1", status: "pending" } });
    const res = await crypton.recovery.current();
    expect(res.request.id).toBe("req1");
  });

  it('recovery start initializes a request', async () => {
    mockResponse({ id: "newReq", status: "pending", method: "trusted_device", created_at: "", expires_at: "", user_id: "", approved_by_credential_id: null });
    const res = await crypton.recovery.start();
    expect(res.id).toBe("newReq");
  });
});

describe('parseJwt', () => {
  it('returns null for invalid input', () => {
    expect(parseJwt("")).toBeNull();
    expect(parseJwt("no-dots-at-all")).toBeNull();
    expect(parseJwt("one.!!invalid!!.three")).toBeNull();
  });

  it('decodes a valid JWT payload and returns typed fields', () => {
    const raw = { sub: "user-uuid", username: "alice", cred_id: "cred-uuid", iat: 1000, exp: 9999999999 };
    const encoded = btoa(JSON.stringify(raw))
      .replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
    const token = `eyJhbGciOiJIUzI1NiJ9.${encoded}.sig`;
    const result = parseJwt(token);
    expect(result).not.toBeNull();
    expect(result?.sub).toBe("user-uuid");
    expect(result?.username).toBe("alice");
    expect(result?.cred_id).toBe("cred-uuid");
  });
});

describe('CryptonError', () => {
  it('is() correctly identifies CryptonError instances', () => {
    const err = new CryptonError("session_expired", "Session Expired");
    expect(CryptonError.is(err)).toBe(true);
    expect(CryptonError.is(new Error("plain error"))).toBe(false);
    expect(CryptonError.is(null)).toBe(false);
    expect(CryptonError.is("string")).toBe(false);
    expect(CryptonError.is(undefined)).toBe(false);
  });

  it('instanceof works and code + name are set correctly', () => {
    const err = new CryptonError("recovery_pending", "Recovery Pending");
    expect(err instanceof CryptonError).toBe(true);
    expect(err instanceof Error).toBe(true);
    expect(err.code).toBe("recovery_pending");
    expect(err.message).toBe("Recovery Pending");
    expect(err.name).toBe("CryptonError");
  });
});
