# Design: Bug Fixes & Feature — 2026-03-16

## Scope

Five bug fixes and one new feature for the ai-kanban project.

---

## 1. Usage Windows Card: Stale Seed, 0% Merge Bug & Rate-Limit-Aware Polling

### Problem

- `claude_usage_cli.rs` seeds the cache with hardcoded past dates (`reset_5hr: "2026-03-15T13:01:00+00:00"`). When the daemon can't fetch fresh data, the UI shows `"resetting…"` because the reset time is in the past.
- The merge logic treats `pct_5hr == 0.0` as "no data", so a legitimately 0% window never updates the cache — it keeps the stale seed value of 5%.
- When `claude /usage` is rate-limited, the daemon continues polling on the fixed 10min/1hr schedule, wasting polls during the blackout window.

### Design

**Backend (`claude_usage_cli.rs`)**

- Change `pct_5hr` and `pct_week` in `ClaudeCliUsage` from `f64` to `Option<f64>`. `Some(0.0)` = successfully read 0%; `None` = parse failed / rate limited.
- Remove all hardcoded dates from the seed. Seed starts as `ClaudeCliUsage::default()` (all `None`). Frontend shows `"—"` until the first successful poll completes.
- Update merge logic: only fall back to cached value when the new field is `None`. If `Some(0.0)`, use it.
- Add `no_data: bool` field to `ClaudeCliUsage`. Set `true` when the Python script runs but produces no parseable output (covers rate limiting, claude not installed, or output format changes). The frontend label for this state is `"Rate limited"` (the most common cause in practice). A `last_successful_poll_at: Option<Instant>` is also stored in `UsageCache` to allow distinguishing "never had data" from "had data, now unavailable."
- **Rate-limit-aware sleep:** after a rate-limited poll, compute `sleep_duration = reset_5hr_utc - Utc::now()`. If `reset_5hr` is known and in the future, sleep until then — **capped at 6 hours maximum** to prevent a stale cached value from silencing the daemon for a full day. Otherwise fall back to 1hr.

**API response (`/api/analytics/usage-windows`)**

- Add `no_data: bool` field to the `UsageWindows` JSON response so the frontend can react. This field is `true` when the most recent poll returned no parseable data AND the daemon has had at least one prior successful poll. When `no_data` is `true` and `last_successful_poll_at` exists, the frontend shows `"Rate limited"`. **When the JSONL fallback path is active (daemon has never had data), `no_data` must be `false`** — the JSONL data is serving the response and there is nothing to flag as unavailable.
- Update `backend/src/api/analytics.rs`: the existing `if cli.pct_5hr > 0.0 || cli.pct_week > 0.0` guard must be updated to pattern-match on `Option<f64>` (e.g. `cli.pct_5hr.is_some() || cli.pct_week.is_some()`).

**Frontend (`usage-windows-card.tsx`)**

- When `data.no_data === true` and there has been a prior successful poll, display `"Rate limited"` as the countdown text for the 5hr window instead of `"resetting…"`.
- When `reset_5hr` is `null`, show `"—"` / `"no data yet"` (initial state before first poll).
- When `reset_5hr` is in the past and `no_data` is false, show `"resetting…"` as currently (window just reset, waiting for next poll to confirm new values).

---

## 2. Token Counting: Include Cache Tokens (Display & Cost)

### Problem

Claude API uses prompt caching. `input_tokens` in each event is only the non-cached tokens — which can be as small as 1.6K while `output_tokens` is 200K. `cache_read_input_tokens` and `cache_creation_input_tokens` are stored in the DB but excluded from all analytics queries, making the input/output ratio appear absurd.

### Design

**Backend models** — add fields to affected structs in `models/analytics.rs`:
- `TaskTokens`, `SessionTokens`, `AnalyticsOverview`, `DailyTokens`, `WeeklyTokens`, `MonthlyTokens`:
  - Add `cache_creation_tokens: i64`
  - Add `cache_read_tokens: i64`

**Backend queries** (`db/analytics.rs`) — for every query that sums `input_tokens`/`output_tokens`, also sum `cache_creation_tokens` and `cache_read_tokens`.

**Cost calculation** — update `estimated_cost_usd` calculation:
- Input (non-cached): `input_tokens * input_price / 1_000_000`
- Cache creation: `cache_creation_tokens * input_price / 1_000_000` (same rate as input)
- Cache read: `cache_read_tokens * (input_price * 0.10) / 1_000_000` (10% of input price)
- Output: `output_tokens * output_price / 1_000_000`

**Frontend display**

- `active-sessions-panel.tsx`: change `(in↑ out↓)` to `(in | cached | out)` three-part format.
- Analytics overview cards: show cache creation and cache read totals alongside input/output.
- `total_tokens` remains `input + output` (non-cached billed tokens) for rate-limit purposes. **Cache tokens must never be included in any calculation that feeds the usage-window rate-limit percentage** (the `tokens_5hr`/`tokens_week` fields in `UsageWindows`). Add `effective_tokens = input + cache_creation + cache_read + output` for context-window size display where relevant.

---

## 3. Missing Replies in Resume Mode

### Problem

In `continue_session`, when a cutoff timestamp is set (resume mode), the filter is:
```
comments where parent.created_at > cutoff
```
Replies added to an *old* parent comment after the session ended are silently dropped — their parent's `created_at` is before the cutoff, so the parent (and its replies) are excluded entirely.

### Design

**Backend (`api/tasks.rs`, `continue_session`)**

