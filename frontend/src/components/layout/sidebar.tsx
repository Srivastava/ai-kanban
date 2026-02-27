'use client';

import { Suspense } from 'react';
import Link from 'next/link';
import { useSearchParams } from 'next/navigation';
import { cn } from '@/lib/utils';
import type { Stage } from '@/types/task';

const stages: { value: Stage | 'all'; label: string }[] = [
  { value: 'all', label: 'All Tasks' },
  { value: 'backlog', label: 'Backlog' },
  { value: 'planning', label: 'Planning' },
  { value: 'ready', label: 'Ready' },
  { value: 'in_progress', label: 'In Progress' },
  { value: 'review', label: 'Review' },
  { value: 'done', label: 'Done' },
];

function SidebarContent() {
  const searchParams = useSearchParams();
  const currentStage = searchParams.get('stage') || 'all';

  return (
    <aside className="w-64 border-r border-border bg-sidebar min-h-screen p-4">
      <nav className="space-y-1">
        {stages.map((stage) => (
          <Link
            key={stage.value}
            href={stage.value === 'all' ? '/' : `/?stage=${stage.value}`}
            className={cn(
              'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
              currentStage === stage.value
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            )}
          >
            {stage.label}
          </Link>
        ))}
      </nav>
    </aside>
  );
}

export function Sidebar() {
  return (
    <Suspense
      fallback={
        <aside className="w-64 border-r border-border bg-sidebar min-h-screen p-4">
          <nav className="space-y-1">
            {stages.map((stage) => (
              <div
                key={stage.value}
                className="flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground"
              >
                {stage.label}
              </div>
            ))}
          </nav>
        </aside>
      }
    >
      <SidebarContent />
    </Suspense>
  );
}
