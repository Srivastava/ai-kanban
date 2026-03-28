// dynamicParams=false + a single placeholder param forces Next.js to statically pre-render
// /tasks/__placeholder__/index.html. Axum's static handler serves this file for any
// /tasks/[real-uuid]/ URL it can't resolve, so the browser hydrates as the task detail
// shell (not the root task list). The real task ID is read from window.location at runtime.
export const dynamicParams = false;

export async function generateStaticParams() {
  return [{ id: '__placeholder__' }];
}

import TaskDetailPageClient from './task-detail-page-client';

export default function TaskDetailPage() {
  return <TaskDetailPageClient />;
}
