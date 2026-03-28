use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/out/"]
struct FrontendAssets;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // 1. Exact file match (assets, favicon, etc.)
    if let Some(file) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return ([(header::CONTENT_TYPE, mime.as_ref())], file.data.to_vec()).into_response();
    }

    // 2. Directory index (e.g. "kanban/" → "kanban/index.html")
    let index_path = if path.is_empty() {
        "index.html".to_string()
    } else {
        format!("{}/index.html", path.trim_end_matches('/'))
    };
    if let Some(file) = FrontendAssets::get(&index_path) {
        let mime = mime_guess::from_path(&index_path).first_or_octet_stream();
        return ([(header::CONTENT_TYPE, mime.as_ref())], file.data.to_vec()).into_response();
    }

    // 3. Task detail fallback: serve the pre-rendered placeholder shell so Next.js
    //    hydrates as the task detail page (not the root task list).
    if path.starts_with("tasks/") {
        if let Some(file) = FrontendAssets::get("tasks/__placeholder__/index.html") {
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            return ([(header::CONTENT_TYPE, mime.as_ref())], file.data.to_vec()).into_response();
        }
    }

    // 4. SPA fallback → root index.html
    match FrontendAssets::get("index.html") {
        Some(file) => {
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data.to_vec()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, Router};
    use tower::ServiceExt;

    fn test_router() -> Router {
        Router::new().fallback(static_handler)
    }

    #[tokio::test]
    async fn unknown_path_returns_200_or_404() {
        // With frontend/out/ built → 200 (SPA fallback index.html)
        // Without frontend/out/ built → 404
        // Either is correct — we verify no panic.
        let app = test_router();
        let req = Request::builder()
            .uri("/tasks/some-unknown-id/")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert!(
            resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
            "Expected 200 or 404, got {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn root_returns_200_or_404() {
        let app = test_router();
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert!(
            resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
            "Expected 200 or 404, got {}",
            resp.status()
        );
    }
}
