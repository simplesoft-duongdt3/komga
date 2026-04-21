use axum::{
    extract::{Path, Query, State},
    routing::get,
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::{SeriesDto, SeriesPageDto};
use crate::domain::repository::SeriesRepository;

#[derive(Deserialize)]
struct PageParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_size")]
    size: usize,
}

fn default_page() -> usize { 0 }
fn default_size() -> usize { 20 }

async fn get_series_by_library(
    State(pool): State<PgPool>,
    Path(library_id): Path<String>,
    Query(params): Query<PageParams>,
) -> Result<Json<SeriesPageDto>, axum::response::Response> {
    let library_uuid = Uuid::parse_str(&library_id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    match repo.find_by_library(library_uuid).await {
        Ok(series_list) => {
            let total = series_list.len();
            let series: Vec<SeriesDto> = series_list.into_iter().map(|s| s.into()).collect();
            Ok(Json(SeriesPageDto {
                content: series,
                total_elements: total,
                total_pages: 1,
                number: params.page,
                size: params.size,
            }))
        }
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

async fn get_series(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<SeriesDto>, axum::response::Response> {
    let uuid = Uuid::parse_str(&id).unwrap_or_default();
    let repo = SeriesRepository::new(pool);
    
    match repo.find_by_id(uuid).await {
        Ok(Some(series)) => Ok(Json(series.into())),
        Ok(None) => Err((axum::http::StatusCode::NOT_FOUND, "Series not found").into_response()),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/libraries/{libraryId}/series", get(get_series_by_library))
        .route("/api/v1/series/{id}", get(get_series))
}