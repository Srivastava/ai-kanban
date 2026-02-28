# API Client Error Logging Design

**Goal:** Add comprehensive error logging to the frontend API client so all errors are captured in the database and visible in the `/logs` tab.

## Problem

The `api-client.ts` throws errors without logging them when:
1. Network errors occur (backend down, CORS, DNS failures)
2. The `fetch` call itself fails

Current state:
- HTTP 4xx/5xx responses ARE logged
- Network/fetch errors are NOT logged
- No validation of inputs before fetch

## Solution

Wrap the `fetch` call in try/catch with logging at every failure point.

## Implementation

### Changes to `src/lib/api-client.ts`

```typescript
export async function apiClient<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const url = `${API_BASE}${endpoint}`;
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

    logger.debug(`API request succeeded`, { method, endpoint, status: response.status });
    return response.json();

  } catch (error) {
    // Re-throw ApiError as-is (already logged)
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
```

## Log Flow

1. Frontend calls `apiClient()`
2. On error, `logger.error()` is called
3. Logger batches and sends to `POST /api/logs`
4. Backend stores in database with `source: "frontend"`
5. `/logs` tab fetches and displays the error

## Testing

- Verify network errors appear in `/logs` tab
- Verify HTTP errors still appear in `/logs` tab
- Verify debug logs appear (when level filter allows DEBUG)
