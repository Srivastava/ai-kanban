'use client';

import { useState, useEffect } from 'react';
import dynamic from 'next/dynamic';
import { useSearchParams, useRouter } from 'next/navigation';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from '@/contexts/websocket-context';

const CommandCenter = dynamic(
  () => import('@/components/analytics/command-center').then(m => m.CommandCenter),
  { ssr: false }
);
import { TaskFilterBar } from '@/components/analytics/task-filter-bar';
import { RoiCards } from '@/components/analytics/roi-cards';
import { ProductivitySection } from '@/components/analytics/productivity-section';
import { TokenTimeChart } from '@/components/analytics/token-time-chart';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';
import { StageBreakdownChart } from '@/components/analytics/stage-breakdown-chart';
import { CostBreakdownTable } from '@/components/analytics/cost-breakdown-table';
import { SessionTimelineChart } from '@/components/analytics/session-timeline-chart';
import { DevActivityCharts } from '@/components/analytics/dev-activity-charts';
import { TokensByTaskChart } from '@/components/analytics/tokens-by-task-chart';
import { CumulativeCostChart } from '@/components/analytics/cumulative-cost-chart';
import { ActivityHeatmap } from '@/components/analytics/activity-heatmap';
import { HourlyBreakdown } from '@/components/analytics/hourly-breakdown';
import { ProjectBubbleChart } from '@/components/analytics/project-bubble-chart';

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

  // Auto-refresh analytics queries when sessions complete
  useEffect(() => {
    const invalidateAll = () => {
      queryClient.invalidateQueries({ queryKey: ['analytics'] });
    };

    const unsubCompleted = subscribe('session_completed', invalidateAll);
    const unsubFailed = subscribe('session_failed', invalidateAll);
    const unsubStopped = subscribe('session_stopped', invalidateAll);
    return () => {
      unsubCompleted();
      unsubFailed();
      unsubStopped();
    };
  }, [subscribe, queryClient]);

  return (
    <main className="flex-1 pb-20 md:pb-6">
      {/* Command Center — always global */}
      <section className="p-4 sm:p-6">
        <CommandCenter />
      </section>

      {/* Sticky task filter — sole filter for entire page */}
      <TaskFilterBar selectedTaskId={selectedTaskId} onSelect={setSelectedTaskId} />

      <div className="p-4 sm:p-6 space-y-10">
        {/* ROI & Cost */}
        <section className="space-y-4">
          <div>
            <h2 className="text-base font-semibold">ROI & Cost</h2>
            <p className="text-sm text-muted-foreground">
              {selectedTaskId ? 'Filtered to selected task' : 'All tasks combined'}
            </p>
          </div>
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
        <section className="space-y-4">
          <h2 className="text-base font-semibold">Usage Trends</h2>
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
        <section className="space-y-4">
          <div>
            <h2 className="text-base font-semibold">Productivity</h2>
            <p className="text-sm text-muted-foreground">
              Commits, PRs, and lines written (requires OTel)
            </p>
          </div>
          <ProductivitySection taskId={selectedTaskId} />
        </section>

        {/* Session Deep Dive */}
        <section className="space-y-4">
          <div>
            <h2 className="text-base font-semibold">Session Deep Dive</h2>
            <p className="text-sm text-muted-foreground">
              {selectedTaskId ? 'Sessions for selected task' : 'Select a task above to filter'}
            </p>
          </div>
          <SessionTimelineChart taskId={selectedTaskId} />
          <DevActivityCharts taskId={selectedTaskId} />
        </section>
      </div>
    </main>
  );
}
