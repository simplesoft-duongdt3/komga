use axum::{
    extract::{Path, Query, State},
    routing::{get, delete},
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::{TaskDto, TaskPageDto};
use crate::domain::model::task::TaskStatus;
use crate::domain::repository::TaskRepository;

#[derive(Deserialize)]
struct TaskParams {
    #[serde(default = "default_status")]
    status: Option<String>,
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_size")]
    size: usize,
}

fn default_page() -> usize { 0 }
fn default_size() -> usize { 20 }
fn default_status() -> Option<String> { None }

async fn get_tasks(
    State(pool): State<PgPool>,
    Query(params): Query<TaskParams>,
) -> Result<Json<TaskPageDto>, axum::response::Response> {
    eprintln!("[DEBUG] get_tasks called");
    let repo = TaskRepository::new(pool);
    let status = params.status.as_deref();
    let size = params.size.max(1);
    
    match repo.find_all(status, size).await {
        Ok(tasks) => {
            let total = tasks.len();
            let task_dtos: Vec<TaskDto> = tasks.into_iter().map(|t| t.into()).collect();
            eprintln!("[DEBUG] Found {} tasks", total);
            Ok(Json(TaskPageDto::new(task_dtos, total, params.page, params.size,)))
        }
        Err(e) => {
            eprintln!("[DEBUG] Task find_all error: {}", e);
            Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
        },
    }
}

async fn get_task(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<TaskDto>, axum::response::Response> {
    let repo = TaskRepository::new(pool);
    
    match repo.find_by_id(&id).await {
        Ok(Some(task)) => Ok(Json(task.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Task not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn delete_task(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let repo = TaskRepository::new(pool);
    
    match repo.find_by_id(&id).await {
        Ok(Some(_task)) => {
            if let Err(e) = repo.update_status(&id, &TaskStatus::Cancelled).await {
                return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response());
            }
            Ok((axum::http::StatusCode::NO_CONTENT, "").into_response())
        }
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Task not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/tasks", get(get_tasks))
        .route("/api/v1/tasks/:id", get(get_task))
        .route("/api/v1/tasks/:id", delete(delete_task))
}