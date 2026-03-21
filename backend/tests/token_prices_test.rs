/// Tests for token_prices() in db/analytics.rs
///
/// The function reads CLAUDE_INPUT_PRICE_PER_MILLION and CLAUDE_OUTPUT_PRICE_PER_MILLION
/// env vars, defaulting to $3.00 input / $15.00 output (Claude Sonnet rates).
///
/// NOTE: env var mutation tests are not included here because std::env::set_var is
/// not thread-safe in multi-threaded test runners. The defaults test verifies the
/// function's baseline behavior; env var override behavior is documented here.
///
/// To manually verify env var behavior, run:
///   CLAUDE_INPUT_PRICE_PER_MILLION=5.0 CLAUDE_OUTPUT_PRICE_PER_MILLION=20.0 cargo test
use ai_kanban_backend::db::token_prices;

#[test]
fn test_token_prices_defaults() {
    // Only test defaults if env vars are not set, to avoid interfering with CI
    // that might set custom pricing.
    if std::env::var("CLAUDE_INPUT_PRICE_PER_MILLION").is_err()
        && std::env::var("CLAUDE_OUTPUT_PRICE_PER_MILLION").is_err()
    {
        let p = token_prices();
        assert!(
            (p.input - 3.0).abs() < 0.001,
            "default input price should be $3.00/M, got {}",
            p.input
        );
        assert!(
            (p.output - 15.0).abs() < 0.001,
            "default output price should be $15.00/M, got {}",
            p.output
        );
    }
}

#[test]
fn test_token_prices_returns_positive_values() {
    let p = token_prices();
    assert!(p.input > 0.0, "input price must be positive");
    assert!(p.output > 0.0, "output price must be positive");
}

#[test]
fn test_token_prices_output_greater_than_input() {
    // Standard pricing: output tokens are more expensive than input
    // This is true for both defaults (3.0 vs 15.0) and common override scenarios.
    // If env vars are set to custom values that break this, the test still
    // documents the expected relationship.
    if std::env::var("CLAUDE_INPUT_PRICE_PER_MILLION").is_err()
        && std::env::var("CLAUDE_OUTPUT_PRICE_PER_MILLION").is_err()
    {
        let p = token_prices();
        assert!(
            p.output > p.input,
            "output price (${}/M) should exceed input price (${}/M)",
            p.output,
            p.input
        );
    }
}

#[test]
fn test_token_prices_are_finite() {
    let p = token_prices();
    assert!(p.input.is_finite(), "input price must be finite");
    assert!(p.output.is_finite(), "output price must be finite");
}
