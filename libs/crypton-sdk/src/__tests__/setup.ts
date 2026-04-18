/**
 * Test setup — mocks for WebAuthn and fetch
 */

export const mockCredentials = {
  create: jest.fn(),
  get: jest.fn(),
};

// Mock navigator.credentials
beforeEach(() => {
  Object.defineProperty(navigator, 'credentials', {
    value: mockCredentials,
    writable: true,
    configurable: true,
  });
  jest.clearAllMocks();
});

/**
 * Helper to mock a successful WebAuthn credential creation
 */
export function mockWebAuthnCredential(options: {
  id?: string;
  rawId?: ArrayBuffer;
  type?: string;
  clientDataJSON?: ArrayBuffer;
  attestationObject?: ArrayBuffer;
}) {
  const {
    id = 'credential-id-123',
    rawId = new ArrayBuffer(32),
    type = 'public-key',
    clientDataJSON = new ArrayBuffer(256),
    attestationObject = new ArrayBuffer(512),
  } = options;

  const credential: any = {
    id,
    rawId,
    type,
    response: {
      clientDataJSON,
      attestationObject,
    },
  };

  return credential;
}

/**
 * Helper to mock a successful WebAuthn assertion
 */
export function mockWebAuthnAssertion(options: {
  id?: string;
  rawId?: ArrayBuffer;
  type?: string;
  clientDataJSON?: ArrayBuffer;
  authenticatorData?: ArrayBuffer;
  signature?: ArrayBuffer;
  userHandle?: ArrayBuffer;
}) {
  const {
    id = 'credential-id-123',
    rawId = new ArrayBuffer(32),
    type = 'public-key',
    clientDataJSON = new ArrayBuffer(256),
    authenticatorData = new ArrayBuffer(64),
    signature = new ArrayBuffer(128),
    userHandle = null,
  } = options;

  const assertion: any = {
    id,
    rawId,
    type,
    response: {
      clientDataJSON,
      authenticatorData,
      signature,
      userHandle,
    },
  };

  return assertion;
}
