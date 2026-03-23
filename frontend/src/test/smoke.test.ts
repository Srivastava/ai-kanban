import { describe, it, expect } from 'vitest';
import { server } from './msw/server';

describe('test infrastructure', () => {
  it('vitest runs', () => {
    expect(1 + 1).toBe(2);
  });

  it('MSW server is configured', () => {
    // Verify the MSW server object exists and has the expected shape
    expect(server).toBeDefined();
    expect(typeof server.listen).toBe('function');
    expect(typeof server.resetHandlers).toBe('function');
    expect(typeof server.close).toBe('function');
  });
});
