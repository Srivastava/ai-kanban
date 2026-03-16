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
