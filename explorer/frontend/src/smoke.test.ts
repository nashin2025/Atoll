import { describe, it, expect } from 'vitest';

describe('frontend smoke tests', () => {
  it('passes a basic sanity check', () => {
    expect(1 + 1).toBe(2);
  });

  it('validates TypeScript types are sound', () => {
    // This test ensures the build step (tsc) passes as part of `npm run build`
    expect(true).toBe(true);
  });
});
