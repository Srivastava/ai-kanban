'use client';

import { Skeleton } from '@/components/ui/skeleton';
import { Card, CardContent } from '@/components/ui/card';

interface KanbanCardSkeletonProps {
  /** Stagger index — each card gets a slightly later entrance */
  index?: number;
}

export function KanbanCardSkeleton({ index = 0 }: KanbanCardSkeletonProps) {
  return (
    <Card
      className="cursor-grab active:cursor-grabbing animate-fade-in-up"
      style={{ animationDelay: `${index * 80}ms` }}
    >
      <CardContent className="p-3 space-y-2.5">
        {/* Title line — varies width per index for a more natural look */}
        <Skeleton className={`h-3.5 ${index % 2 === 0 ? 'w-4/5' : 'w-3/5'}`} />
        {/* Second title line (some cards have 2-line titles) */}
        {index % 3 === 0 && <Skeleton className="h-3.5 w-2/5" />}
        <div className="flex items-center justify-between pt-0.5">
          {/* Stage chip */}
          <Skeleton className="h-5 w-16 rounded-full" />
          <div className="flex items-center gap-1.5">
            {/* Cost or priority badge */}
            <Skeleton className="h-3.5 w-10 rounded" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
