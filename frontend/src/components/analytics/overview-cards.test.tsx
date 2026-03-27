import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { OverviewCards } from './overview-cards';
import { mockOverview } from '@/test/msw/fixtures';

describe('OverviewCards', () => {
  it('renders all 4 card labels', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      expect(screen.getByText(/Token Usage/i)).toBeInTheDocument();
      expect(screen.getByText(/Estimated Cost/i)).toBeInTheDocument();
      expect(screen.getByText(/Total Sessions/i)).toBeInTheDocument();
      expect(screen.getByText(/Tasks with AI/i)).toBeInTheDocument();
    });
  });

  it('shows formatted total sessions', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      expect(screen.getByText(mockOverview.total_sessions.toString())).toBeInTheDocument();
    });
  });

  it('shows loading skeleton while fetching', () => {
    renderWithProviders(<OverviewCards />);
    // Before data loads, skeletons appear (animate-shimmer elements)
    const skeletons = document.querySelectorAll('.animate-shimmer');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('shows estimated cost with dollar sign', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      expect(screen.getByText(/\$1\.13/)).toBeInTheDocument();
    });
  });
});
