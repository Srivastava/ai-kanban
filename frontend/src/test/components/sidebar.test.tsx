import { describe, it, expect, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';

// Mock next-themes — not available in jsdom
vi.mock('next-themes', () => ({
  useTheme: () => ({ resolvedTheme: 'light', setTheme: vi.fn() }),
}));

// Mock hooks that hit the API
vi.mock('@/hooks/use-sidebar-metrics', () => ({
  useSidebarMetrics: () => null,
}));

// Import after mocks
import { Sidebar } from '@/components/layout/sidebar';

// usePathname is mocked in setup.ts to return '/'
// useSearchParams is mocked to return empty URLSearchParams

describe('Sidebar', () => {
  it('renders the Kanban Board navigation link', () => {
    renderWithProviders(<Sidebar />);
    expect(screen.getByRole('link', { name: /kanban board/i })).toBeInTheDocument();
  });

  it('renders the Analytics navigation link', () => {
    renderWithProviders(<Sidebar />);
    expect(screen.getAllByRole('link', { name: /analytics/i }).length).toBeGreaterThan(0);
  });

  it('renders the Logs navigation link', () => {
    renderWithProviders(<Sidebar />);
    expect(screen.getAllByRole('link', { name: /logs/i }).length).toBeGreaterThan(0);
  });

  it('renders the Settings navigation link', () => {
    renderWithProviders(<Sidebar />);
    expect(screen.getAllByRole('link', { name: /settings/i }).length).toBeGreaterThan(0);
  });

  it('renders the Tasks collapsible button', () => {
    renderWithProviders(<Sidebar />);
    expect(screen.getByRole('button', { name: /tasks/i })).toBeInTheDocument();
  });

  it('renders task stage links by default (tasks section open)', () => {
    renderWithProviders(<Sidebar />);
    // The tasks section starts open — should show stage filter links
    expect(screen.getByRole('link', { name: /all tasks/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /backlog/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /planning/i })).toBeInTheDocument();
  });

  it('collapses task stage list when Tasks button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Sidebar />);

    // Initially open
    expect(screen.getByRole('link', { name: /all tasks/i })).toBeInTheDocument();

    // Click to collapse
    await user.click(screen.getByRole('button', { name: /tasks/i }));
    expect(screen.queryByRole('link', { name: /all tasks/i })).not.toBeInTheDocument();
  });

  it('re-expands task stage list when Tasks button is clicked again', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Sidebar />);

    await user.click(screen.getByRole('button', { name: /tasks/i }));
    await user.click(screen.getByRole('button', { name: /tasks/i }));

    expect(screen.getByRole('link', { name: /all tasks/i })).toBeInTheDocument();
  });

  it('renders the theme toggle button', () => {
    renderWithProviders(<Sidebar />);
    expect(screen.getByRole('button', { name: /toggle dark mode/i })).toBeInTheDocument();
  });

  it('renders mobile bottom nav with Tasks link', () => {
    renderWithProviders(<Sidebar />);
    // Mobile nav has a "Tasks" link
    const taskLinks = screen.getAllByRole('link', { name: /^tasks$/i });
    expect(taskLinks.length).toBeGreaterThanOrEqual(1);
  });

  it('highlights the active link for the current path', () => {
    // setup.ts mocks usePathname to return '/'
    renderWithProviders(<Sidebar />);
    // "All Tasks" link is at href="/" so it should be active when pathname is "/"
    const allTasksLink = screen.getByRole('link', { name: /all tasks/i });
    expect(allTasksLink).toHaveClass('bg-sidebar-accent');
  });
});
