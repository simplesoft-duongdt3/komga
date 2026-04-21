use axum::{
    extract::{Query, State},
    routing::get,
    Router, Json,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::{BookDto, SeriesDto};

#[derive(Deserialize)]
struct SearchParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize { 20 }

async fn search_all(
    State(pool): State<PgPool>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResultsDto>, axum::response::Response> {
    Ok(Json(SearchResultsDto {
        books: vec![],
        series: vec![],
        total: 0,
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResultsDto {
    pub books: Vec<BookDto>,
    pub series: Vec<SeriesDto>,
    pub total: usize,
}

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/api/v1/search", get(search_all))
}