Change the context-building logic when `cutoff` is set:

1. For each comment (regardless of its `created_at`), collect replies whose `reply.created_at > cutoff`.
2. If a comment has such replies: include the parent comment content as context (prefixed with `[context]`) plus **only the replies whose `reply.created_at > cutoff`** (reply-level date filter also applies).
3. Also include any top-level comment whose own `created_at > cutoff` as before (including all its non-litellm replies unconditionally, since the parent itself is "new").
4. De-duplicate: use a `HashSet<comment_id>` to track already-included parent comments. Process rule 3 first (new top-level comments), then rule 2 (old parents with new replies) — skipping any parent already in the set.
5. When `cutoff` is `None` (no true resume), behaviour is unchanged: all non-litellm comments are included with all their replies.

This ensures no user reply is ever dropped due to its parent being "old."

---

## 4. Active Session Definition (30-Minute Window)

### Problem

`active_count()` only counts processes currently running in memory. The moment a session finishes, it drops to zero — so the usage daemon immediately slows to 1hr polling even if a session ended 2 minutes ago, and `active_sessions_today` counts any token event from today rather than "recently active."

### Design

**Backend (`claude/queue.rs` or `claude/manager.rs`)**

- Add `last_session_ended_at: Arc<RwLock<Option<Instant>>>` to `ClaudeManager`.
- Set it to `Instant::now()` whenever a session completes (in the completion task).
- Add `pub async fn recently_active(&self) -> bool` — returns `true` if a session is currently running OR ended within the last 30 minutes.
- **Known limitation:** `Instant` does not persist across process restarts. After a server restart, `recently_active()` returns `false` immediately even if a session ended 5 minutes before the restart. This is acceptable — the daemon will fall back to 1hr polling after restart, which is conservative but not harmful.

**Usage daemon (`claude_usage_cli.rs`)**

- Replace the `active_count() > 0` check with `recently_active()`.
- When `recently_active()` returns true, use the 10-minute polling interval.

**Analytics (`db/analytics.rs`)**

- Fix `active_sessions_today` query: change from counting token events by calendar day to counting sessions from the `sessions` table where `started_at >= DATE('now')` (i.e., started today). This keeps the metric name accurate. The 30-minute window is only used for the daemon polling decision (`recently_active()`), not for this metric.

---

## 5. Project Path Combobox (New Feature)

### Problem

The task creation dialog has a free-text input for the project path. Users must type full absolute paths manually. All projects live under `~/Projects/`, but there's no guidance or discovery.

### Design

**New backend endpoint** `GET /api/fs/projects`

- Reads `~/Projects/` (expanding `~` to `$HOME`).
- Returns a JSON array of immediate subdirectory names (not full paths): `["ai-kanban", "my-app", ...]`.
- Sorted alphabetically.
- Returns `[]` if the directory doesn't exist or is unreadable (never errors).

**Backend: directory auto-creation**

- In `create_task` handler: if `project_path` starts with `~/Projects/` and the directory doesn't exist: expand `~` to `$HOME`, then **walk the path components** and reject any component that is `..` or contains `/` or `\` (returning 400). Do this check on the raw input string *before* the directory exists — do not rely on `std::fs::canonicalize` which errors on non-existent paths. After the component check passes, call `std::fs::create_dir_all(expanded_path)`. Log a warning if creation fails but don't block task creation.

**Frontend (`create-task-dialog.tsx`)**

- Replace the plain `<Input>` for project path with a combobox:
  - Text input field showing only the directory name (not the full path).
  - Dropdown below showing existing subdirs from `GET /api/fs/projects`, filtered as the user types.
  - The actual value sent to the backend is always `~/Projects/<input-value>`.
- **Info hint** below the input: *"Enter a directory name under `~/Projects/`. New directories are created automatically."*
- **Fallback**: if the endpoint fails, the combobox degrades to a plain text input.

---

## Files Changed (Summary)

### Backend
- `backend/src/api/claude_usage_cli.rs` — Option<f64> pcts, remove seed dates, rate-limit sleep, `rate_limited` flag
- `backend/src/models/analytics.rs` — add cache fields to token structs
- `backend/src/db/analytics.rs` — include cache tokens in all queries, update cost calc, update active_sessions_today
- `backend/src/models/mod.rs` — re-export new fields
- `backend/src/api/analytics.rs` — pass `no_data` through to `UsageWindows` response; update `Option<f64>` guard replacing the old `> 0.0` comparison
- `backend/src/api/tasks.rs` — fix resume-mode reply filtering; auto-create project dir
- `backend/src/claude/manager.rs` — add `last_session_ended_at`, `recently_active()`
- `backend/src/claude/queue.rs` — expose `recently_active()`
- `backend/src/api/mod.rs` — register new `/api/fs/projects` route; add `pub mod fs;` module declaration
- `backend/src/api/fs.rs` (new) — `GET /api/fs/projects` handler

### Frontend
- `frontend/src/components/analytics/usage-windows-card.tsx` — rate_limited text, null reset handling
- `frontend/src/components/logs/active-sessions-panel.tsx` — `in | cached | out` display
- `frontend/src/components/analytics/overview-cards.tsx` — cache token totals
- `frontend/src/types/analytics.ts` — add cache fields, rate_limited flag
- `frontend/src/components/tasks/create-task-dialog.tsx` — combobox with ~/Projects/ base
- `frontend/src/lib/api-client.ts` — add `getProjects()` helper function
