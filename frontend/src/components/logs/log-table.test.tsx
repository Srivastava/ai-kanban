import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LogTable } from './log-table';
import { mockLog } from '@/test/msw/fixtures';
import type { LogEntry } from '@/types/log';

const errorLog: LogEntry = {
  ...mockLog,
  id: 2,
  level: 'ERROR',
  message: 'Something broke',
  metadata: '{"endpoint": "/api/tasks"}',
};

describe('LogTable', () => {
  it('renders log entries', () => {
    render(<LogTable logs={[mockLog]} filter={{}} />);
    expect(screen.getByText('Test log message')).toBeInTheDocument();
  });

  it('shows empty state when no logs match filter', () => {
    render(<LogTable logs={[mockLog]} filter={{ search: 'xyz-no-match' }} />);
    expect(screen.getByText(/No logs match/i)).toBeInTheDocument();
  });

  it('filters by search term client-side', () => {
    render(<LogTable logs={[mockLog, errorLog]} filter={{ search: 'broke' }} />);
    expect(screen.queryByText('Test log message')).not.toBeInTheDocument();
    expect(screen.getByText('Something broke')).toBeInTheDocument();
  });

  it('expands row detail on click', () => {
    render(<LogTable logs={[errorLog]} filter={{}} />);

    // Before click: metadata not visible
    expect(screen.queryByText('/api/tasks')).not.toBeInTheDocument();

    // Click row
    fireEvent.click(screen.getByText('Something broke'));

    // After click: metadata is shown in expanded detail
    expect(screen.getByText(/api\/tasks/)).toBeInTheDocument();
  });

  it('collapses row on second click', () => {
    render(<LogTable logs={[errorLog]} filter={{}} />);
    const row = screen.getByText('Something broke');

    fireEvent.click(row);
    expect(screen.getByText(/api\/tasks/)).toBeInTheDocument();

    fireEvent.click(row);
    expect(screen.queryByText(/api\/tasks/)).not.toBeInTheDocument();
  });

  it('displays correct source color for frontend logs', () => {
    render(<LogTable logs={[mockLog]} filter={{}} />);
    const sourceEl = screen.getByText('frontend');
    expect(sourceEl.className).toContain('blue');
  });
});
