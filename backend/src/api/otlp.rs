use crate::api::otlp_parser::parse_otlp_metrics;
use crate::db::{OtelMetricsRepository, SessionRepository};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::Value;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct OtlpState {
    pub otel_repo: OtelMetricsRepository,
    pub session_repo: SessionRepository,
}

/// POST /v1/metrics — OTLP/HTTP JSON metrics
pub async fn receive_metrics(
    State(state): State<OtlpState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let metrics = parse_otlp_metrics(&body);
    let count = metrics.len();
    debug!(count, "Received OTLP metrics batch");

    for mut m in metrics {
        if !m.claude_session_id.is_empty() {
            match state.session_repo.find_by_claude_session_id(&m.claude_session_id).await {
                Ok(Some(session)) => {
                    m.session_id = Some(session.id.clone());
                    m.task_id = Some(session.task_id.clone());
                }
                Ok(None) => {
                    debug!(
                        claude_session_id = %m.claude_session_id,
                        "No ACTO session for OTel metric — storing unaffiliated"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Failed to look up session for OTel correlation");
                }
            }
        }

        if let Err(e) = state.otel_repo.insert(m).await {
            warn!(error = %e, "Failed to insert OTel metric");
        }
    }

    StatusCode::OK
}

/// POST /v1/logs — accept and discard (future: store as structured events)
pub async fn receive_logs(Json(_body): Json<Value>) -> impl IntoResponse {
    StatusCode::OK
}

pub fn otlp_router(state: OtlpState) -> axum::Router {
    use axum::routing::post;
    axum::Router::new()
        .route("/v1/metrics", post(receive_metrics))
        .route("/v1/logs", post(receive_logs))
        .with_state(state)
}
