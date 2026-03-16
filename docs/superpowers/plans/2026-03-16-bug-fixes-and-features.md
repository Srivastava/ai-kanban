# Bug Fixes & Features Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 5 bugs (stale usage seed, wrong token ratios, missing replies in resume, active-session definition) and add a project-path combobox to task creation.

**Architecture:** Backend fixes are in Rust/Axum (`backend/src/`); frontend in Next.js (`frontend/src/`). No new DB migrations required — `cache_read_tokens` and `cache_creation_tokens` already exist in `token_events`. Changes are ordered: data-model first, then behavior, then UI.

**Tech Stack:** Rust/Axum/SQLite/sqlx (backend), Next.js/TanStack Query/React Testing Library/Vitest (frontend).

**Spec:** `docs/superpowers/specs/2026-03-16-bug-fixes-and-features-design.md`

---

## Chunk 1: Backend — Usage Daemon & Analytics Models

---

### Task 1: Fix `ClaudeCliUsage` struct and parser

**Files:**
- Modify: `backend/src/api/claude_usage_cli.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)]` block in `claude_usage_cli.rs`:

```rust
#[test]
fn test_parse_returns_some_pcts_when_data_present() {
    let raw = "Current session\n████ 88% used\nResets 1am (America/Los_Angeles)\nCurrent week (all models)\n██ 15% used\nResets Mar 21, 11am (America/Los_Angeles)\n";
    let u = parse_claude_usage_output(raw);
    assert_eq!(u.pct_5hr, Some(88.0));
    assert_eq!(u.pct_week, Some(15.0));
}

#[test]
fn test_parse_returns_none_pcts_when_no_data() {
    let u = parse_claude_usage_output("");
    assert_eq!(u.pct_5hr, None);
    assert_eq!(u.pct_week, None);
    assert!(u.reset_5hr.is_none());
    assert!(u.reset_week.is_none());
}

#[test]
fn test_parse_zero_pct_returns_some_zero() {
    let raw = "Current session\n0% used\nResets 1am (America/Los_Angeles)\nCurrent week\n0% used\n";
    let u = parse_claude_usage_output(raw);
    assert_eq!(u.pct_5hr, Some(0.0));
    assert_eq!(u.pct_week, Some(0.0));
}
```

