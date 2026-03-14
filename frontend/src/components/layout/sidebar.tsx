'use client';

import { Suspense } from 'react';
import Link from 'next/link';
import { usePathname, useSearchParams } from 'next/navigation';
import { LayoutGrid, BarChart2, FileText, Settings, List } from 'lucide-react';
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

const mobileNavItems = [
  { href: '/', label: 'Tasks', icon: List },
  { href: '/kanban', label: 'Kanban', icon: LayoutGrid },
  { href: '/analytics', label: 'Analytics', icon: BarChart2 },
  { href: '/logs', label: 'Logs', icon: FileText },
  { href: '/settings', label: 'Settings', icon: Settings },
];

function SidebarContent() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const currentStage = searchParams.get('stage') || 'all';

  const isActive = (stage: string) => {
    if (stage === 'all') return pathname === '/' && currentStage === 'all';
    return pathname === '/' && currentStage === stage;
  };

  const isNavActive = (href: string) => {
    if (href === '/') return pathname === '/';
    return pathname.startsWith(href);
  };

  return (
    <>
      {/*
        Desktop sidebar: w-56, visible.
        Mobile: w-0 + overflow-hidden collapses it to zero width without
        removing it from the flex flow — avoids SSR/hydration mismatches
        that happen when toggling display:none vs display:block.
      */}
      <aside className="w-0 overflow-hidden md:w-56 shrink-0 md:border-r border-border bg-sidebar md:min-h-screen md:p-4">
        <nav className="space-y-1">
          <Link
            href="/kanban"
            className={cn(
              'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors whitespace-nowrap',
              pathname === '/kanban'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            )}
          >
            <LayoutGrid className="h-4 w-4 shrink-0" />
            Kanban Board
          </Link>
          <Link
            href="/analytics"
            className={cn(
              'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors whitespace-nowrap',
              pathname === '/analytics'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            )}
          >
            <BarChart2 className="h-4 w-4 shrink-0" />
            Analytics
          </Link>
          <Link
            href="/logs"
            className={cn(
              'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors whitespace-nowrap',
              pathname === '/logs'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            )}
          >
            <FileText className="h-4 w-4 shrink-0" />
            Logs
          </Link>
          <Link
            href="/settings"
            className={cn(
              'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors whitespace-nowrap',
              pathname === '/settings'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            )}
          >
            <Settings className="h-4 w-4 shrink-0" />
            Settings
          </Link>
          <div className="h-px bg-border my-2" />
          {stages.map((stage) => (
            <Link
              key={stage.value}
              href={stage.value === 'all' ? '/' : `/?stage=${stage.value}`}
              className={cn(
                'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors whitespace-nowrap',
                isActive(stage.value)
                  ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                  : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
              )}
            >
              {stage.label}
            </Link>
          ))}
        </nav>
      </aside>

      {/* Mobile bottom nav — CSS position:fixed, no effect on flex layout */}
      <nav className="md:hidden fixed bottom-0 left-0 right-0 z-50 border-t border-border bg-background/95 backdrop-blur-sm">
        <div className="flex items-stretch h-14">
          {mobileNavItems.map(({ href, label, icon: Icon }) => (
            <Link
              key={href}
              href={href}
              className={cn(
                'flex flex-1 flex-col items-center justify-center gap-0.5 text-[10px] font-medium transition-colors',
                isNavActive(href)
                  ? 'text-primary'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <Icon className="h-5 w-5" />
              {label}
            </Link>
          ))}
        </div>
      </nav>
    </>
  );
}

export function Sidebar() {
  return (
    <Suspense
      fallback={
        <aside className="w-0 overflow-hidden md:w-56 shrink-0 md:border-r border-border bg-sidebar md:min-h-screen md:p-4" />
      }
    >
      <SidebarContent />
    </Suspense>
  );
}
