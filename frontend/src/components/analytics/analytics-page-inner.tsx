'use client';

import { useState, useEffect } from 'react';
import { useSearchParams, useRouter } from 'next/navigation';
import { CommandCenter } from '@/components/analytics/command-center';
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

export function AnalyticsPageInner() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(
    searchParams.get('task')
  );

  // Sync task selection to URL
  useEffect(() => {
    const params = new URLSearchParams(searchParams.toString());
    if (selectedTaskId) {
      params.set('task', selectedTaskId);
    } else {
      params.delete('task');
    }
    router.replace(`?${params.toString()}`, { scroll: false });
  }, [selectedTaskId, searchParams, router]);

  return (
    <main className="flex-1 pb-20 md:pb-6">
      {/* Command Center — always global */}
      <section className="p-4 sm:p-6">
        <CommandCenter />
      </section>

      {/* Sticky task filter */}
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
            <CostBreakdownTable />
            <TokensByTaskChart />
          </div>
        </section>

        {/* Usage Trends */}
        <section className="space-y-4">
          <h2 className="text-base font-semibold">Usage Trends</h2>
          <TokenTimeChart />
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <ToolBreakdownChart />
            <LanguageChart />
          </div>
          <StageBreakdownChart />
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
          <SessionTimelineChart />
          <DevActivityCharts />
        </section>
      </div>
    </main>
  );
}
