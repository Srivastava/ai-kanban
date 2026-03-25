'use client';

import { Suspense, useState, useEffect } from 'react';
import Link from 'next/link';
import { usePathname, useSearchParams } from 'next/navigation';
import { useTheme } from 'next-themes';
import {
  LayoutGrid, BarChart2, FileText, Settings, List,
  ChevronDown, Moon, Sun,
} from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { cn } from '@/lib/utils';
import { apiClient } from '@/lib/api-client';
import type { Stage } from '@/types/task';
import { useSidebarMetrics } from '@/hooks/use-sidebar-metrics';
import { stageColors } from '@/lib/task-colors';
import type { AnalyticsOverview } from '@/types/analytics';

// ── constants ─────────────────────────────────────────────────────────────────

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

// ── dark mode toggle ──────────────────────────────────────────────────────────

function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  return (
    <button
      aria-label="Toggle dark mode"
      onClick={() => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark')}
      className="rounded-md p-1.5 text-sidebar-foreground hover:bg-sidebar-accent/50 transition-colors"
    >
      {mounted && resolvedTheme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
    </button>
  );
}

// ── sidebar metrics panel ─────────────────────────────────────────────────────

function SidebarMetrics() {
  const metrics = useSidebarMetrics();
  const { data } = useQuery<AnalyticsOverview>({
    queryKey: ['analytics', 'overview'],
    queryFn: () => apiClient<AnalyticsOverview>('/api/analytics/overview'),
    staleTime: 60_000,
    refetchInterval: 60_000,
  });

  if (!metrics) return null;

  return (
    <div className="pt-3 border-t border-border">
      <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground px-1 mb-2">
        Usage
      </p>
      <div className="grid grid-cols-2 gap-1.5">
        {metrics.map((m) => (
          <div key={m.label} className="rounded-md bg-muted/40 px-2 py-1.5">
            <p className="text-[10px] text-muted-foreground leading-tight">{m.label}</p>
            <p className="text-sm font-semibold text-foreground leading-tight">{m.value}</p>
          </div>
        ))}
      </div>
      {data && data.active_sessions_today > 0 && (
        <div className="mt-1.5 flex items-center gap-1.5 text-[11px] text-green-600 dark:text-green-400 px-1">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-green-500 motion-safe:animate-breathe" />
          {data.active_sessions_today} active today
        </div>
      )}
    </div>
  );
}

// ── sidebar content ───────────────────────────────────────────────────────────

function SidebarContent() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const currentStage = searchParams.get('stage') || 'all';
  const [tasksOpen, setTasksOpen] = useState(true);

  const isActive = (stage: string) => {
    if (stage === 'all') return pathname === '/' && currentStage === 'all';
    return pathname === '/' && currentStage === stage;
  };

  const isNavActive = (href: string) => {
    if (href === '/') return pathname === '/';
    return pathname.startsWith(href);
  };

  const navLink = (href: string, icon: React.ElementType, label: string) => {
    const Icon = icon;
    const active = isNavActive(href);
    return (
      <Link
        href={href}
        aria-current={active ? 'page' : undefined}
        className={cn(
          'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-all duration-150 whitespace-nowrap',
          active
            ? 'bg-primary/10 text-primary font-semibold border-l-2 border-primary'
            : 'text-sidebar-foreground hover:bg-sidebar-accent/50 border-l-2 border-transparent'
        )}
      >
        <Icon className="h-4 w-4 shrink-0" />
        {label}
      </Link>
    );
  };

  return (
    <>
      <aside className="w-0 overflow-hidden md:w-56 shrink-0 md:border-r border-border bg-sidebar md:min-h-screen md:p-4 md:flex md:flex-col">
        {/* Top row: app name + theme toggle */}
        <div className="hidden md:flex items-center justify-between mb-3">
          <span className="text-sm font-bold tracking-tight text-foreground">
            AI Kanban
          </span>
          <ThemeToggle />
        </div>

        <nav className="space-y-1 flex-1 pb-4">
          {navLink('/kanban', LayoutGrid, 'Kanban Board')}
          {navLink('/analytics', BarChart2, 'Analytics')}
          {navLink('/logs', FileText, 'Logs')}
          {navLink('/settings', Settings, 'Settings')}

          <div className="h-px bg-border my-2" />

          {/* Collapsible Tasks section */}
          <button
            onClick={() => setTasksOpen((o) => !o)}
            aria-expanded={tasksOpen}
            aria-controls="sidebar-tasks-list"
            className="w-full flex items-center justify-between rounded-md px-3 py-2 text-sm font-medium text-sidebar-foreground hover:bg-sidebar-accent/50 transition-colors"
          >
            <span className="flex items-center gap-3">
              <List className="h-4 w-4 shrink-0" />
              Tasks
            </span>
            <ChevronDown
              className={`h-3.5 w-3.5 text-muted-foreground transition-transform duration-200 ${tasksOpen ? 'rotate-0' : '-rotate-90'}`}
            />
          </button>

          <div
            className="grid transition-[grid-template-rows] duration-200 ease-out"
            style={{ gridTemplateRows: tasksOpen ? '1fr' : '0fr' }}
          >
            <div
              id="sidebar-tasks-list"
              inert={!tasksOpen}
              className="overflow-hidden ml-4 space-y-0.5 border-l border-border pl-3"
            >
              {stages.map((stage) => (
                <Link
                  key={stage.value}
                  href={stage.value === 'all' ? '/' : `/?stage=${stage.value}`}
                  className={cn(
                    'flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors whitespace-nowrap',
                    isActive(stage.value)
                      ? 'bg-sidebar-accent text-sidebar-accent-foreground font-medium'
                      : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
                  )}
                >
                  {stage.value !== 'all' && (
                    <span className={`h-2 w-2 rounded-full shrink-0 ${stageColors[stage.value as Stage]}`} />
                  )}
                  {stage.label}
                </Link>
              ))}
            </div>
          </div>

          {/* Metrics panel below Tasks section */}
          <SidebarMetrics />
        </nav>
      </aside>

      {/* Mobile bottom nav */}
      <nav className="md:hidden fixed bottom-0 left-0 right-0 z-50 border-t border-border bg-background">
        <div className="flex items-stretch h-14">
          {mobileNavItems.map(({ href, label, icon: Icon }) => (
            <Link
              key={href}
              href={href}
              aria-current={isNavActive(href) ? 'page' : undefined}
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
