import { logger } from '@/lib/logger';

function getApiBase() {
  if (typeof window !== 'undefined') return `http://${window.location.hostname}:3001`;
  return 'http://localhost:3001';
}

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = 'ApiError';
  }
}

export async function apiClient<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  const url = `${getApiBase()}${endpoint}`;
  const method = options?.method ?? 'GET';
  // Extract task_id / session_id from URL for richer error logs
  const taskIdMatch = endpoint.match(/\/api\/tasks\/([0-9a-f-]{36})/i);
  const sessionIdMatch = endpoint.match(/\/api\/sessions\/([0-9a-f-]{36})/i);
  const urlTaskId = taskIdMatch?.[1];
  const urlSessionId = sessionIdMatch?.[1];

  // Scoped logger carrying task_id/session_id for all log calls in this request
  const log = logger.withContext({ task_id: urlTaskId, session_id: urlSessionId });

  // Pre-fetch validation
  if (!endpoint) {
    log.error('API client called with empty endpoint', { options });
    throw new Error('API endpoint is required');
  }

  log.debug(`API request starting`, {
    method,
    endpoint,
    url,
    hasBody: !!options?.body,
  });

  try {
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      const errorText = await response.text().catch(() => 'Unknown error');
      log.error(`API error response`, {
        method,
        endpoint,
        status: response.status,
        errorText,
      });
      throw new ApiError(response.status, errorText);
    }

    log.debug(`API request succeeded`, {
      method,
      endpoint,
      status: response.status,
    });

    // 204/205 No Content — skip JSON parse (e.g. DELETE responses)
    if (response.status === 204 || response.status === 205) {
      return undefined as T;
    }
    return response.json();

  } catch (error) {
    // Re-throw ApiError as-is (already logged above)
    if (error instanceof ApiError) throw error;

    log.error(`API network error`, {
      method,
      endpoint,
      url,
      errorType: error?.constructor?.name ?? 'Unknown',
      errorMessage: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
    });
    throw error;
  }
}
