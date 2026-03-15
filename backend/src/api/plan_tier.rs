/// Returns the active Claude plan tier and its rate-limit values.
///
/// Precedence:
///   1. CLAUDE_5HR_TOKEN_LIMIT env var (> 0) — user-configured, always wins
///   2. CLAUDE_PLAN_TIER env var ("pro" | "max5" | "max20")
///   3. Hard-coded Pro defaults
pub fn plan_tier_from_env() -> crate::models::PlanTier {
    // Check for user-configured explicit limits first
    let explicit_5hr: i64 = std::env::var("CLAUDE_5HR_TOKEN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let explicit_week: i64 = std::env::var("CLAUDE_WEEKLY_TOKEN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if explicit_5hr > 0 || explicit_week > 0 {
        return crate::models::PlanTier {
            tier: "custom".to_string(),
            limit_5hr: explicit_5hr,
            limit_week: explicit_week,
        };
    }

    // Map tier name to defaults
    let tier_name = std::env::var("CLAUDE_PLAN_TIER")
        .unwrap_or_else(|_| "pro".to_string())
        .to_lowercase();

    // Limits are output-token counts (what Claude Code /usage tracks).
    // Pro: ~350K output tokens / 5hr, ~3.5M / week (derived empirically).
    // max5 / max20 scale proportionally per Anthropic tier documentation.
    match tier_name.as_str() {
        "max5"  => crate::models::PlanTier { tier: "max5".to_string(),  limit_5hr: 1_750_000,  limit_week: 17_500_000 },
        "max20" => crate::models::PlanTier { tier: "max20".to_string(), limit_5hr: 7_000_000,  limit_week: 70_000_000 },
        _       => crate::models::PlanTier { tier: "pro".to_string(),   limit_5hr: 350_000,    limit_week: 3_500_000  },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_to_pro() {
        std::env::remove_var("CLAUDE_PLAN_TIER");
        std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
        std::env::remove_var("CLAUDE_WEEKLY_TOKEN_LIMIT");
        let t = plan_tier_from_env();
        assert_eq!(t.tier, "pro");
        assert_eq!(t.limit_5hr, 350_000);
        assert_eq!(t.limit_week, 3_500_000);
    }

    #[test]
    fn test_explicit_5hr_limit_wins() {
        std::env::set_var("CLAUDE_5HR_TOKEN_LIMIT", "50000");
        std::env::remove_var("CLAUDE_PLAN_TIER");
        let t = plan_tier_from_env();
        assert_eq!(t.tier, "custom");
        assert_eq!(t.limit_5hr, 50_000);
        std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
    }

    #[test]
    fn test_max5_tier() {
        std::env::remove_var("CLAUDE_5HR_TOKEN_LIMIT");
        std::env::remove_var("CLAUDE_WEEKLY_TOKEN_LIMIT");
        std::env::set_var("CLAUDE_PLAN_TIER", "max5");
        let t = plan_tier_from_env();
        assert_eq!(t.tier, "max5");
        assert_eq!(t.limit_5hr, 1_750_000);
        std::env::remove_var("CLAUDE_PLAN_TIER");
    }
}
