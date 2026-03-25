import { Suspense } from 'react';
import { Sidebar } from '@/components/layout/sidebar';
import { AnalyticsPageInner } from '@/components/analytics/analytics-page-inner';

export default function AnalyticsPage() {
  return (
    <div className="flex min-h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <div className="border-b border-border px-4 sm:px-6 py-4">
          <h1 className="text-4xl sm:text-5xl font-black tracking-tighter leading-none">Analytics</h1>
          <p className="text-xs text-primary/70 mt-1 uppercase tracking-widest font-medium">Claude usage · cost · productivity</p>
        </div>
        <Suspense fallback={<div className="flex-1 p-6 text-xs text-muted-foreground uppercase tracking-widest font-medium animate-pulse">Loading…</div>}>
          <AnalyticsPageInner />
        </Suspense>
      </div>
    </div>
  );
}
