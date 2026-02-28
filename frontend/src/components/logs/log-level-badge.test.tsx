import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LogLevelBadge } from './log-level-badge';

describe('LogLevelBadge', () => {
  it('renders DEBUG badge', () => {
    render(<LogLevelBadge level="DEBUG" />);
    expect(screen.getByText('DEBUG')).toBeInTheDocument();
  });

  it('renders INFO badge', () => {
    render(<LogLevelBadge level="INFO" />);
    expect(screen.getByText('INFO')).toBeInTheDocument();
  });

  it('renders WARN badge with amber color class', () => {
    render(<LogLevelBadge level="WARN" />);
    const badge = screen.getByText('WARN');
    expect(badge.className).toContain('amber');
  });

  it('renders ERROR badge with red color class', () => {
    render(<LogLevelBadge level="ERROR" />);
    const badge = screen.getByText('ERROR');
    expect(badge.className).toContain('red');
  });
});