- [ ] **Step 2: Run tests — expect compile error** (field types don't match yet)

```bash
cd /home/utility/Projects/ai-kanban/backend
cargo test test_parse_returns_some_pcts_when_data_present 2>&1 | head -20
```

- [ ] **Step 3: Change `ClaudeCliUsage` struct fields to `Option<f64>`**

Replace the struct definition:

```rust
#[derive(Debug, Default, Clone)]
pub struct ClaudeCliUsage {
    /// 5-hour window usage (0.0–100.0), None if not parseable
    pub pct_5hr: Option<f64>,
    /// Weekly window usage (0.0–100.0), None if not parseable
    pub pct_week: Option<f64>,
    /// ISO-8601 UTC reset time for 5hr window
    pub reset_5hr: Option<String>,
    /// ISO-8601 UTC reset time for weekly window
    pub reset_week: Option<String>,
}
```

- [ ] **Step 4: Update `parse_claude_usage_output` to return `Option<f64>`**

Change the pct extraction lines (the `pct_re.captures_iter` calls) from:
```rust
let pct_5hr = pct_re.captures_iter(section_5hr)
    .filter_map(|c| c[1].parse::<f64>().ok())
    .next()
    .unwrap_or(0.0);
```
To:
```rust
let pct_5hr: Option<f64> = pct_re.captures_iter(section_5hr)
    .filter_map(|c| c[1].parse::<f64>().ok())
    .next();

let pct_week: Option<f64> = pct_re.captures_iter(section_week)
    .filter_map(|c| c[1].parse::<f64>().ok())
    .next();
```

The return line is already `ClaudeCliUsage { pct_5hr, pct_week, reset_5hr, reset_week }` — no change needed there.

- [ ] **Step 5: Fix existing test that used `f64` equality**

Update the existing `test_parse_sample_output` test:
```rust
assert_eq!(u.pct_5hr, Some(88.0));
assert_eq!(u.pct_week, Some(15.0));
```

- [ ] **Step 6: Run tests — expect pass**

```bash
cargo test -p backend -- claude_usage_cli 2>&1 | tail -10
```
Expected: all 4 tests pass.

- [ ] **Step 7: Commit**

```bash
git add backend/src/api/claude_usage_cli.rs
git commit -m "fix: change ClaudeCliUsage pcts to Option<f64> to distinguish 0% from no-data"
```

---

### Task 2: Fix daemon — remove stale seed, fix merge, add rate-limit sleep

**Files:**
- Modify: `backend/src/api/claude_usage_cli.rs`

- [ ] **Step 1: Update `UsageCache` struct**

Replace the `UsageCache` struct:

```rust
#[derive(Clone, Debug, Default)]
pub struct UsageCache {
    pub data: ClaudeCliUsage,
    /// Set when the last successful poll completed
    pub fetched_at: Option<Instant>,
    /// True when the most recent poll returned no parseable data
    pub last_poll_no_data: bool,
}
```

- [ ] **Step 2: Replace `start_usage_daemon` body**

Replace the entire function body of `start_usage_daemon` with:

```rust
pub fn start_usage_daemon(queue: Option<Arc<SessionQueue>>) -> SharedUsageCache {
    // Start empty — no hardcoded seed dates.
    // The first poll runs immediately and populates the cache within seconds.
    let cache: SharedUsageCache = Arc::new(RwLock::new(UsageCache::default()));
    let cache_clone = cache.clone();

    tokio::spawn(async move {
        loop {
            let result = tokio::task::spawn_blocking(run_claude_usage).await;

            let has_data = match &result {
                Ok(new) => {
                    new.pct_5hr.is_some() || new.pct_week.is_some()
                        || new.reset_5hr.is_some() || new.reset_week.is_some()
                }
                Err(_) => false,
            };

            if let Ok(new) = result {
                if let Ok(mut c) = cache_clone.write() {
                    c.last_poll_no_data = !has_data;
                    if has_data {
                        let merged = ClaudeCliUsage {
                            pct_5hr: new.pct_5hr.or(c.data.pct_5hr),
                            pct_week: new.pct_week.or(c.data.pct_week),
                            reset_5hr: new.reset_5hr.or_else(|| c.data.reset_5hr.clone()),
                            reset_week: new.reset_week.or_else(|| c.data.reset_week.clone()),
                        };
                        info!(
                            pct_5hr = ?merged.pct_5hr,
                            pct_week = ?merged.pct_week,
                            "Usage daemon: refreshed from claude /usage"
                        );
                        c.data = merged;
                        c.fetched_at = Some(Instant::now());
                    } else {
                        warn!("Usage daemon: no parseable data (rate limited or error); keeping cached value");
                    }
                }
            } else {
                warn!("Usage daemon: spawn_blocking failed");
                if let Ok(mut c) = cache_clone.write() {
                    c.last_poll_no_data = true;
                }
            }

            // Dynamic interval:
            // - Rate limited: sleep until the 5hr reset (capped at 6h)
            // - Active session (running or ended <30min ago): 10 min
            // - Idle: 1 hour
            let interval_secs: u64 = if !has_data {
                // Rate limited — sleep until reset_5hr if known, else 1h
                let reset_str = cache_clone.read().ok()
                    .and_then(|c| c.data.reset_5hr.clone());
                if let Some(s) = reset_str {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                        let secs = (dt.with_timezone(&Utc) - Utc::now()).num_seconds();
                        if secs > 60 {
                            info!("Usage daemon: rate limited, sleeping {}s (capped at 6h)", secs.min(6 * 3600));
                            secs.min(6 * 3600) as u64
                        } else {
                            3600
                        }
                    } else {
                        3600
                    }
                } else {
                    3600
                }
            } else {
                match &queue {
                    Some(q) if q.active_count().await > 0 => {
                        info!("Usage daemon: active session, polling again in 10m");
                        600
                    }
                    _ => {
                        info!("Usage daemon: idle, polling again in 1h");
                        3600
                    }
                }
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
        }
    });

    cache
}
```

Note: `recently_active()` replaces `active_count() > 0` in Task 5 — for now we keep the existing check.

- [ ] **Step 3: Run existing tests**

```bash
cargo test -p backend -- claude_usage_cli 2>&1 | tail -15
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add backend/src/api/claude_usage_cli.rs
git commit -m "fix: remove stale seed dates, fix merge logic, add rate-limit-aware sleep in usage daemon"
```

---

### Task 3: Update `UsageWindows` model and analytics API handler

**Files:**
- Modify: `backend/src/models/analytics.rs`
- Modify: `backend/src/api/analytics.rs`
- Modify: `frontend/src/types/analytics.ts` (partial — just `UsageWindows`)

- [ ] **Step 1: Update `UsageWindows` model**

In `backend/src/models/analytics.rs`, find the `UsageWindows` struct (around the 80-line mark). Make two changes: change `reset_week: String` to `reset_week: Option<String>` (it is currently `String`, not `Option<String>`), and add the `no_data` field:

```rust
pub struct UsageWindows {
    pub tokens_5hr: i64,
    pub tokens_week: i64,
    pub limit_5hr: i64,
    pub limit_week: i64,
    pub reset_5hr: Option<String>,
    pub reset_week: Option<String>,  // ← was String, now Option<String>
    pub no_data: bool,               // ← add this
}
```

- [ ] **Step 2: Update the `usage_windows` handler in `analytics.rs`**

Open `backend/src/api/analytics.rs`. Find the `usage_windows` function. Replace the `if cli.pct_5hr > 0.0 || cli.pct_week > 0.0` branch with `Option<f64>`-aware logic and pass `no_data`:

```rust
async fn usage_windows(State(state): State<AnalyticsApiState>) -> impl IntoResponse {
    info!("API: Getting usage windows from daemon cache");
    let plan = crate::api::plan_tier::plan_tier_from_env();

    let (cli, last_poll_no_data, has_prior_data) = state
        .usage_cache
        .read()
        .map(|c| (c.data.clone(), c.last_poll_no_data, c.fetched_at.is_some()))
        .unwrap_or_default();

    // no_data = true only when: last poll failed AND we had a prior successful poll
    // (if daemon never had data, JSONL fallback serves the response — no_data stays false)
    let no_data = last_poll_no_data && has_prior_data;

    let (tokens_5hr, tokens_week, reset_5hr, reset_week) =
        if cli.pct_5hr.is_some() || cli.pct_week.is_some() {
            let t5 = cli.pct_5hr
                .map(|p| ((p / 100.0) * plan.limit_5hr as f64).round() as i64)
                .unwrap_or(0);
            let tw = cli.pct_week
                .map(|p| ((p / 100.0) * plan.limit_week as f64).round() as i64)
                .unwrap_or(0);
            let r5 = cli.reset_5hr;
            let rw = cli.reset_week.unwrap_or_else(|| {
                let j = crate::api::claude_jsonl::read_claude_usage();
                crate::api::claude_jsonl::reset_week_from_earliest(j.earliest_week)
            });
            (t5, tw, r5, Some(rw))
        } else {
            // Daemon hasn't gotten data yet — use JSONL
            let j = crate::api::claude_jsonl::read_claude_usage();
            let r5 = crate::api::claude_jsonl::reset_5hr_from_earliest(j.earliest_5hr);
            let rw = crate::api::claude_jsonl::reset_week_from_earliest(j.earliest_week);
            (j.tokens_5hr, j.tokens_week, r5, Some(rw))
        };

    let windows = crate::models::UsageWindows {
        tokens_5hr,
        tokens_week,
        limit_5hr: plan.limit_5hr,
        limit_week: plan.limit_week,
        reset_5hr,
        reset_week,
        no_data,
    };
    Json(windows).into_response()
}
```

Note: `reset_week` in the model is `Option<String>` — check the existing model definition and update it to `Option<String>` if it is currently `String`.

- [ ] **Step 3: Add `no_data` to frontend `UsageWindows` type**

In `frontend/src/types/analytics.ts`, update:

```typescript
export interface UsageWindows {
  tokens_5hr: number;
  tokens_week: number;
  limit_5hr: number;
  limit_week: number;
  reset_5hr: string | null;
  reset_week: string | null;
  no_data: boolean;
}
```

- [ ] **Step 4: Compile check**

```bash
cargo build -p backend 2>&1 | grep -E "error|warning" | head -20
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add backend/src/models/analytics.rs backend/src/api/analytics.rs frontend/src/types/analytics.ts
git commit -m "fix: pass no_data flag through usage-windows API; fix Option<f64> guard"
```

---

### Task 4: Add cache token fields to analytics models and DB queries

**Files:**
- Modify: `backend/src/models/analytics.rs`
- Modify: `backend/src/db/analytics.rs`

- [ ] **Step 1: Add cache fields to token structs in `models/analytics.rs`**

Add `cache_creation_tokens: i64` and `cache_read_tokens: i64` to these structs:

```rust
pub struct AnalyticsOverview {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_creation_tokens: i64,  // ← add
    pub total_cache_read_tokens: i64,      // ← add
    pub total_sessions: i64,
    pub total_tasks_with_sessions: i64,
    pub estimated_cost_usd: f64,
    pub active_sessions_today: i64,
}

pub struct DailyTokens {
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,  // ← add
    pub cache_read_tokens: i64,      // ← add
}

pub struct WeeklyTokens {
    pub week_start: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,  // ← add
    pub cache_read_tokens: i64,      // ← add
}

pub struct MonthlyTokens {
    pub month: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,  // ← add
    pub cache_read_tokens: i64,      // ← add
}

pub struct TaskTokens {
    pub task_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,  // ← add
    pub cache_read_tokens: i64,      // ← add
    pub total_tokens: i64,           // stays: input + output only
}

pub struct SessionTokens {
    pub session_id: String,
    pub task_title: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,  // ← add
    pub cache_read_tokens: i64,      // ← add
    pub total_tokens: i64,           // stays: input + output only
    pub started_at: Option<String>,
}
```

- [ ] **Step 2: Update `overview()` query in `db/analytics.rs`**

Find the `SELECT COALESCE(SUM(input_tokens), 0)...` overview query and expand it:

```sql
SELECT
    COALESCE(SUM(input_tokens), 0) as input_tokens,
    COALESCE(SUM(output_tokens), 0) as output_tokens,
    COALESCE(SUM(cache_creation_tokens), 0) as cache_creation_tokens,
    COALESCE(SUM(cache_read_tokens), 0) as cache_read_tokens
FROM token_events
```

Update the Rust extraction and cost calc:

```rust
let total_input: i64 = totals.get("input_tokens");
let total_output: i64 = totals.get("output_tokens");
let total_cache_creation: i64 = totals.get("cache_creation_tokens");
let total_cache_read: i64 = totals.get("cache_read_tokens");

let (input_price, output_price) = token_prices();
let estimated_cost_usd =
    (total_input as f64 / 1_000_000.0) * input_price
    + (total_output as f64 / 1_000_000.0) * output_price
    + (total_cache_creation as f64 / 1_000_000.0) * input_price
    + (total_cache_read as f64 / 1_000_000.0) * (input_price * 0.10);
```

Update `AnalyticsOverview` construction:
```rust
Ok(AnalyticsOverview {
    total_input_tokens: total_input,
    total_output_tokens: total_output,
    total_cache_creation_tokens: total_cache_creation,
    total_cache_read_tokens: total_cache_read,
    total_sessions: session_count,
    total_tasks_with_sessions: task_count,
    estimated_cost_usd,
    active_sessions_today,
})
```

- [ ] **Step 3: Fix `active_sessions_today` query**

Replace:
```sql
SELECT COUNT(DISTINCT session_id) as count FROM token_events WHERE DATE(timestamp) = DATE('now')
```
With:
```sql
SELECT COUNT(*) as count FROM sessions WHERE DATE(started_at) = DATE('now')
```

- [ ] **Step 4: Update `daily_tokens` query**

Add cache columns (same pattern for weekly and monthly):
```sql
SELECT
    DATE(timestamp) as date,
    SUM(input_tokens) as input_tokens,
    SUM(output_tokens) as output_tokens,
    SUM(cache_creation_tokens) as cache_creation_tokens,
    SUM(cache_read_tokens) as cache_read_tokens
FROM token_events
WHERE DATE(timestamp) >= DATE('now', ?)
GROUP BY DATE(timestamp)
ORDER BY date ASC
```

Update the `.map()` to include the new fields:
```rust
DailyTokens {
    date: row.get("date"),
    input_tokens: row.get("input_tokens"),
    output_tokens: row.get("output_tokens"),
    cache_creation_tokens: row.get("cache_creation_tokens"),
    cache_read_tokens: row.get("cache_read_tokens"),
}
```

Repeat the same pattern for `weekly_tokens()` and `monthly_tokens()`.

- [ ] **Step 5: Update `tokens_by_task` query**

```sql
SELECT
    te.task_id,
    COALESCE(t.title, 'Unknown Task') as task_title,
    SUM(te.input_tokens) as input_tokens,
    SUM(te.output_tokens) as output_tokens,
    SUM(te.cache_creation_tokens) as cache_creation_tokens,
    SUM(te.cache_read_tokens) as cache_read_tokens
FROM token_events te
LEFT JOIN tasks t ON te.task_id = t.id
GROUP BY te.task_id
ORDER BY (SUM(te.input_tokens) + SUM(te.output_tokens)) DESC
```

Update the map:
```rust
let input: i64 = row.get("input_tokens");
let output: i64 = row.get("output_tokens");
let cache_creation: i64 = row.get("cache_creation_tokens");
let cache_read: i64 = row.get("cache_read_tokens");
TaskTokens {
    task_id: row.get("task_id"),
    task_title: row.get("task_title"),
    input_tokens: input,
    output_tokens: output,
    cache_creation_tokens: cache_creation,
    cache_read_tokens: cache_read,
    total_tokens: input + output,  // cache excluded from total
}
```

- [ ] **Step 6: Update `tokens_by_session` query** — same pattern as above.

- [ ] **Step 7: Compile check**

```bash
cargo build -p backend 2>&1 | grep "error" | head -20
```
Expected: no errors. Fix any field-name mismatches the compiler reports.

- [ ] **Step 8: Commit**

```bash
git add backend/src/models/analytics.rs backend/src/db/analytics.rs
git commit -m "feat: add cache_creation/cache_read token fields to all analytics queries and models"
```

---

## Chunk 2: Backend — Behavior Fixes & FS Feature

---

### Task 5: Add `recently_active()` to `ClaudeManager` and update daemon

**Files:**
- Modify: `backend/src/claude/manager.rs`
- Modify: `backend/src/claude/queue.rs`
- Modify: `backend/src/api/claude_usage_cli.rs`

- [ ] **Step 1: Add `last_session_ended_at` field to `ClaudeManager`**

The existing `active_sessions` field uses `tokio::sync::RwLock` (async). For `last_session_ended_at` we must use `std::sync::RwLock` (blocking) — the `recently_active()` method uses `.read()` which returns `Result` (std) not a future (tokio).

Add this import near the top of `manager.rs` (alongside existing `use std::time::Instant`):
```rust
use std::sync::RwLock as StdRwLock;
```

Add to the struct:
```rust
pub struct ClaudeManager {
    active_sessions: Arc<RwLock<HashMap<String, RunningSession>>>,  // tokio RwLock
    last_session_ended_at: Arc<StdRwLock<Option<Instant>>>,         // ← add (std RwLock)
    output_tx: broadcast::Sender<ClaudeEvent>,
    // ... rest unchanged
}
```

In the `new()` constructor, add:
```rust
Self {
    active_sessions: Arc::new(RwLock::new(HashMap::new())),
    last_session_ended_at: Arc::new(StdRwLock::new(None)),  // ← add
    // ... rest unchanged
}
```

- [ ] **Step 2: Clone and set `last_session_ended_at` in `start_session`**

In `start_session`, near the other `_for_completion` clones (~line 525), add:
```rust
let last_ended_for_completion = self.last_session_ended_at.clone();
```

In the completion task, after `sessions.remove(&session_id_for_completion)` (~line 544), add:
```rust
if let Ok(mut ts) = last_ended_for_completion.write() {
    *ts = Some(Instant::now());
}
```

- [ ] **Step 3: Add `recently_active()` method**

After the `active_count()` method (~line 822), add:

```rust
/// Returns true if a session is currently running OR ended within the last 30 minutes.
/// Note: resets to false on server restart (Instant is not persisted).
pub async fn recently_active(&self) -> bool {
    // active_sessions uses tokio RwLock → .read().await
    if self.active_sessions.read().await.len() > 0 {
        return true;
    }
    // last_session_ended_at uses std RwLock → .read() returns Result
    if let Ok(ts) = self.last_session_ended_at.read() {
        if let Some(ended) = *ts {
            return ended.elapsed() < std::time::Duration::from_secs(30 * 60);
        }
    }
    false
}
```

- [ ] **Step 4: Expose `recently_active()` on `SessionQueue`**

In `backend/src/claude/queue.rs`, after the `active_count()` method:

```rust
pub async fn recently_active(&self) -> bool {
    self.manager.recently_active().await
}
```

- [ ] **Step 5: Update daemon to use `recently_active()`**

In `backend/src/api/claude_usage_cli.rs`, replace:
```rust
Some(q) if q.active_count().await > 0 => {
    info!("Usage daemon: active session, polling again in 10m");
    600
}
```
With:
```rust
Some(q) if q.recently_active().await => {
    info!("Usage daemon: recently active, polling again in 10m");
    600
}
```

- [ ] **Step 6: Compile check**

```bash
cargo build -p backend 2>&1 | grep "error" | head -20
```

- [ ] **Step 7: Commit**

```bash
git add backend/src/claude/manager.rs backend/src/claude/queue.rs backend/src/api/claude_usage_cli.rs
git commit -m "feat: add recently_active() with 30-min window; update usage daemon polling"
```

---

### Task 6: Fix missing replies in `continue_session`

**Files:**
- Modify: `backend/src/api/tasks.rs`

- [ ] **Step 1: Write a unit test for the comment context builder**

Add a `#[cfg(test)]` block at the bottom of `backend/src/api/tasks.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::build_comment_history;
    use crate::models::{Comment, CommentWithReplies};
    use chrono::{DateTime, Utc, TimeZone};

    fn make_comment(id: &str, author: &str, content: &str, ts: DateTime<Utc>) -> Comment {
        Comment {
            id: id.to_string(),
            task_id: "t1".to_string(),
            parent_id: None,
            author: author.to_string(),
            content: content.to_string(),
            created_at: ts,
        }
    }

    fn make_reply(id: &str, parent_id: &str, author: &str, content: &str, ts: DateTime<Utc>) -> Comment {
        Comment {
            id: id.to_string(),
            task_id: "t1".to_string(),
            parent_id: Some(parent_id.to_string()),
            author: author.to_string(),
            content: content.to_string(),
            created_at: ts,
        }
    }

    #[test]
    fn test_no_cutoff_includes_all_comments_and_replies() {
        let t = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let comments = vec![
            CommentWithReplies {
                comment: make_comment("c1", "user", "hello", t),
                replies: vec![make_reply("r1", "c1", "claude", "hi", t)],
            },
        ];
        let result = build_comment_history(&comments, None).unwrap();
        assert!(result.contains("[You]: hello"));
        assert!(result.contains("[Claude]: hi"));
    }

    #[test]
    fn test_cutoff_excludes_old_parent_with_no_new_replies() {
        let old = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();
        let comments = vec![
            CommentWithReplies {
                comment: make_comment("c1", "user", "old comment", old),
                replies: vec![],
            },
        ];
        let result = build_comment_history(&comments, Some(cutoff));
        assert!(result.is_none());
    }

    #[test]
    fn test_cutoff_includes_old_parent_as_context_when_new_reply_exists() {
        let old = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let new_ts = Utc.with_ymd_and_hms(2026, 3, 3, 10, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();
        let comments = vec![
            CommentWithReplies {
                comment: make_comment("c1", "user", "old parent", old),
                replies: vec![make_reply("r1", "c1", "user", "new reply", new_ts)],
            },
        ];
        let result = build_comment_history(&comments, Some(cutoff)).unwrap();
        assert!(result.contains("[context]"), "Should include parent as context");
        assert!(result.contains("old parent"));
        assert!(result.contains("new reply"));
    }

    #[test]
    fn test_cutoff_excludes_old_reply_on_old_parent() {
        let old = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();
        let comments = vec![
            CommentWithReplies {
                comment: make_comment("c1", "user", "old parent", old),
                replies: vec![make_reply("r1", "c1", "user", "old reply", old)],
            },
        ];
        let result = build_comment_history(&comments, Some(cutoff));
        assert!(result.is_none());
    }

    #[test]
    fn test_litellm_comments_excluded() {
        let t = Utc.with_ymd_and_hms(2026, 3, 3, 10, 0, 0).unwrap();
        let comments = vec![
            CommentWithReplies {
                comment: make_comment("c1", "litellm", "summary", t),
                replies: vec![],
            },
        ];
        let result = build_comment_history(&comments, None);
        assert!(result.is_none());
    }
}
```

- [ ] **Step 2: Run — expect compile error** (function doesn't exist yet)

```bash
cargo test -p backend -- tasks::tests 2>&1 | head -20
```

- [ ] **Step 3: Extract `build_comment_history` helper function**

First, update the module-level `use` declarations at the top of `tasks.rs` (around line 2). Add `CommentWithReplies` to the existing models import and add `HashSet`:

```rust
use crate::models::{CommentWithReplies, CreateTask, UpdateTask};
use std::collections::HashSet;
```

Then add the helper function just above the `continue_session` handler:

```rust
/// Build the comment context string for injecting into a Claude session.
///
/// cutoff=None: include all non-litellm comments and their replies.
/// cutoff=Some(dt):
///   - Rule 3 (first): include comments where parent.created_at > dt (with all their replies).
///   - Rule 2 (second): for any remaining parent with replies where reply.created_at > dt,
///     include the parent as `[context]` plus only the qualifying replies.
pub fn build_comment_history(
    comments: &[CommentWithReplies],
    cutoff: Option<chrono::DateTime<chrono::Utc>>,
) -> Option<String> {
    fn fmt(author: &str, content: &str) -> String {
        let prefix = if author == "claude" { "[Claude]" } else { "[You]" };
        format!("{}: {}", prefix, content)
    }

    let lines: Vec<String> = match cutoff {
        None => comments
            .iter()
            .filter(|c| c.comment.author != "litellm")
            .flat_map(|c| {
                let mut ls = vec![fmt(&c.comment.author, &c.comment.content)];
                for r in c.replies.iter().filter(|r| r.author != "litellm") {
                    ls.push(format!("  {}", fmt(&r.author, &r.content)));
                }
                ls
            })
            .collect(),

        Some(cutoff_dt) => {
            let mut included_ids: HashSet<String> = HashSet::new();
            let mut ls: Vec<String> = Vec::new();

            // Rule 3: new top-level comments (parent.created_at > cutoff)
            for c in comments
                .iter()
                .filter(|c| c.comment.author != "litellm" && c.comment.created_at > cutoff_dt)
            {
                included_ids.insert(c.comment.id.clone());
                ls.push(fmt(&c.comment.author, &c.comment.content));
                for r in c.replies.iter().filter(|r| r.author != "litellm") {
                    ls.push(format!("  {}", fmt(&r.author, &r.content)));
                }
            }

            // Rule 2: old parents that have new replies
            for c in comments
                .iter()
                .filter(|c| c.comment.author != "litellm" && !included_ids.contains(&c.comment.id))
            {
                let new_replies: Vec<_> = c
                    .replies
                    .iter()
                    .filter(|r| r.author != "litellm" && r.created_at > cutoff_dt)
                    .collect();
                if !new_replies.is_empty() {
                    ls.push(format!("[context] {}", fmt(&c.comment.author, &c.comment.content)));
                    for r in &new_replies {
                        ls.push(format!("  {}", fmt(&r.author, &r.content)));
                    }
                }
            }

            ls
        }
    };

    if lines.is_empty() { None } else { Some(lines.join("\n")) }
}
```

- [ ] **Step 4: Replace the inline comment-building logic in `continue_session`**

Find the block starting at `let new_comment_count = human_comments.len();` and replace from there down to `let conversation_context = ...`:

```rust
let comment_history = build_comment_history(&comments, cutoff);
let new_comment_count = comment_history.as_ref().map(|h| h.lines().count()).unwrap_or(0);

let conversation_context = if resume_claude_session_id.is_some() {
    comment_history.map(|h| format!("## New messages since last session:\n{h}"))
} else {
    match (&task.compressed_context, &comment_history) {
        (Some(compressed), Some(history)) => Some(format!(
            "## Prior session context (compressed):\n{compressed}\n\n## Recent conversation:\n{history}"
        )),
        (Some(compressed), None) => Some(format!(
            "## Prior session context (compressed):\n{compressed}"
        )),
        (None, Some(history)) => Some(history.clone()),
        (None, None) => None,
    }
};
```

Remove the now-unused `human_comments` binding (the `let human_comments: Vec<_> = comments.iter()...collect();` block) to avoid a compiler warning. `total_comments` and `total_replies` can stay as they are still used in the `info!()` log. Update `new_comment_count` in the log to use the new variable.

- [ ] **Step 5: Run tests**

```bash
cargo test -p backend -- tasks::tests 2>&1 | tail -15
```
Expected: all 5 tests pass.

- [ ] **Step 6: Compile**

```bash
cargo build -p backend 2>&1 | grep "error" | head -10
```

- [ ] **Step 7: Commit**

```bash
git add backend/src/api/tasks.rs
git commit -m "fix: include replies to old parent comments in resume-mode context (cutoff-aware)"
```

---

### Task 7: `GET /api/fs/projects` endpoint

**Files:**
- Create: `backend/src/api/fs.rs`
- Modify: `backend/src/api/mod.rs`
- Modify: `backend/src/api/routes.rs`

- [ ] **Step 1: Create `backend/src/api/fs.rs`**

```rust
//! Filesystem utility routes — e.g. listing ~/Projects/ subdirectories.
use axum::{response::IntoResponse, Json};
use tracing::warn;

/// GET /api/fs/projects
/// Returns a sorted list of immediate subdirectory names under ~/Projects/.
/// Returns [] if the directory doesn't exist or is unreadable.
pub async fn list_projects() -> impl IntoResponse {
    let home = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(h) => h,
        Err(_) => {
            warn!("HOME not set — cannot list ~/Projects/");
            return Json(Vec::<String>::new()).into_response();
        }
    };

    let projects_dir = std::path::PathBuf::from(home).join("Projects");
    let mut dirs: Vec<String> = Vec::new();

    match std::fs::read_dir(&projects_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        dirs.push(name.to_string());
                    }
                }
            }
        }
        Err(e) => {
            warn!(path = %projects_dir.display(), error = %e, "Cannot read ~/Projects/");
        }
    }

    dirs.sort();
    Json(dirs).into_response()
}
```

- [ ] **Step 2: Register the module in `backend/src/api/mod.rs`**

Add near the top with the other `pub mod` declarations (search for existing ones like `pub mod tasks`):

```rust
pub mod fs;
```

- [ ] **Step 3: Register the route in `backend/src/api/routes.rs`**

Add the import at the top:
```rust
use crate::api::fs::list_projects;
```

Add the route inside `create_router`:
```rust
.route("/api/fs/projects", axum::routing::get(list_projects))
```

- [ ] **Step 4: Compile check**

```bash
cargo build -p backend 2>&1 | grep "error" | head -10
```

- [ ] **Step 5: Smoke test** (with the backend running)

```bash
curl -s http://localhost:3001/api/fs/projects | head -5
```
Expected: JSON array of directory names.

- [ ] **Step 6: Commit**

```bash
git add backend/src/api/fs.rs backend/src/api/mod.rs backend/src/api/routes.rs
git commit -m "feat: add GET /api/fs/projects endpoint listing ~/Projects/ subdirs"
```

---

### Task 8: Auto-create project directory in `create_task`

**Files:**
- Modify: `backend/src/api/tasks.rs`

- [ ] **Step 1: Add directory-creation logic to `create_task` handler**

In the `create_task` function, add path validation and creation before `state.repo.create(create).await`. Insert after the `info!()` log line:

```rust
// Auto-create ~/Projects/<name> if the path starts with ~/Projects/
if let Some(name) = create.project_path.strip_prefix("~/Projects/") {
    // Security: reject any path component that could escape the Projects directory
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid project path: use a simple directory name under ~/Projects/" })),
        ).into_response();
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let full_path = std::path::PathBuf::from(home).join("Projects").join(name);
        if !full_path.exists() {
            if let Err(e) = std::fs::create_dir_all(&full_path) {
                warn!(path = %full_path.display(), error = %e, "Failed to create project directory");
            } else {
                info!(path = %full_path.display(), "Created project directory");
            }
        }
    }
}
```

- [ ] **Step 2: Compile check**

```bash
cargo build -p backend 2>&1 | grep "error" | head -10
```

- [ ] **Step 3: Commit**

```bash
git add backend/src/api/tasks.rs
git commit -m "feat: auto-create ~/Projects/<name> when task is created with that path"
```

---

## Chunk 3: Frontend — Types & Components

---

### Task 9: Update remaining TypeScript analytics types

**Files:**
- Modify: `frontend/src/types/analytics.ts`

- [ ] **Step 1: Add cache fields to token interfaces**

Update (note: `UsageWindows` was already updated in Task 3):

```typescript
export interface AnalyticsOverview {
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_creation_tokens: number;  // ← add
  total_cache_read_tokens: number;      // ← add
  total_sessions: number;
  total_tasks_with_sessions: number;
  estimated_cost_usd: number;
  active_sessions_today: number;
}

export interface DailyTokens {
  date: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;  // ← add
  cache_read_tokens: number;      // ← add
}

export interface WeeklyTokens {
  week_start: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;  // ← add
  cache_read_tokens: number;      // ← add
}

export interface MonthlyTokens {
  month: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;  // ← add
  cache_read_tokens: number;      // ← add
}

export interface TaskTokens {
  task_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;  // ← add
  cache_read_tokens: number;      // ← add
  total_tokens: number;
}

export interface SessionTokens {
  session_id: string;
  task_title: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;  // ← add
  cache_read_tokens: number;      // ← add
  total_tokens: number;
  started_at: string | null;
}
```

- [ ] **Step 2: Type-check**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx tsc --noEmit 2>&1 | head -30
```
Expected: errors only in component files that now use the types — those will be fixed in subsequent tasks.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/types/analytics.ts
git commit -m "feat: add cache token fields to analytics TypeScript types"
```

---

### Task 10: Fix `usage-windows-card.tsx`

**Files:**
- Modify: `frontend/src/components/analytics/usage-windows-card.tsx`

- [ ] **Step 1: Write a failing component test**

Create `frontend/src/components/analytics/usage-windows-card.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { UsageWindowsCard } from './usage-windows-card';

vi.mock('@/hooks/use-analytics', () => ({
  useUsageWindows: () => ({
    data: {
      tokens_5hr: 0,
      tokens_week: 0,
      limit_5hr: 0,
      limit_week: 0,
      reset_5hr: null,
      reset_week: null,
      no_data: true,
    },
    isLoading: false,
    isFetching: false,
    refetch: vi.fn(),
    dataUpdatedAt: Date.now(),
  }),
}));

describe('UsageWindowsCard', () => {
  it('shows Rate limited when no_data is true and reset is null', () => {
    render(<UsageWindowsCard />);
    expect(screen.getByText('Rate limited')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run — expect fail**

```bash
cd /home/utility/Projects/ai-kanban/frontend
npx vitest run src/components/analytics/usage-windows-card.test.tsx 2>&1 | tail -15
```

- [ ] **Step 3: Update `formatReset` and the card component**

In `usage-windows-card.tsx`, update `formatReset` to accept a `noData` flag and return "Rate limited":

```tsx
function formatReset(
  iso: string | null,
  noData: boolean,
): { label: string; countdown: string } {
  if (noData) return { label: '—', countdown: 'Rate limited' };
  if (!iso) return { label: '—', countdown: 'No data yet' };
  const d = new Date(iso);
  const label = new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
    timeZoneName: 'short',
  }).format(d);

  const msLeft = d.getTime() - Date.now();
  if (msLeft <= 0) return { label, countdown: 'resetting…' };
  const h = Math.floor(msLeft / 3_600_000);
  const m = Math.floor((msLeft % 3_600_000) / 60_000);
  const countdown = h > 0 ? `${h}h ${m}m` : `${m}m`;
  return { label, countdown };
}
```

Update the two `formatReset` call sites in `UsageWindowsCard`:

```tsx
const noData = data?.no_data ?? false;
const reset5hr = formatReset(data?.reset_5hr ?? null, noData);
const resetWeek = formatReset(data?.reset_week ?? null, false); // weekly window not rate-limited separately
```

- [ ] **Step 4: Run test — expect pass**

```bash
npx vitest run src/components/analytics/usage-windows-card.test.tsx 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/analytics/usage-windows-card.tsx frontend/src/components/analytics/usage-windows-card.test.tsx
git commit -m "fix: show 'Rate limited' in usage-windows card when no_data is true"
```

---

### Task 11: Update `active-sessions-panel.tsx` — add cache columns

**Files:**
- Modify: `frontend/src/components/logs/active-sessions-panel.tsx`

- [ ] **Step 1: Update `SessionRow` to show `in | cached | out`**

In `SessionRow`, replace the token display cell:

```tsx
<td className="px-4 py-1.5 text-right font-mono text-[11px]">
  <span className="font-medium">{fmtTokens(session.total_tokens)}</span>
  <span className="text-muted-foreground ml-1.5">
    {fmtTokens(session.input_tokens)}↑
    {' '}
    <span className="text-amber-500/80">{fmtTokens((session.cache_creation_tokens ?? 0) + (session.cache_read_tokens ?? 0))}⚡</span>
    {' '}
    {fmtTokens(session.output_tokens)}↓
  </span>
