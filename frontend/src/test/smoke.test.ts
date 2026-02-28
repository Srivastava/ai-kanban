import { describe, it, expect } from 'vitest';

describe('test infrastructure', () => {
  it('vitest runs', () => {
    expect(1 + 1).toBe(2);
  });

  it('MSW server is configured', async () => {
    const response = await fetch('http://localhost:3001/api/tasks');
    expect(response.ok).toBe(true);
    const data = await response.json();
    expect(Array.isArray(data)).toBe(true);
    expect(data[0].id).toBe('task-123');
  });
});
