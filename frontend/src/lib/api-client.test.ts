import { describe, it, expect } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/msw/server';
import { apiClient, ApiError } from './api-client';

describe('apiClient', () => {
  it('returns parsed JSON on success', async () => {
    server.use(
      http.get('/api/test', () =>
        HttpResponse.json({ ok: true })
      )
    );

    const result = await apiClient<{ ok: boolean }>('/api/test');
    expect(result.ok).toBe(true);
  });

  it('sends Content-Type: application/json header', async () => {
    let capturedContentType: string | null = null;

    server.use(
      http.post('/api/test', ({ request }) => {
        capturedContentType = request.headers.get('content-type');
        return HttpResponse.json({ ok: true }, { status: 201 });
      })
    );

    await apiClient('/api/test', { method: 'POST', body: JSON.stringify({}) });
    expect(capturedContentType).toBe('application/json');
  });

  it('throws ApiError with status on 4xx response', async () => {
    server.use(
      http.get('/api/notfound', () =>
        HttpResponse.json({ error: 'Not found' }, { status: 404 })
      )
    );

    await expect(apiClient('/api/notfound')).rejects.toThrow(ApiError);
    await expect(apiClient('/api/notfound')).rejects.toMatchObject({ status: 404 });
  });

  it('throws ApiError with status on 5xx response', async () => {
    server.use(
      http.get('/api/broken', () =>
        HttpResponse.json({ error: 'Internal' }, { status: 500 })
      )
    );

    await expect(apiClient('/api/broken')).rejects.toThrow(ApiError);
    await expect(apiClient('/api/broken')).rejects.toMatchObject({ status: 500 });
  });

  it('merges extra headers with Content-Type', async () => {
    let capturedAuthHeader: string | null = null;

    server.use(
      http.get('/api/auth', ({ request }) => {
        capturedAuthHeader = request.headers.get('x-custom');
        return HttpResponse.json({});
      })
    );

    await apiClient('/api/auth', { headers: { 'x-custom': 'value' } });
    expect(capturedAuthHeader).toBe('value');
  });
});
