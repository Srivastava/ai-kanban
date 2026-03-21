// Single source of truth for Claude token pricing.
// All analytics components must import from here — never define prices inline.
export const PRICING = {
  input:      3.0,   // USD per 1M tokens
  output:     15.0,
  cacheWrite: 3.75,
  cacheRead:  0.30,
} as const;

export function estimateCost(
  input: number,
  output: number,
  cacheWrite = 0,
  cacheRead = 0,
): number {
  return (
    (input      / 1_000_000) * PRICING.input +
    (output     / 1_000_000) * PRICING.output +
    (cacheWrite / 1_000_000) * PRICING.cacheWrite +
    (cacheRead  / 1_000_000) * PRICING.cacheRead
  );
}
