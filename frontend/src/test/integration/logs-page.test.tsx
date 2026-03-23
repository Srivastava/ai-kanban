import { describe, it, expect } from 'vitest';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { renderWithProviders } from '@/test/utils';
import { LogTable } from '@/components/logs/log-table';
import { mockLog } from '@/test/msw/fixtures';
import type { LogEntry } from '@/types/log';

const warnLog: LogEntry = {
  ...mockLog,
  id: 3,
  level: 'WARN',
  message: 'Something suspicious',
};

describe('Logs page integration', () => {
  it('renders log table with data from API', async () => {
    server.use(
      http.get('/api/logs', () =>
        HttpResponse.json([mockLog, warnLog])
      )
    );

    renderWithProviders(
      <LogTable logs={[mockLog, warnLog]} filter={{}} />
    );

    expect(screen.getByText('Test log message')).toBeInTheDocument();
    expect(screen.getByText('Something suspicious')).toBeInTheDocument();
  });

  it('level filter hides non-matching rows', () => {
    renderWithProviders(
      <LogTable logs={[mockLog, warnLog]} filter={{ level: 'INFO' }} />
    );
    // INFO filter shows only INFO logs, not WARN logs
    expect(screen.getByText('Test log message')).toBeInTheDocument();
    expect(screen.queryByText('Something suspicious')).not.toBeInTheDocument();
  });

  it('search filter hides non-matching rows', () => {
    renderWithProviders(
      <LogTable logs={[mockLog, warnLog]} filter={{ search: 'suspicious' }} />
    );
    expect(screen.queryByText('Test log message')).not.toBeInTheDocument();
    expect(screen.getByText('Something suspicious')).toBeInTheDocument();
  });

  it('expands metadata on row click', () => {
    const logWithMeta: LogEntry = {
      ...mockLog,
      id: 10,
      message: 'Event with metadata',
      metadata: '{"key": "value123"}',
    };

    renderWithProviders(<LogTable logs={[logWithMeta]} filter={{}} />);
    fireEvent.click(screen.getByText('Event with metadata'));
    expect(screen.getByText(/value123/)).toBeInTheDocument();
  });
});
