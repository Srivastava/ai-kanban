'use client';

import { KanbanCardSkeleton } from './kanban-card-skeleton';
import { Skeleton } from '@/components/ui/skeleton';

const COLUMN_COUNTS = [2, 1, 2, 3, 1, 2];

export function KanbanBoardSkeleton() {
  return (
    <div className="space-y-3">
      {/* Stats bar skeleton */}
      <div className="flex items-center gap-3 pb-1">
        <Skeleton className="h-4 w-16 rounded" />
        <Skeleton className="h-4 w-20 rounded" />
        <Skeleton className="ml-auto h-7 w-24 rounded" />
      </div>

      {/* Columns — desktop */}
      <div className="hidden sm:flex gap-3 overflow-x-auto pb-4">
        {COLUMN_COUNTS.map((count, col) => (
          <div
            key={col}
            className="flex flex-col w-full sm:min-w-[270px] sm:max-w-[310px] rounded-lg bg-muted/10 border-t-4 border-t-border/40 animate-fade-in-up"
            style={{ animationDelay: `${col * 40}ms` }}
          >
            {/* Column header */}
            <div className="flex items-center justify-between px-3 pt-3 pb-2.5 bg-muted/20">
              <Skeleton className="h-3 w-16 rounded" />
              <Skeleton className="h-6 w-6 rounded" />
            </div>
            {/* Cards */}
            <div className="flex-1 rounded-lg p-2 space-y-2 min-h-[200px] bg-muted/30">
              {Array.from({ length: count }).map((_, i) => (
                <KanbanCardSkeleton key={i} index={col * 3 + i} />
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* Mobile skeleton */}
      <div className="sm:hidden space-y-3">
        <div className="flex gap-1 pb-2 border-b border-border">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-9 w-16 rounded-full shrink-0" />
          ))}
        </div>
        <div className="space-y-2">
          {Array.from({ length: 3 }).map((_, i) => (
            <KanbanCardSkeleton key={i} index={i} />
          ))}
        </div>
      </div>
    </div>
  );
}
