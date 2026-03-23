use crate::api::AttachmentApiState;
use crate::models::TaskAttachment;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use tokio::fs;
use uuid::Uuid;

// GET /api/tasks/:task_id/attachments
pub async fn list_attachments(
    State(state): State<AttachmentApiState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<TaskAttachment>>, StatusCode> {
    state
        .repo
        .list_for_task(&task_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// POST /api/tasks/:task_id/attachments  (multipart/form-data, field name: "file")
pub async fn upload_attachment(
    State(state): State<AttachmentApiState>,
    Path(task_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<TaskAttachment>, StatusCode> {
    // Verify task exists
    state
        .task_repo
        .find(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let filename: String = field
            .file_name()
            .unwrap_or("upload")
            .to_string();
        let mime_type: String = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data: Vec<u8> = field
            .bytes()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .to_vec();

        // Write to disk: <attachments_dir>/<task_id>/<uuid>-<filename>
        let dir = format!("{}/{}", state.attachments_dir, task_id);
        fs::create_dir_all(&dir)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let id = Uuid::new_v4().to_string();
        let safe_name = filename.replace(['/', '\\'], "_").replace("..", "_");
        let storage_path = format!("{}/{}-{}", dir, id, safe_name);
        fs::write(&storage_path, &data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let attachment = TaskAttachment {
            id,
            task_id,
            filename,
            storage_path,
            mime_type,
            size_bytes: data.len() as i64,
            created_at: Utc::now(),
        };

        return state
            .repo
            .create(&attachment)
            .await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }
    Err(StatusCode::BAD_REQUEST)
}

// DELETE /api/tasks/:task_id/attachments/:attachment_id
pub async fn delete_attachment(
    State(state): State<AttachmentApiState>,
    Path((task_id, attachment_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let attachment = state
        .repo
        .get(&attachment_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if attachment.task_id != task_id {
        return Err(StatusCode::NOT_FOUND);
    }

    // Delete file from disk (best-effort)
    let _ = fs::remove_file(&attachment.storage_path).await;
    state
        .repo
        .delete(&attachment_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// GET /api/tasks/:task_id/attachments/:attachment_id/file  — serve file
pub async fn serve_attachment(
    State(state): State<AttachmentApiState>,
    Path((task_id, attachment_id)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    let attachment = state
        .repo
        .get(&attachment_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if attachment.task_id != task_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let data = fs::read(&attachment.storage_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, &attachment.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", attachment.filename),
        )
        .body(Body::from(data))
        .unwrap())
}

pub fn attachment_routes() -> axum::Router<AttachmentApiState> {
    use axum::routing::{delete, get, post};
    axum::Router::new()
        .route("/", get(list_attachments).post(upload_attachment))
        .route("/:attachment_id", delete(delete_attachment))
        .route("/:attachment_id/file", get(serve_attachment))
}
