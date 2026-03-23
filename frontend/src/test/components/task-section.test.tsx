import { describe, it, expect } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/utils';
import { TaskSection } from '@/components/tasks/task-section';

describe('TaskSection', () => {
  it('renders title', () => {
    renderWithProviders(
      <TaskSection title="My Section"><p>child</p></TaskSection>
    );
    expect(screen.getByText('My Section')).toBeInTheDocument();
  });

  it('shows children when open by default', () => {
    renderWithProviders(
      <TaskSection title="Section"><p>visible content</p></TaskSection>
    );
    expect(screen.getByText('visible content')).toBeInTheDocument();
  });

  it('hides children when defaultOpen is false', () => {
    renderWithProviders(
      <TaskSection title="Section" defaultOpen={false}><p>hidden content</p></TaskSection>
    );
    expect(screen.queryByText('hidden content')).not.toBeInTheDocument();
  });

  it('toggles open on button click', async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <TaskSection title="Toggle"><p>toggle content</p></TaskSection>
    );
    // Initially open
    expect(screen.getByText('toggle content')).toBeInTheDocument();
    // Click to close
    await user.click(screen.getByRole('button', { name: /toggle/i }));
    expect(screen.queryByText('toggle content')).not.toBeInTheDocument();
    // Click to re-open
    await user.click(screen.getByRole('button', { name: /toggle/i }));
    expect(screen.getByText('toggle content')).toBeInTheDocument();
  });

  it('renders a button for the title', () => {
    renderWithProviders(
      <TaskSection title="Clickable"><p>x</p></TaskSection>
    );
    expect(screen.getByRole('button')).toBeInTheDocument();
  });
});