</td>
```

Update the column header:
```tsx
<th className="text-right px-4 py-1.5 font-medium">in / cached / out</th>
```

- [ ] **Step 2: Type-check**

```bash
npx tsc --noEmit 2>&1 | grep "active-sessions" | head -10
```
Expected: no errors (new fields are already in the type from Task 9).

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/logs/active-sessions-panel.tsx
git commit -m "feat: show input/cached/output token breakdown in active sessions panel"
```

---

### Task 12: Update `overview-cards.tsx` — show cache token totals

**Files:**
- Modify: `frontend/src/components/analytics/overview-cards.tsx`

- [ ] **Step 1: Read the current file**

```bash
cat frontend/src/components/analytics/overview-cards.tsx
```

Identify where `total_input_tokens` and `total_output_tokens` are rendered, then add cache totals alongside them. For example, if there's a card showing total tokens, update it to show:

```tsx
// Where input/output totals are shown, add:
<div className="text-xs text-muted-foreground mt-1">
  <span>{formatTokens(data.total_cache_creation_tokens ?? 0)} created</span>
  {' · '}
  <span>{formatTokens(data.total_cache_read_tokens ?? 0)} read (cached)</span>
</div>
```

Adapt to match the existing card structure you see in the file.

- [ ] **Step 2: Type-check**

