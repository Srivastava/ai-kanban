import { describe, it, expect } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithProviders } from '@/test/utils';
import { SessionIntelligenceCard } from './session-intelligence-card';
import { mockSessionSummary } from '@/test/msw/fixtures';

describe('SessionIntelligenceCard', () => {
  it('renders Total Sessions label and value', async () => {
    renderWithProviders(<SessionIntelligenceCard />);
    await waitFor(() => {
      expect(screen.getByText(/Total Sessions/i)).toBeInTheDocument();
      expect(screen.getByText(mockSessionSummary.total_sessions.toString())).toBeInTheDocument();
    });
  });

  it('renders burn rate tokens per minute label', async () => {
    renderWithProviders(<SessionIntelligenceCard />);
    await waitFor(() => {
      expect(screen.getByText(/tokens\/min/i)).toBeInTheDocument();
    });
  });

  it('renders total cost with dollar sign', async () => {
    renderWithProviders(<SessionIntelligenceCard />);
    await waitFor(() => {
      expect(screen.getByText(/\$1\.1250/)).toBeInTheDocument();
    });
  });

  it('shows loading skeleton while fetching', () => {
    renderWithProviders(<SessionIntelligenceCard />);
    expect(document.querySelector('.animate-pulse')).toBeTruthy();
  });
});
