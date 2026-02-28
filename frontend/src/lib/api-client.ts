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

  // Pre-fetch validation
  if (!endpoint) {
    logger.error('API client called with empty endpoint', { options });
    throw new Error('API endpoint is required');
  }

  logger.debug(`API request starting`, {
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
      logger.error(`API error response`, {
        method,
        endpoint,
        status: response.status,
        errorText,
      });
      throw new ApiError(response.status, errorText);
    }

    logger.debug(`API request succeeded`, {
      method,
      endpoint,
      status: response.status,
    });
    return response.json();

  } catch (error) {
    // Re-throw ApiError as-is (already logged above)
    if (error instanceof ApiError) throw error;

    // Log network/unexpected errors
    logger.error(`API network error`, {
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