```bash
npx tsc --noEmit 2>&1 | grep "overview-cards" | head -10
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/analytics/overview-cards.tsx
git commit -m "feat: show cache creation and cache read totals in analytics overview cards"
```

---

### Task 13: Project path combobox in `create-task-dialog.tsx`

**Files:**
- Modify: `frontend/src/lib/api-client.ts`
- Modify: `frontend/src/components/tasks/create-task-dialog.tsx`

- [ ] **Step 1: Add `getProjects()` to `api-client.ts`**

In `frontend/src/lib/api-client.ts`, add after the existing exports:

```typescript
export async function getProjects(): Promise<string[]> {
  try {
    return await apiClient<string[]>('/api/fs/projects');
  } catch {
    return [];
  }
}
```

- [ ] **Step 2: Write a component test**

Create `frontend/src/components/tasks/create-task-dialog.test.tsx`:

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CreateTaskDialog } from './create-task-dialog';

vi.mock('@/hooks/use-tasks', () => ({
  useCreateTask: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

vi.mock('@/lib/api-client', () => ({
  getProjects: async () => ['ai-kanban', 'my-app'],
  apiClient: vi.fn(),
}));

describe('CreateTaskDialog', () => {
  it('renders the project name input', () => {
    render(<CreateTaskDialog open={true} onOpenChange={vi.fn()} />);
    expect(screen.getByLabelText(/project/i)).toBeTruthy();
  });

  it('rejects path separators in project name', () => {
    render(<CreateTaskDialog open={true} onOpenChange={vi.fn()} />);
    const input = screen.getByPlaceholderText(/e\.g\. my-app/i);
    fireEvent.change(input, { target: { value: '../evil' } });
    // Should sanitize: the value should not contain ..
    expect((input as HTMLInputElement).value).not.toContain('..');
  });
});
```

- [ ] **Step 3: Run — expect fail**

```bash
npx vitest run src/components/tasks/create-task-dialog.test.tsx 2>&1 | tail -15
```

- [ ] **Step 4: Rewrite `CreateTaskDialog` with combobox**

Replace `create-task-dialog.tsx` with:

```tsx
'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { useCreateTask } from '@/hooks/use-tasks';
import { getProjects } from '@/lib/api-client';

interface CreateTaskDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function sanitizeProjectName(raw: string): string {
  // Remove path separators and collapse .. sequences
  return raw.replace(/[/\\]/g, '').replace(/\.\./g, '');
}

export function CreateTaskDialog({ open, onOpenChange }: CreateTaskDialogProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [projectName, setProjectName] = useState('');
  const [existingProjects, setExistingProjects] = useState<string[]>([]);

  const createTask = useCreateTask();

  // Fetch existing project names when dialog opens
  useEffect(() => {
    if (open) {
      getProjects().then(setExistingProjects);
    }
  }, [open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !projectName.trim()) return;

    const sanitized = sanitizeProjectName(projectName.trim());
    if (!sanitized) return;

    await createTask.mutateAsync({
      title: title.trim(),
      description: description.trim() || undefined,
      project_path: `~/Projects/${sanitized}`,
    });

    setTitle('');
    setDescription('');
    setProjectName('');
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create New Task</DialogTitle>
            <DialogDescription>
              Add a new task to your Kanban board.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <label htmlFor="title" className="text-sm font-medium">
                Title *
              </label>
              <Input
                id="title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="Enter task title"
                required
              />
            </div>
            <div className="grid gap-2">
              <label htmlFor="description" className="text-sm font-medium">
                Description
              </label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Enter task description (optional)"
                rows={3}
              />
            </div>
            <div className="grid gap-2">
              <label htmlFor="projectName" className="text-sm font-medium">
                Project *
              </label>
              <Input
                id="projectName"
                list="projects-datalist"
                value={projectName}
                onChange={(e) => setProjectName(sanitizeProjectName(e.target.value))}
                placeholder="e.g. my-app"
                required
              />
              <datalist id="projects-datalist">
                {existingProjects.map((p) => (
                  <option key={p} value={p} />
                ))}
              </datalist>
              <p className="text-xs text-muted-foreground">
                Select an existing project or type a new name.
                New directories are created automatically under{' '}
                <code className="font-mono bg-muted px-1 rounded">~/Projects/</code>.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={!title.trim() || !projectName.trim() || createTask.isPending}
            >
              {createTask.isPending ? 'Creating...' : 'Create Task'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 5: Run test — expect pass**

```bash
npx vitest run src/components/tasks/create-task-dialog.test.tsx 2>&1 | tail -15
```

- [ ] **Step 6: Type-check the whole frontend**

```bash
npx tsc --noEmit 2>&1 | head -30
```
Expected: no errors. Fix any remaining component files that reference the new type fields.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/api-client.ts frontend/src/components/tasks/create-task-dialog.tsx frontend/src/components/tasks/create-task-dialog.test.tsx
git commit -m "feat: project path combobox with ~/Projects/ base and auto-create"
```

---

## Final Verification

- [ ] **Build both targets**

```bash
cd /home/utility/Projects/ai-kanban
cargo build -p backend 2>&1 | grep "error"
cd frontend && npx tsc --noEmit && npx vitest run 2>&1 | tail -20
```

- [ ] **Start backend and smoke-test key endpoints**

```bash
# In one terminal: cargo run -p backend
curl -s http://localhost:3001/api/analytics/usage-windows | python3 -m json.tool
curl -s http://localhost:3001/api/fs/projects
curl -s http://localhost:3001/api/analytics/overview | python3 -m json.tool
```

Expected:
- `usage-windows` includes `no_data` field
- `fs/projects` returns a JSON array
- `overview` includes `total_cache_creation_tokens` and `total_cache_read_tokens`

- [ ] **Final commit if any cleanup needed**

```bash
git add -p
git commit -m "chore: final cleanup and verification"
```
