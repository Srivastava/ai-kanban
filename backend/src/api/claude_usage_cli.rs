/// Runs `claude /usage` in a PTY and parses its output to extract
/// the same usage data that Claude Code itself tracks.
///
/// This matches exactly what `claude /usage` reports, avoiding the
/// inaccuracies of parsing JSONL files directly.
use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Los_Angeles;
use regex::Regex;
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{info, warn};

use crate::claude::SessionQueue;

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

/// Python one-liner that opens a PTY, runs `claude /usage`, reads until it has
/// the usage data, kills the process, then writes the captured bytes to stdout.
/// Using Python's pty module is the only portable way to give Claude a PTY
/// without depending on `script` (which blocks waiting for ESC).
const CLAUDE_USAGE_PY: &str = r#"
import pty, os, re, sys, select, time, signal
output = b''
master, slave = pty.openpty()
pid = os.fork()
if pid == 0:
    os.setsid()
    os.dup2(slave, 0); os.dup2(slave, 1); os.dup2(slave, 2)
    os.close(master)
    os.execvp('claude', ['claude', '/usage'])
    os._exit(1)
os.close(slave)
deadline = time.time() + 15
while time.time() < deadline:
    r, _, _ = select.select([master], [], [], 0.3)
    if r:
        try:
            chunk = os.read(master, 4096)
            output += chunk
        except OSError:
            break
    # Stop once we have both "% used" occurrences and a reset time
    if output.count(b'% used') >= 2:
        break
try:
    os.kill(pid, signal.SIGKILL)
except OSError:
    pass
try:
    os.waitpid(pid, 0)
except OSError:
    pass
sys.stdout.buffer.write(output)
"#;

/// Run `claude /usage` in a PTY (via Python) and return parsed usage.
/// Falls back to `ClaudeCliUsage::default()` on any error.
pub fn run_claude_usage() -> ClaudeCliUsage {
    let output = Command::new("python3")
        .args(["-c", CLAUDE_USAGE_PY])
        .output();

    let bytes = match output {
        Ok(o) => o.stdout,
        Err(e) => {
            warn!("Failed to run claude /usage via python3: {e}");
            return ClaudeCliUsage::default();
        }
    };

    let raw = String::from_utf8_lossy(&bytes);
    parse_claude_usage_output(&raw)
}

/// Strip ANSI/VT escape sequences and control characters, returning clean text.
fn strip_ansi(input: &str) -> String {
    // Remove ESC[...m color codes and other CSI sequences
    let csi = Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap();
    // Remove standalone ESC followed by a single non-CSI character
    let esc = Regex::new(r"\x1b[^\x1b\[]").unwrap();
    let s = csi.replace_all(input, "");
    let s = esc.replace_all(&s, "");
    // Replace remaining control characters (except newline) with space
    s.chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect()
}

