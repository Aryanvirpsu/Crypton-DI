/**
 * Jest setup file — runs before all tests
 */

// Mock navigator.credentials before tests run
const mockCredentialsAPI = {
  create: jest.fn(),
  get: jest.fn(),
};

Object.defineProperty(navigator, 'credentials', {
  value: mockCredentialsAPI,
  writable: true,
  configurable: true,
});

// jsdom provides TextEncoder/TextDecoder for Response support
declare global {
  var TextEncoder: any;
  var TextDecoder: any;
}
if (typeof global.TextEncoder === 'undefined') {
  const util = require('util');
  global.TextEncoder = util.TextEncoder;
  global.TextDecoder = util.TextDecoder;
}
