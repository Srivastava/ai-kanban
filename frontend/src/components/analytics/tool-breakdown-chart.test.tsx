import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { ToolBreakdownChart } from './tool-breakdown-chart';

describe('ToolBreakdownChart', () => {
  it('shows a legend entry for every tool name', async () => {
    renderWithProviders(<ToolBreakdownChart />);
    await waitFor(() => {
      expect(screen.getByText('Read')).toBeInTheDocument();
      expect(screen.getByText('Write')).toBeInTheDocument();
      expect(screen.getByText('Bash')).toBeInTheDocument();
    });
  });

  it('shows loading skeleton before data arrives', () => {
    renderWithProviders(<ToolBreakdownChart />);
    expect(document.querySelector('.animate-pulse')).toBeTruthy();
  });

  it('does not show empty state when tool data exists', async () => {
    renderWithProviders(<ToolBreakdownChart />);
    await waitFor(() => {
      expect(screen.queryByText('No tool data yet')).not.toBeInTheDocument();
    });
  });
});
