use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::api::dto::{TaskDto, TaskPageDto};

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
    State(_pool): State<PgPool>,
    Query(_params): Query<TaskParams>,
) -> Result<Json<TaskPageDto>, axum::response::Response> {
    Ok(Json(TaskPageDto {
        content: vec![],
        total_elements: 0,
        total_pages: 0,
        number: 0,
        size: 0,
    }))
}

async fn get_task(
    State(_pool): State<PgPool>,
    Path(_id): Path<String>,
) -> Result<Json<TaskDto>, axum::response::Response> {
    Err((axum::http::StatusCode::NOT_FOUND, "Task not implemented yet").into_response())
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/tasks", get(get_tasks))
        .route("/api/v1/tasks/{id}", get(get_task))
}