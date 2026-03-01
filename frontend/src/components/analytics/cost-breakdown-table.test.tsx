import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { CostBreakdownTable } from './cost-breakdown-table';
import { mockCostByTask } from '@/test/msw/fixtures';

describe('CostBreakdownTable', () => {
  it('renders table headers', async () => {
    renderWithProviders(<CostBreakdownTable />);
    await waitFor(() => {
      expect(screen.getByText(/Task/i)).toBeInTheDocument();
      expect(screen.getByText(/Cost/i)).toBeInTheDocument();
    });
  });

  it('renders a row for each task', async () => {
    renderWithProviders(<CostBreakdownTable />);
    await waitFor(() => {
      expect(screen.getByText(mockCostByTask[0].task_title)).toBeInTheDocument();
      expect(screen.getByText(mockCostByTask[1].task_title)).toBeInTheDocument();
    });
  });

  it('shows cost with dollar sign', async () => {
    renderWithProviders(<CostBreakdownTable />);
    await waitFor(() => {
      expect(screen.getByText(/\$0\.60/)).toBeInTheDocument();
    });
  });

  it('shows loading skeleton while fetching', () => {
    renderWithProviders(<CostBreakdownTable />);
    expect(document.querySelector('.animate-pulse')).toBeTruthy();
  });
});
