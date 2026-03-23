import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { renderWithProviders } from '@/test/utils';
import { OverviewCards } from '@/components/analytics/overview-cards';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';

describe('Analytics page components integration', () => {
  it('OverviewCards loads and displays token count', async () => {
    renderWithProviders(<OverviewCards />);
    await waitFor(() => {
      // 150000 + 45000 = 195000 → "195.0K"
      expect(screen.getByText(/195\.0K/)).toBeInTheDocument();
    });
  });

  it('ToolBreakdownChart renders without crashing', async () => {
    renderWithProviders(<ToolBreakdownChart />);
    await waitFor(() => {
      expect(screen.getByText(/Tool Usage/i)).toBeInTheDocument();
    });
  });

  it('LanguageChart renders without crashing', async () => {
    renderWithProviders(<LanguageChart />);
    await waitFor(() => {
      expect(screen.getByText(/Tokens by Language/i)).toBeInTheDocument();
    });
  });

  it('ToolBreakdownChart shows empty state when no data', async () => {
    server.use(
      http.get('/api/analytics/tokens/by-tool', () =>
        HttpResponse.json([])
      )
    );

    renderWithProviders(<ToolBreakdownChart />);
    await waitFor(() => {
      expect(screen.getByText(/No tool data yet/i)).toBeInTheDocument();
    });
  });
});
