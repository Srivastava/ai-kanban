import { logger } from '@/lib/logger';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

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
  logger.debug(`API request: ${options?.method ?? 'GET'} ${endpoint}`, {
    method: options?.method ?? 'GET',
    endpoint,
  });

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const errorText = await response.text().catch(() => 'Unknown error');
    logger.error(`API error: ${options?.method ?? 'GET'} ${endpoint} → ${response.status}`, {
      endpoint,
      status: response.status,
      message: errorText,
    });
    throw new ApiError(response.status, errorText);
  }

  return response.json();
}
