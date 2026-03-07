import { Sidebar } from '@/components/layout/sidebar';
import { OverviewCards } from '@/components/analytics/overview-cards';
import { UsageWindowsCard } from '@/components/analytics/usage-windows-card';
import { TokenTimeChart } from '@/components/analytics/token-time-chart';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';
import { StageBreakdownChart } from '@/components/analytics/stage-breakdown-chart';
import { SessionIntelligenceCard } from '@/components/analytics/session-intelligence-card';
import { CostBreakdownTable } from '@/components/analytics/cost-breakdown-table';
import { SessionTimelineChart } from '@/components/analytics/session-timeline-chart';
import { DevActivityCharts } from '@/components/analytics/dev-activity-charts';
import { TokensByTaskChart } from '@/components/analytics/tokens-by-task-chart';
import { TokenEfficiencyChart } from '@/components/analytics/token-efficiency-chart';

export default function AnalyticsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <div className="border-b border-border px-6 py-4">
          <h1 className="text-xl font-semibold">Analytics</h1>
          <p className="text-sm text-muted-foreground">Claude token usage and efficiency</p>
        </div>
        <main className="flex-1 p-6 space-y-8">
          <section className="space-y-4">
            <h2 className="text-base font-semibold text-foreground">Overview</h2>
            <OverviewCards />
          </section>
          <section className="space-y-4">
            <h2 className="text-base font-semibold text-foreground">Rate Limits</h2>
            <UsageWindowsCard />
          </section>
          <section className="space-y-4">
            <h2 className="text-base font-semibold text-foreground">Trends</h2>
            <TokenTimeChart />
            <TokensByTaskChart />
          </section>
          <section className="space-y-4">
            <h2 className="text-base font-semibold text-foreground">Breakdowns</h2>
            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <ToolBreakdownChart />
              <LanguageChart />
            </div>
            <StageBreakdownChart />
            <TokenEfficiencyChart />
          </section>
          <section className="space-y-4">
            <h2 className="text-base font-semibold text-foreground">Agent Intelligence</h2>
            <SessionIntelligenceCard />
            <CostBreakdownTable />
            <SessionTimelineChart />
          </section>
          <section className="space-y-4">
            <div>
              <h2 className="text-base font-semibold text-foreground">Dev Activity</h2>
              <p className="text-sm text-muted-foreground">
                Lines changed and token usage per task — select a task to view details
              </p>
            </div>
            <DevActivityCharts />
          </section>
        </main>
      </div>
    </div>
  );
}
