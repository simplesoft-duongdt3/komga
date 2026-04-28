use axum::{
    extract::{Query, State},
    routing::get,
    Router, Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::dto::{BookDto, SeriesDto};
use crate::domain::repository::{BookRepository, SeriesRepository};
use crate::infrastructure::search::SearchIndex;

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
) -> Result<Json<SearchResultsDto>, (axum::http::StatusCode, String)> {
    let query_str = params.query.trim().to_string();
    if query_str.is_empty() {
        return Ok(Json(SearchResultsDto {
            books: vec![],
            series: vec![],
            total: 0,
        }));
    }

    let index_path = std::env::current_dir().unwrap_or_default().join("search_index");
    let search_index = match SearchIndex::new(&index_path) {
        Ok(idx) => Some(idx),
        Err(e) => {
            tracing::warn!("Search index unavailable: {}", e);
            None
        }
    };

    if let Some(index) = search_index {
        let ids = match index.search(&query_str, params.limit) {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!("Search error: {}", e);
                vec![]
            }
        };

        if !ids.is_empty() {
            let book_repo = BookRepository::new(pool.clone());
            let series_repo = SeriesRepository::new(pool.clone());
            let mut books = Vec::new();
            let mut series = Vec::new();

            for id in &ids {
                if let Ok(uuid) = Uuid::parse_str(id) {
                    if let Ok(Some(book)) = book_repo.find_by_id(uuid).await {
                        books.push(BookDto::from(book));
                    } else if let Ok(Some(s)) = series_repo.find_by_id(uuid).await {
                        series.push(SeriesDto::from(s));
                    }
                }
            }

            return Ok(Json(SearchResultsDto {
                total: books.len() + series.len(),
                books,
                series,
            }));
        }
    }

    let book_repo = BookRepository::new(pool.clone());
    let series_repo = SeriesRepository::new(pool.clone());

    let books = book_repo.search_by_name(&query_str, params.limit).await
        .unwrap_or_default()
        .into_iter()
        .map(BookDto::from)
        .collect::<Vec<_>>();

    let series = series_repo.search_by_name(&query_str, params.limit).await
        .unwrap_or_default()
        .into_iter()
        .map(SeriesDto::from)
        .collect::<Vec<_>>();

    Ok(Json(SearchResultsDto {
        total: books.len() + series.len(),
        books,
        series,
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