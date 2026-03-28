use crate::db::{OtelMetricsRepository, TokenEventRepository};
use axum::{extract::State, response::IntoResponse};
use tracing::warn;

#[derive(Clone)]
pub struct PrometheusState {
    pub otel_repo: OtelMetricsRepository,
    pub token_events: TokenEventRepository,
}

pub async fn metrics_handler(State(state): State<PrometheusState>) -> impl IntoResponse {
    let mut out = String::new();

    // --- Cost (from otel_metrics) ---
    match sqlx_sum_metric(&state.otel_repo, "claude_code.cost.usage").await {
        Ok(val) => {
            out.push_str(
                "# HELP claude_code_cost_usd_total Total USD cost from Claude Code sessions\n",
            );
            out.push_str("# TYPE claude_code_cost_usd_total counter\n");
            out.push_str(&format!("claude_code_cost_usd_total {val}\n\n"));
        }
        Err(e) => warn!(error = %e, "Failed to query cost metric"),
    }

    // --- Tokens (from token_events, better correlated) ---
    match token_totals(&state.token_events).await {
        Ok((input, output, cache_create, cache_read)) => {
            out.push_str("# HELP claude_code_tokens_total Total tokens used by Claude Code\n");
            out.push_str("# TYPE claude_code_tokens_total counter\n");
            out.push_str(&format!(
                "claude_code_tokens_total{{type=\"input\"}} {input}\n"
            ));
            out.push_str(&format!(
                "claude_code_tokens_total{{type=\"output\"}} {output}\n"
            ));
            out.push_str(&format!(
                "claude_code_tokens_total{{type=\"cache_creation\"}} {cache_create}\n"
            ));
            out.push_str(&format!(
                "claude_code_tokens_total{{type=\"cache_read\"}} {cache_read}\n\n"
            ));
        }
        Err(e) => warn!(error = %e, "Failed to query token events"),
    }

    // --- Sessions (from otel_metrics) ---
    match sqlx_sum_metric(&state.otel_repo, "claude_code.session.count").await {
        Ok(val) => {
            out.push_str("# HELP claude_code_sessions_total Total Claude Code sessions\n");
            out.push_str("# TYPE claude_code_sessions_total counter\n");
            out.push_str(&format!("claude_code_sessions_total {val}\n\n"));
        }
        Err(e) => warn!(error = %e, "Failed to query session count"),
    }

    // --- Commits (from otel_metrics) ---
    match sqlx_sum_metric(&state.otel_repo, "claude_code.commit.count").await {
        Ok(val) => {
            out.push_str(
                "# HELP claude_code_commits_total Total git commits made by Claude Code\n",
            );
            out.push_str("# TYPE claude_code_commits_total counter\n");
            out.push_str(&format!("claude_code_commits_total {val}\n\n"));
        }
        Err(e) => warn!(error = %e, "Failed to query commit count"),
    }

    // --- Pull Requests (from otel_metrics) ---
    match sqlx_sum_metric(&state.otel_repo, "claude_code.pull_request.count").await {
        Ok(val) => {
            out.push_str(
                "# HELP claude_code_prs_total Total pull requests created by Claude Code\n",
            );
            out.push_str("# TYPE claude_code_prs_total counter\n");
            out.push_str(&format!("claude_code_prs_total {val}\n\n"));
        }
        Err(e) => warn!(error = %e, "Failed to query PR count"),
    }

    // --- Lines of code (from otel_metrics, split by type attribute) ---
    match lines_of_code_totals(&state.otel_repo).await {
        Ok((added, removed)) => {
            out.push_str(
                "# HELP claude_code_lines_of_code_total Lines of code changed by Claude Code\n",
            );
            out.push_str("# TYPE claude_code_lines_of_code_total counter\n");
            out.push_str(&format!(
                "claude_code_lines_of_code_total{{type=\"added\"}} {added}\n"
            ));
            out.push_str(&format!(
                "claude_code_lines_of_code_total{{type=\"removed\"}} {removed}\n\n"
            ));
        }
        Err(e) => warn!(error = %e, "Failed to query lines of code"),
    }

    // --- Active time (from otel_metrics) ---
    match sqlx_sum_metric(&state.otel_repo, "claude_code.active_time.total").await {
        Ok(val) => {
            out.push_str(
                "# HELP claude_code_active_time_seconds_total Total active time in seconds\n",
            );
            out.push_str("# TYPE claude_code_active_time_seconds_total counter\n");
            out.push_str(&format!("claude_code_active_time_seconds_total {val}\n\n"));
        }
        Err(e) => warn!(error = %e, "Failed to query active time"),
    }

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    )
}

async fn sqlx_sum_metric(repo: &OtelMetricsRepository, metric_name: &str) -> anyhow::Result<f64> {
    repo.sum_metric(metric_name).await
}

async fn token_totals(repo: &TokenEventRepository) -> anyhow::Result<(f64, f64, f64, f64)> {
    repo.aggregate_totals().await
}

async fn lines_of_code_totals(repo: &OtelMetricsRepository) -> anyhow::Result<(f64, f64)> {
    repo.sum_lines_of_code().await
}
