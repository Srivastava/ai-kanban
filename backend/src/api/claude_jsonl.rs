/// Parses ~/.claude/projects/**/*.jsonl files to compute real token usage
/// matching what `claude /usage` reports.
///
/// Claude Code rate limits track **output_tokens** per 5-hour window and per week.
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ClaudeUsage {
    pub tokens_5hr: i64,
    pub tokens_week: i64,
    /// Timestamp of earliest message in the 5hr window (for reset calculation)
    pub earliest_5hr: Option<DateTime<Utc>>,
    /// Timestamp of earliest message in the 7-day rolling window (for reset calculation)
    pub earliest_week: Option<DateTime<Utc>>,
}

pub fn read_claude_usage() -> ClaudeUsage {
    let home = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(h) => h,
        Err(_) => return ClaudeUsage::default(),
    };

    let projects_dir = PathBuf::from(&home).join(".claude").join("projects");
    if !projects_dir.exists() {
        return ClaudeUsage::default();
    }

    let now = Utc::now();
    let window_5hr_start = now - Duration::hours(5);
    // Rolling 7-day window — matches what `claude /usage` reports
    let window_week_start = now - Duration::days(7);

    let mut usage = ClaudeUsage::default();

    collect_jsonl_files(
        &projects_dir,
        window_5hr_start,
        window_week_start,
        &mut usage,
    );

    usage
}

/// Recursively walk `dir`, calling `parse_jsonl_file` for every `.jsonl` found.
fn collect_jsonl_files(
    dir: &PathBuf,
    window_5hr_start: DateTime<Utc>,
    window_week_start: DateTime<Utc>,
    usage: &mut ClaudeUsage,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, window_5hr_start, window_week_start, usage);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            parse_jsonl_file(&path, window_5hr_start, window_week_start, usage);
        }
    }
}

fn parse_jsonl_file(
    path: &PathBuf,
    window_5hr_start: DateTime<Utc>,
    window_week_start: DateTime<Utc>,
    usage: &mut ClaudeUsage,
) {
    use std::collections::HashMap;

    let Ok(file) = File::open(path) else { return };
    let reader = BufReader::new(file);

    // Deduplicate by message_id (matching ccusage methodology):
    // Claude emits multiple events per message_id (streaming-start + final).
    // Keep the LAST entry per message_id to avoid double-counting.
    // Key: message_id, Value: (timestamp, total_tokens)
    let mut seen: HashMap<String, (DateTime<Utc>, i64)> = HashMap::new();

    for line in reader.lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };

        // Only assistant messages carry usage data
        let Some(msg) = v.get("message") else {
            continue;
        };
        let Some(usage_obj) = msg.get("usage") else {
            continue;
        };

        // Need a message ID for deduplication
        let Some(msg_id) = msg.get("id").and_then(|id| id.as_str()) else {
            continue;
        };

        let ts_str = match v.get("timestamp").and_then(|t| t.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let Ok(ts) = DateTime::parse_from_rfc3339(ts_str).map(|t| t.with_timezone(&Utc)) else {
            continue;
        };

        // Sum ALL token types — matches ccusage methodology
        let input = usage_obj
            .get("input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let output = usage_obj
            .get("output_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let cache_create = usage_obj
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let cache_read = usage_obj
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let total = input + output + cache_create + cache_read;

        if total == 0 {
            continue;
        }

        // Overwrite — keep last event per message_id
        seen.insert(msg_id.to_string(), (ts, total));
    }

    // Accumulate deduplicated events into usage windows
    for (ts, total) in seen.values() {
        let ts = *ts;
        let total = *total;
        if ts >= window_5hr_start {
            usage.tokens_5hr += total;
            if usage.earliest_5hr.map_or(true, |e| ts < e) {
                usage.earliest_5hr = Some(ts);
            }
        }
        if ts >= window_week_start {
            usage.tokens_week += total;
            if usage.earliest_week.map_or(true, |e| ts < e) {
                usage.earliest_week = Some(ts);
            }
        }
    }
}

/// Calculate the ISO-8601 reset time for the 5hr window:
/// = earliest event in window + 5 hours
pub fn reset_5hr_from_earliest(earliest: Option<DateTime<Utc>>) -> Option<String> {
    earliest.map(|e| (e + Duration::hours(5)).to_rfc3339())
}

/// Calculate the ISO-8601 reset time for the rolling 7-day window:
/// = earliest event in window + 7 days.
/// Falls back to next Monday 00:00 UTC if there is no usage data.
pub fn reset_week_from_earliest(earliest: Option<DateTime<Utc>>) -> String {
    if let Some(e) = earliest {
        return (e + Duration::days(7)).to_rfc3339();
    }
    // Fallback: next Monday 00:00 UTC
    let now = Utc::now();
    let days_until_monday = (7 - now.weekday().num_days_from_monday() as i64) % 7;
    let days_until_monday = if days_until_monday == 0 {
        7
    } else {
        days_until_monday
    };
    (now + Duration::days(days_until_monday))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| Utc.from_utc_datetime(&dt))
        .unwrap_or(now)
        .to_rfc3339()
}
