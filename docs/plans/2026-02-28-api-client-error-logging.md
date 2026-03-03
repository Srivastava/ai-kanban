# API Client Error Logging Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive error logging to the frontend API client so all network and fetch errors are captured in the database and visible in the `/logs` tab.

**Architecture:** Wrap the `fetch` call in try/catch, add input validation, and log all error cases using the existing `logger`. Logs flow to `POST /api/logs` → database → `/logs` tab.

**Tech Stack:** TypeScript, existing logger, fetch API

---

## Task 1: Update api-client.ts with Error Logging

**Files:**
- Modify: `frontend/src/lib/api-client.ts`

**Step 1: Add try/catch and validation**

Replace the entire `apiClient` function with:

```typescript
export async function apiClient<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
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
```

**Step 2: Verify TypeScript compiles**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit
```

Expected: No errors

**Step 3: Run existing tests**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npm test -- src/lib/api-client.test.ts
```

Expected: All 5 tests pass

**Step 4: Commit**

```bash
cd /home/utility/Projects/ai-kanban
git add frontend/src/lib/api-client.ts
git commit -m "feat(api-client): add comprehensive error logging

- Add try/catch around fetch call
- Log network errors with full context
- Add pre-fetch validation for empty endpoint
- Log debug info on request start and success

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

Single task that adds error logging to the API client. All errors will now:
1. Be logged via `logger.error()`
2. Be shipped to `POST /api/logs`
3. Appear in the `/logs` tab with `source: "frontend"`
