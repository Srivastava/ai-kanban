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
        <Suspense fallback={
          <div role="status" aria-live="polite" className="flex-1 p-6 space-y-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {[1,2,3,4].map(i => <div key={i} className="h-32 rounded-xl bg-muted/40 animate-shimmer" />)}
            </div>
            <div className="h-48 rounded-xl bg-muted/40 animate-shimmer" />
            <div className="h-48 rounded-xl bg-muted/40 animate-shimmer" />
          </div>
        }>
          <AnalyticsPageInner />
        </Suspense>
      </div>
    </div>
  );
}
