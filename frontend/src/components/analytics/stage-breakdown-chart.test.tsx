import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { StageBreakdownChart } from './stage-breakdown-chart';
import { mockTokensByStage } from '@/test/msw/fixtures';

describe('StageBreakdownChart', () => {
  it('renders section heading', async () => {
    renderWithProviders(<StageBreakdownChart />);
    await waitFor(() => {
      expect(screen.getByText(/Tokens by Stage/i)).toBeInTheDocument();
    });
  });

  it('renders stage labels from data', async () => {
    renderWithProviders(<StageBreakdownChart />);
    await waitFor(() => {
      mockTokensByStage.forEach(({ stage }) => {
        expect(screen.getByText(stage)).toBeInTheDocument();
      });
    });
  });

  it('shows loading skeleton while fetching', () => {
    renderWithProviders(<StageBreakdownChart />);
    expect(document.querySelector('.animate-pulse')).toBeTruthy();
  });
});
