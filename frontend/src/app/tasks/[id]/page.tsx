export const dynamicParams = false;
// Suppresses Next.js 16 static export build error when generateStaticParams returns [].
// Has no runtime effect under output: 'export' — all data is fetched client-side.
export const revalidate = 0;

export async function generateStaticParams() {
  return [];
}

import TaskDetailPageClient from './task-detail-page-client';

export default function TaskDetailPage() {
  return <TaskDetailPageClient />;
}
