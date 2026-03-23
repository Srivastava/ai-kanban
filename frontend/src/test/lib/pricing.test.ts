import { describe, it, expect } from 'vitest';
import { PRICING, estimateCost } from '@/lib/pricing';

describe('PRICING constants', () => {
  it('has the expected input token price per 1M tokens', () => {
    expect(PRICING.input).toBe(3.0);
  });

  it('has the expected output token price per 1M tokens', () => {
    expect(PRICING.output).toBe(15.0);
  });

  it('has the expected cache write price per 1M tokens', () => {
    expect(PRICING.cacheWrite).toBe(3.75);
  });

  it('has the expected cache read price per 1M tokens', () => {
    expect(PRICING.cacheRead).toBe(0.30);
  });
});

describe('estimateCost', () => {
  it('returns 0 for all zero tokens', () => {
    expect(estimateCost(0, 0)).toBe(0);
  });

  it('calculates cost for input tokens only', () => {
    // 1M input tokens at $3.0/M = $3.00
    expect(estimateCost(1_000_000, 0)).toBeCloseTo(3.0);
  });

  it('calculates cost for output tokens only', () => {
    // 1M output tokens at $15.0/M = $15.00
    expect(estimateCost(0, 1_000_000)).toBeCloseTo(15.0);
  });

  it('calculates combined input and output cost', () => {
    // 500k input ($1.50) + 200k output ($3.00) = $4.50
    const cost = estimateCost(500_000, 200_000);
    expect(cost).toBeCloseTo(4.5);
  });

  it('uses 0 as default for cacheWrite and cacheRead', () => {
    const withDefaults = estimateCost(100_000, 50_000);
    const withExplicitZeros = estimateCost(100_000, 50_000, 0, 0);
    expect(withDefaults).toBe(withExplicitZeros);
  });

  it('includes cache write tokens in cost', () => {
    // 1M cache write tokens at $3.75/M
    const withCache = estimateCost(0, 0, 1_000_000, 0);
    expect(withCache).toBeCloseTo(3.75);
  });

  it('includes cache read tokens in cost', () => {
    // 1M cache read tokens at $0.30/M
    const withCacheRead = estimateCost(0, 0, 0, 1_000_000);
    expect(withCacheRead).toBeCloseTo(0.30);
  });

  it('calculates cost correctly for all four token types', () => {
    const input = 1_000_000;       // $3.00
    const output = 1_000_000;      // $15.00
    const cacheWrite = 1_000_000;  // $3.75
    const cacheRead = 1_000_000;   // $0.30
    const expected = 3.0 + 15.0 + 3.75 + 0.30; // $22.05
    expect(estimateCost(input, output, cacheWrite, cacheRead)).toBeCloseTo(expected);
  });

  it('handles small token counts with precision', () => {
    // 1000 input tokens = 1000/1_000_000 * $3.0 = $0.003
    const cost = estimateCost(1000, 0);
    expect(cost).toBeCloseTo(0.003, 6);
  });

  it('cache read is cheaper than input tokens', () => {
    const inputCost = estimateCost(1_000_000, 0, 0, 0);
    const cacheReadCost = estimateCost(0, 0, 0, 1_000_000);
    expect(cacheReadCost).toBeLessThan(inputCost);
  });
});
