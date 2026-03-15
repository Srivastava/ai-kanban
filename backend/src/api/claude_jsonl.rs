/// Parses ~/.claude/projects/**/*.jsonl files to compute real token usage
/// matching what `claude /usage` reports.
///
/// Claude Code rate limits track **output_tokens** per 5-hour window and per week.
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ClaudeUsage {
    pub tokens_5hr: i64,
    pub tokens_week: i64,
    /// Timestamp of earliest message in the 5hr window (for reset calculation)
    pub earliest_5hr: Option<DateTime<Utc>>,
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

    // Start of current week (Monday 00:00 UTC)
    let days_from_monday = now.weekday().num_days_from_monday() as i64;
    let week_start = (now - Duration::days(days_from_monday))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| Utc.from_utc_datetime(&dt))
        .unwrap_or(now);

    let mut usage = ClaudeUsage::default();

    let Ok(project_entries) = std::fs::read_dir(&projects_dir) else {
        return usage;
    };

    for project_entry in project_entries.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }
        let Ok(file_entries) = std::fs::read_dir(&project_path) else {
            continue;
        };
        for file_entry in file_entries.flatten() {
            let path = file_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            parse_jsonl_file(&path, window_5hr_start, week_start, &mut usage);
        }
    }

    usage
}

fn parse_jsonl_file(
    path: &PathBuf,
    window_5hr_start: DateTime<Utc>,
    week_start: DateTime<Utc>,
    usage: &mut ClaudeUsage,
) {
    let Ok(file) = File::open(path) else { return };
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };

        // Only assistant messages carry usage data
        let Some(msg) = v.get("message") else { continue };
        let Some(usage_obj) = msg.get("usage") else { continue };

        let ts_str = match v.get("timestamp").and_then(|t| t.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let Ok(ts) = DateTime::parse_from_rfc3339(ts_str).map(|t| t.with_timezone(&Utc)) else {
            continue;
        };

        // Output tokens are the primary rate-limit metric
        let output = usage_obj
            .get("output_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        if output == 0 {
            continue;
        }

        if ts >= window_5hr_start {
            usage.tokens_5hr += output;
            if usage.earliest_5hr.map_or(true, |e| ts < e) {
                usage.earliest_5hr = Some(ts);
            }
        }
        if ts >= week_start {
            usage.tokens_week += output;
        }
    }
}

/// Calculate the ISO-8601 reset time for the 5hr window:
/// = earliest event in window + 5 hours
pub fn reset_5hr_from_earliest(earliest: Option<DateTime<Utc>>) -> Option<String> {
    earliest.map(|e| (e + Duration::hours(5)).to_rfc3339())
}

/// Next Monday 00:00 UTC
pub fn next_monday_reset() -> String {
    let now = Utc::now();
    let days_until_monday = (7 - now.weekday().num_days_from_monday() as i64) % 7;
    let days_until_monday = if days_until_monday == 0 { 7 } else { days_until_monday };
    let next_monday = (now + Duration::days(days_until_monday))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| Utc.from_utc_datetime(&dt))
        .unwrap_or(now);
    next_monday.to_rfc3339()
}
