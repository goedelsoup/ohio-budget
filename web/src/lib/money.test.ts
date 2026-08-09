import { describe, expect, it } from 'vitest';
import { compact, delta, dollars, exact, percent } from './money.ts';

describe('dollars', () => {
  it('renders whole dollars from cents with grouping', () => {
    expect(dollars(845_759_877_200)).toBe('$8,457,598,772');
  });

  it('renders zero as zero, not as absent', () => {
    expect(dollars(0)).toBe('$0');
  });
});

describe('exact', () => {
  it('keeps the cents where the cents are the point', () => {
    // The one computed gap in the corpus turns on a figure that is not a round dollar.
    expect(exact(797_500_359_689)).toBe('$7,975,003,596.89');
  });
});

describe('compact', () => {
  it('abbreviates billions and millions', () => {
    expect(compact(845_759_877_200)).toBe('$8.46B');
    expect(compact(9_225_000_000)).toBe('$92.3M');
  });

  it('uses a minus sign rather than a hyphen', () => {
    expect(compact(-9_225_000_000)).toBe('−$92.3M');
  });
});

describe('delta', () => {
  it('always shows direction', () => {
    expect(delta(9_225_000_000)).toBe('+$92.3M');
    expect(delta(-7_288_820_200)).toBe('−$72.9M');
  });

  it('renders no movement without a sign', () => {
    expect(delta(0)).toBe('$0');
  });
});

describe('percent', () => {
  it('signs the value and drops the sign at zero', () => {
    expect(percent(-0.09731835815369168)).toBe('−0.10%');
    expect(percent(1.5)).toBe('+1.50%');
    expect(percent(0)).toBe('0.00%');
  });
});
