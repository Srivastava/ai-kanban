'use client';

import { Skeleton } from '@/components/ui/skeleton';
import { Card, CardContent } from '@/components/ui/card';

export function KanbanCardSkeleton() {
  return (
    <Card className="cursor-grab active:cursor-grabbing">
      <CardContent className="p-3 space-y-2">
        <Skeleton className="h-4 w-3/4" />
        <div className="flex items-center justify-between">
          <Skeleton className="h-5 w-16 rounded-full" />
          <Skeleton className="h-3 w-8" />
        </div>
      </CardContent>
    </Card>
  );
}
