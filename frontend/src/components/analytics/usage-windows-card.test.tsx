import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { UsageWindowsCard } from './usage-windows-card';

vi.mock('@/hooks/use-analytics', () => ({
  useUsageWindows: () => ({
    data: {
      tokens_5hr: 0,
      tokens_week: 0,
      limit_5hr: 0,
      limit_week: 0,
      reset_5hr: null,
      reset_week: null,
      no_data: true,
    },
    isLoading: false,
    isFetching: false,
    refetch: vi.fn(),
    dataUpdatedAt: Date.now(),
  }),
}));

describe('UsageWindowsCard', () => {
  it('shows Rate limited when no_data is true', () => {
    render(<UsageWindowsCard />);
    expect(screen.getByText('Rate limited')).toBeTruthy();
  });
});
