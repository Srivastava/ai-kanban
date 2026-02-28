import { Sidebar } from '@/components/layout/sidebar';
import { OverviewCards } from '@/components/analytics/overview-cards';
import { UsageWindowsCard } from '@/components/analytics/usage-windows-card';
import { TokenTimeChart } from '@/components/analytics/token-time-chart';
import { ToolBreakdownChart } from '@/components/analytics/tool-breakdown-chart';
import { LanguageChart } from '@/components/analytics/language-chart';
import { SessionTimelineChart } from '@/components/analytics/session-timeline-chart';

export default function AnalyticsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <div className="border-b border-border px-6 py-4">
          <h1 className="text-xl font-semibold">Analytics</h1>
          <p className="text-sm text-muted-foreground">Claude token usage and efficiency</p>
        </div>
        <main className="flex-1 p-6 space-y-6">
          <OverviewCards />
          <UsageWindowsCard />
          <TokenTimeChart />
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <ToolBreakdownChart />
            <LanguageChart />
          </div>
          <SessionTimelineChart />
        </main>
      </div>
    </div>
  );
}
