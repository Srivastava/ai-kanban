import { http, HttpResponse } from 'msw';
import { mockTask, mockTask2, mockLog, mockOverview, mockCostByTask, mockTokensByStage, mockSessionSummary, mockBurnRate } from './fixtures';

const API_BASE = 'http://localhost:3001';

export const handlers = [
  // Tasks
  http.get(`${API_BASE}/api/tasks`, () => {
    return HttpResponse.json([mockTask, mockTask2]);
  }),
  http.get(`${API_BASE}/api/tasks/:id`, ({ params }) => {
    if (params.id === mockTask.id) return HttpResponse.json(mockTask);
    return HttpResponse.json({ error: 'Not found' }, { status: 404 });
  }),
  http.post(`${API_BASE}/api/tasks`, async ({ request }) => {
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json(
      { ...mockTask, id: 'new-task-id', title: body.title as string },
      { status: 201 }
    );
  }),
  http.patch(`${API_BASE}/api/tasks/:id`, async ({ params, request }) => {
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json({ ...mockTask, id: params.id as string, ...body });
  }),
  http.delete(`${API_BASE}/api/tasks/:id`, () => {
    return new HttpResponse(null, { status: 204 });
  }),
  http.post(`${API_BASE}/api/tasks/:id/move`, async ({ params, request }) => {
    const body = await request.json() as Record<string, unknown>;
    return HttpResponse.json({ ...mockTask, id: params.id as string, stage: body.stage });
  }),

  // Logs
  http.get(`${API_BASE}/api/logs`, () => {
    return HttpResponse.json([mockLog]);
  }),
  http.post(`${API_BASE}/api/logs`, () => {
    return HttpResponse.json({ ...mockLog, id: 2 }, { status: 201 });
  }),

  // Analytics
  http.get(`${API_BASE}/api/analytics/overview`, () => {
    return HttpResponse.json(mockOverview);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/daily`, () => {
    return HttpResponse.json([
      { date: '2026-02-25', input_tokens: 10000, output_tokens: 3000 },
      { date: '2026-02-26', input_tokens: 25000, output_tokens: 7500 },
      { date: '2026-02-27', input_tokens: 15000, output_tokens: 4500 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/weekly`, () => {
    return HttpResponse.json([
      { week_start: '2026-02-16', input_tokens: 80000, output_tokens: 24000 },
      { week_start: '2026-02-23', input_tokens: 50000, output_tokens: 15000 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/monthly`, () => {
    return HttpResponse.json([
      { month: '2026-01', input_tokens: 200000, output_tokens: 60000 },
      { month: '2026-02', input_tokens: 150000, output_tokens: 45000 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-task`, () => {
    return HttpResponse.json([
      { task_id: 'task-123', task_title: 'Test task', input_tokens: 80000, output_tokens: 24000, total_tokens: 104000 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-session`, () => {
    return HttpResponse.json([]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-tool`, () => {
    return HttpResponse.json([
      { tool_name: 'Read', input_tokens: 50000, output_tokens: 0, call_count: 120 },
      { tool_name: 'Write', input_tokens: 0, output_tokens: 24000, call_count: 45 },
      { tool_name: 'Bash', input_tokens: 10000, output_tokens: 5000, call_count: 30 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/by-language`, () => {
    return HttpResponse.json([
      { file_ext: '.rs', input_tokens: 30000, output_tokens: 12000, call_count: 80 },
      { file_ext: '.ts', input_tokens: 20000, output_tokens: 8000, call_count: 60 },
    ]);
  }),
  http.get(`${API_BASE}/api/analytics/tokens/efficiency`, () => {
    return HttpResponse.json([]);
  }),
  http.get(`${API_BASE}/api/analytics/sessions/:id/timeline`, () => {
    return HttpResponse.json([]);
  }),
  http.get(`${API_BASE}/api/analytics/cost/by-task`, () => HttpResponse.json(mockCostByTask)),
  http.get(`${API_BASE}/api/analytics/tokens/by-stage`, () => HttpResponse.json(mockTokensByStage)),
  http.get(`${API_BASE}/api/analytics/sessions/summary`, () => HttpResponse.json(mockSessionSummary)),
  http.get(`${API_BASE}/api/analytics/burn-rate`, () => HttpResponse.json(mockBurnRate)),

  // Sessions
  http.get(`${API_BASE}/api/sessions`, () => {
    return HttpResponse.json([]);
  }),
  http.post(`${API_BASE}/api/tasks/:id/sessions`, () => {
    return HttpResponse.json({ id: 'sess-123', task_id: 'task-123', status: 'running' }, { status: 201 });
  }),

  // Comments
  http.get(`${API_BASE}/api/tasks/:id/comments`, () => {
    return HttpResponse.json([]);
  }),
];
