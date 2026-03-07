use crate::db::SettingsRepository;
use crate::models::UpdateFeatureFlag;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use tracing::{info, warn};

#[derive(Clone)]
pub struct SettingsApiState {
    pub repo: SettingsRepository,
}

pub fn settings_routes() -> Router<SettingsApiState> {
    Router::new()
        .route("/flags", get(get_flags))
        .route("/flags/:key", patch(update_flag))
}

async fn get_flags(State(state): State<SettingsApiState>) -> impl IntoResponse {
    match state.repo.get_all().await {
        Ok(flags) => Json(flags).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn update_flag(
    State(state): State<SettingsApiState>,
    Path(key): Path<String>,
    Json(update): Json<UpdateFeatureFlag>,
) -> impl IntoResponse {
    info!(key = %key, enabled = update.enabled, "Updating feature flag");
    match state.repo.set_flag(&key, update.enabled).await {
        Ok(flag) => {
            info!(key = %flag.key, enabled = flag.enabled, "Feature flag updated");
            Json(flag).into_response()
        }
        Err(e) => {
            warn!(key = %key, error = %e, "Failed to update feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
