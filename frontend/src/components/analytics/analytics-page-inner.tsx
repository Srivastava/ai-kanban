'use client';

import { useState, useEffect } from 'react';
import dynamic from 'next/dynamic';
import { useSearchParams, useRouter } from 'next/navigation';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/contexts/websocket-context';

// All chart/heavy components are dynamically imported (ssr: false) so Recharts
// and other large deps are excluded from the server bundle and only loaded when
// the analytics page is actually visited.
const CommandCenter     = dynamic(() => import('@/components/analytics/command-center').then(m => m.CommandCenter), { ssr: false });
const RoiCards          = dynamic(() => import('@/components/analytics/roi-cards').then(m => m.RoiCards), { ssr: false });
const CostBreakdownTable= dynamic(() => import('@/components/analytics/cost-breakdown-table').then(m => m.CostBreakdownTable), { ssr: false });
const TokensByTaskChart = dynamic(() => import('@/components/analytics/tokens-by-task-chart').then(m => m.TokensByTaskChart), { ssr: false });
const ProjectBubbleChart= dynamic(() => import('@/components/analytics/project-bubble-chart').then(m => m.ProjectBubbleChart), { ssr: false });
const CumulativeCostChart=dynamic(() => import('@/components/analytics/cumulative-cost-chart').then(m => m.CumulativeCostChart), { ssr: false });
const ActivityHeatmap   = dynamic(() => import('@/components/analytics/activity-heatmap').then(m => m.ActivityHeatmap), { ssr: false });
const HourlyBreakdown   = dynamic(() => import('@/components/analytics/hourly-breakdown').then(m => m.HourlyBreakdown), { ssr: false });
const TokenTimeChart    = dynamic(() => import('@/components/analytics/token-time-chart').then(m => m.TokenTimeChart), { ssr: false });
const ToolBreakdownChart= dynamic(() => import('@/components/analytics/tool-breakdown-chart').then(m => m.ToolBreakdownChart), { ssr: false });
const LanguageChart     = dynamic(() => import('@/components/analytics/language-chart').then(m => m.LanguageChart), { ssr: false });
const StageBreakdownChart=dynamic(() => import('@/components/analytics/stage-breakdown-chart').then(m => m.StageBreakdownChart), { ssr: false });
const ProductivitySection=dynamic(() => import('@/components/analytics/productivity-section').then(m => m.ProductivitySection), { ssr: false });
const SessionTimelineChart=dynamic(() => import('@/components/analytics/session-timeline-chart').then(m => m.SessionTimelineChart), { ssr: false });
const DevActivityCharts = dynamic(() => import('@/components/analytics/dev-activity-charts').then(m => m.DevActivityCharts), { ssr: false });
// TaskFilterBar is above the fold / interactive — keep it static
import { TaskFilterBar } from '@/components/analytics/task-filter-bar';

// ── Section header — numbered chapter style ───────────────────────────────
interface SectionHeaderProps {
  num: string;
  title: string;
  sub?: string;
}

function SectionHeader({ num, title, sub }: SectionHeaderProps) {
  return (
    <div className="flex items-end gap-4 border-b-2 border-border pb-4 overflow-hidden">
      <span
        aria-hidden
        className="text-[52px] sm:text-[80px] font-black leading-none tracking-tighter text-primary/[0.10] select-none -mb-2 tabular-nums shrink-0 motion-safe:animate-fade-in-up"
        style={{ animationDuration: '0.5s' }}
      >
        {num}
      </span>
      <div className="pb-1 min-w-0 motion-safe:animate-fade-in-up" style={{ animationDelay: '60ms' }}>
        <h2 className="text-2xl sm:text-3xl font-black tracking-tighter leading-none">{title}</h2>
        {sub && (
          <p className="text-[11px] text-muted-foreground mt-1.5 uppercase tracking-widest font-medium">
            {sub}
          </p>
        )}
      </div>
    </div>
  );
}

export function AnalyticsPageInner() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(
    searchParams.get('task')
  );
  const queryClient = useQueryClient();
  const { subscribe } = useWebSocket();

  // Sync task selection to URL
  useEffect(() => {
    const current = searchParams.get('task') ?? null;
    if (current === selectedTaskId) return;
    if (selectedTaskId) {
      router.replace(`?task=${selectedTaskId}`, { scroll: false });
    } else {
      router.replace('?', { scroll: false });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedTaskId]);

  // Auto-refresh analytics queries when session status changes (complete/stop/fail)
  useEffect(() => {
    const invalidateAll = () => {
      queryClient.invalidateQueries({ queryKey: ['analytics'] });
    };
    const unsub = subscribe('session_status', invalidateAll);
    return unsub;
  }, [subscribe, queryClient]);

  return (
    <main className="flex-1 pb-20 md:pb-6">
      {/* Command Center — always global */}
      <section className="p-4 sm:p-6">
        <CommandCenter />
      </section>

      {/* Sticky task filter — sole filter for entire page */}
      <TaskFilterBar selectedTaskId={selectedTaskId} onSelect={setSelectedTaskId} />

      <div className="p-4 sm:p-6 space-y-16">
        {/* ROI & Cost */}
        <section className="space-y-6 motion-safe:animate-fade-in-up" style={{ animationDelay: '80ms' }}>
          <SectionHeader
            num="01"
            title="ROI & Cost"
            sub={selectedTaskId ? 'filtered to selected task' : 'all tasks combined'}
          />
          <RoiCards taskId={selectedTaskId} />
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <CostBreakdownTable taskId={selectedTaskId} />
            <TokensByTaskChart taskId={selectedTaskId} />
          </div>
          <ProjectBubbleChart
            selectedTaskId={selectedTaskId}
            onTaskSelect={setSelectedTaskId}
          />
          <CumulativeCostChart taskId={selectedTaskId} />
        </section>

        {/* Usage Trends */}
        <section className="space-y-6 motion-safe:animate-fade-in-up" style={{ animationDelay: '160ms' }}>
          <SectionHeader num="02" title="Usage Trends" />
          {/* Heatmap + Hourly */}
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
            <div className="lg:col-span-2">
              <ActivityHeatmap taskId={selectedTaskId} />
            </div>
            <HourlyBreakdown taskId={selectedTaskId} />
          </div>
          <TokenTimeChart taskId={selectedTaskId} />
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <ToolBreakdownChart taskId={selectedTaskId} />
            <LanguageChart taskId={selectedTaskId} />
          </div>
          <StageBreakdownChart taskId={selectedTaskId} />
        </section>

        {/* Productivity */}
        <section className="space-y-6 motion-safe:animate-fade-in-up" style={{ animationDelay: '240ms' }}>
          <SectionHeader
            num="03"
            title="Productivity"
            sub="commits, PRs, and lines written — requires OTel"
          />
          <ProductivitySection taskId={selectedTaskId} />
        </section>

        {/* Session Deep Dive */}
        <section className="space-y-6 motion-safe:animate-fade-in-up" style={{ animationDelay: '320ms' }}>
          <SectionHeader
            num="04"
            title="Session Deep Dive"
            sub={selectedTaskId ? 'sessions for selected task' : 'select a task above to filter'}
          />
          <SessionTimelineChart taskId={selectedTaskId} />
          <DevActivityCharts taskId={selectedTaskId} />
        </section>
      </div>
    </main>
  );
}