/// Parse the cleaned output of `claude /usage` into `ClaudeCliUsage`.
pub fn parse_claude_usage_output(raw: &str) -> ClaudeCliUsage {
    let clean = strip_ansi(raw);
    // Collapse repeated spaces within each line for easier regex matching
    let collapsed: String = clean
        .lines()
        .map(|l| {
            let trimmed = l.trim();
            // Collapse runs of spaces to single space
            let mut out = String::new();
            let mut prev_space = false;
            for c in trimmed.chars() {
                if c == ' ' {
                    if !prev_space {
                        out.push(' ');
                    }
                    prev_space = true;
                } else {
                    out.push(c);
                    prev_space = false;
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n");

    let pct_re = Regex::new(r"(\d+)%\s*used").unwrap();

    // Split output into the 5hr section (before "Current week") and the weekly section (after).
    let week_marker_pos = collapsed
        .find("Current week")
        .or_else(|| collapsed.find("current week"));

    let (section_5hr, section_week) = if let Some(pos) = week_marker_pos {
        (&collapsed[..pos], &collapsed[pos..])
    } else {
        (collapsed.as_str(), "")
    };

    let pct_5hr: Option<f64> = pct_re
        .captures_iter(section_5hr)
        .filter_map(|c| c[1].parse::<f64>().ok())
        .next();

    let pct_week: Option<f64> = pct_re
        .captures_iter(section_week)
        .filter_map(|c| c[1].parse::<f64>().ok())
        .next();

    // For reset times, look for any line in each section that contains am/pm or "Resets"
    let reset_5hr = section_5hr
        .lines()
        .find(|l| {
            let lo = l.to_lowercase();
            lo.contains("reset") || lo.contains("am") || lo.contains("pm")
        })
        .and_then(|l| parse_reset_time(l));

    let reset_week = section_week
        .lines()
        .find(|l| {
            let lo = l.to_lowercase();
            lo.contains("reset") || lo.contains("am") || lo.contains("pm")
        })
        .and_then(|l| parse_reset_time(l));

    ClaudeCliUsage {
        pct_5hr,
        pct_week,
        reset_5hr,
        reset_week,
    }
}

/// Parse a reset-time string into a UTC ISO-8601 timestamp.
///
/// Handles two formats Claude Code uses:
/// - "Resets 1am (America/Los_Angeles)"  — time-of-day (next occurrence)
/// - "Resets Mar 21, 11am (America/Los_Angeles)"  — specific date+time
fn parse_reset_time(line: &str) -> Option<String> {
    let now_utc = Utc::now();
    // Collapse all whitespace in the line so "1 a m" becomes "1am" etc.
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();

    // Try "Mar21,11am" style first (date + time)
    let date_time_re = Regex::new(
        r"(?i)(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)(\d{1,2}),(\d{1,2})(am|pm)",
    )
    .unwrap();
    if let Some(caps) = date_time_re.captures(&compact) {
        let month_str = &caps[1];
        let day: u32 = caps[2].parse().ok()?;
        let hour: u32 = caps[3].parse().ok()?;
        let ampm = &caps[4];

        let month = month_to_u32(month_str)?;
        let year = if now_utc.month() > month || (now_utc.month() == month && now_utc.day() > day) {
            now_utc.year() + 1
        } else {
            now_utc.year()
        };
        let hour24 = to_24h(hour, ampm);
        let naive = NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour24, 0, 0)?;
        let la_dt = Los_Angeles.from_local_datetime(&naive).single()?;
        return Some(la_dt.with_timezone(&Utc).to_rfc3339());
    }

    // Try "1am" or "11pm" style (time-of-day only, next occurrence)
    let time_re = Regex::new(r"(?i)Resets.*?(\d{1,2})(am|pm)").unwrap();
    if let Some(caps) = time_re.captures(line) {
        let hour: u32 = caps[1].parse().ok()?;
        let ampm = &caps[2];
        let hour24 = to_24h(hour, ampm);

        // Try compact form if normal line didn't match
        let la_now = now_utc.with_timezone(&Los_Angeles);
        let naive_today = la_now.date_naive().and_hms_opt(hour24, 0, 0)?;
        let la_today = Los_Angeles.from_local_datetime(&naive_today).single()?;
        // If that time is in the past (or within 1 min), use tomorrow
        let candidate = if la_today.with_timezone(&Utc) <= now_utc + Duration::minutes(1) {
            let naive_tomorrow =
                (la_now.date_naive() + Duration::days(1)).and_hms_opt(hour24, 0, 0)?;
            Los_Angeles.from_local_datetime(&naive_tomorrow).single()?
        } else {
            la_today
        };
        return Some(candidate.with_timezone(&Utc).to_rfc3339());
    }

    // Try compact version of time-of-day
    let compact_time_re = Regex::new(r"(?i)(\d{1,2})(am|pm)").unwrap();
    if let Some(caps) = compact_time_re.captures(&compact) {
        let hour: u32 = caps[1].parse().ok()?;
        let ampm = &caps[2];
        let hour24 = to_24h(hour, ampm);

        let la_now = now_utc.with_timezone(&Los_Angeles);
        let naive_today = la_now.date_naive().and_hms_opt(hour24, 0, 0)?;
        let la_today = Los_Angeles.from_local_datetime(&naive_today).single()?;
        let candidate = if la_today.with_timezone(&Utc) <= now_utc + Duration::minutes(1) {
            let naive_tomorrow =
                (la_now.date_naive() + Duration::days(1)).and_hms_opt(hour24, 0, 0)?;
            Los_Angeles.from_local_datetime(&naive_tomorrow).single()?
        } else {
            la_today
        };
        return Some(candidate.with_timezone(&Utc).to_rfc3339());
    }

    None
}

fn month_to_u32(m: &str) -> Option<u32> {
    match m.to_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn to_24h(hour: u32, ampm: &str) -> u32 {
    match ampm.to_lowercase().as_str() {
        "am" => {
            if hour == 12 {
                0
            } else {
                hour
            }
        }
        _ => {
            if hour == 12 {
                12
            } else {
                hour + 12
            }
        }
    }
}

/// Cached result from the last successful `claude /usage` poll.
#[derive(Clone, Debug, Default)]
pub struct UsageCache {
    pub data: ClaudeCliUsage,
    /// Set when the last successful poll completed
    pub fetched_at: Option<Instant>,
    /// True when the most recent poll returned no parseable data
    pub last_poll_no_data: bool,
}

pub type SharedUsageCache = Arc<RwLock<UsageCache>>;

/// Create the shared cache and spawn the background polling daemon.
///
/// Poll frequency is dynamic:
/// - Rate limited: sleep until reset_5hr (capped at 6h)
/// - 10 minutes when there is an active Claude session (fast feedback)
/// - 1 hour when idle (avoid rate limits)
///
/// Smart field-level caching: reset times and percentages from the previous
/// successful poll are preserved when the new poll returns missing/zero values.
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
                    new.pct_5hr.is_some()
                        || new.pct_week.is_some()
                        || new.reset_5hr.is_some()
                        || new.reset_week.is_some()
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
                let reset_str = cache_clone
                    .read()
                    .ok()
                    .and_then(|c| c.data.reset_5hr.clone());
                if let Some(s) = reset_str {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                        let secs = (dt.with_timezone(&Utc) - Utc::now()).num_seconds();
                        if secs > 60 {
                            info!(
                                "Usage daemon: rate limited, sleeping {}s (capped at 6h)",
                                secs.min(6 * 3600)
                            );
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
                    Some(q) if q.recently_active().await => {
                        info!("Usage daemon: recently active, polling again in 10m");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_output() {
        let raw = "\
Current session
████████████████████████████████████████████ 88% used
Rese s 1 m (America/Los_Angeles)
Current week (all models)
███████▌ 15% used
Resets Apr 21, 11am (America/Los_Angeles)
";
        let u = parse_claude_usage_output(raw);
        assert_eq!(u.pct_5hr, Some(88.0));
        assert_eq!(u.pct_week, Some(15.0));
        assert!(u.reset_week.is_some());
        // reset_week should be April 21 (year-agnostic — date resolves to next occurrence)
        let rw = u.reset_week.unwrap();
        assert!(rw.contains("-04-21"), "got: {rw}");
    }

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
        let raw =
            "Current session\n0% used\nResets 1am (America/Los_Angeles)\nCurrent week\n0% used\n";
        let u = parse_claude_usage_output(raw);
        assert_eq!(u.pct_5hr, Some(0.0));
        assert_eq!(u.pct_week, Some(0.0));
    }
}
