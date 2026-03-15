import { Suspense } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { AnalyticsPageInner } from '@/components/analytics/analytics-page-inner';

export default function AnalyticsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <div className="border-b border-border px-4 sm:px-6 py-4">
          <h1 className="text-xl font-semibold">Analytics</h1>
          <p className="text-sm text-muted-foreground">
            Claude usage, cost, and productivity intelligence
          </p>
        </div>
        <Suspense fallback={<div className="flex-1 p-6 text-muted-foreground text-sm">Loading…</div>}>
          <AnalyticsPageInner />
        </Suspense>
      </div>
    </div>
  );
}
