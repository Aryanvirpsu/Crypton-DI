/**
 * Jest setup file — runs once before the whole suite (setupFilesAfterEnv).
 *
 * navigator.credentials is mocked per-test in __tests__/setup.ts instead of
 * here — having it in both files meant whichever ran last silently won,
 * which is exactly the kind of thing that hides a real bug.
 */

export {}; // make this a module (isolatedModules requires it)

// jsdom's environment doesn't provide TextEncoder/TextDecoder, which the
// `Response` polyfill used in tests needs. Both are already declared by the
// "DOM" lib in tsconfig, so we only need the runtime assignment here.
if (typeof (globalThis as { TextEncoder?: unknown }).TextEncoder === 'undefined') {
  const util = require('util');
  (globalThis as { TextEncoder?: unknown }).TextEncoder = util.TextEncoder;
  (globalThis as { TextDecoder?: unknown }).TextDecoder = util.TextDecoder;
}
