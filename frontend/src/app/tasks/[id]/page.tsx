export const dynamicParams = false;
export const revalidate = 0;

export async function generateStaticParams() {
  return [];
}

import TaskDetailPageClient from './task-detail-page-client';

export default function TaskDetailPage() {
  return <TaskDetailPageClient />;
}
