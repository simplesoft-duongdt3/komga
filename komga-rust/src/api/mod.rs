pub mod dto;

use axum::Router;
use sqlx::PgPool;

pub fn routes() -> Router<PgPool> {
    Router::new()
        .merge(auth::routes())
        .merge(library::routes())
        .merge(series::routes())
        .merge(book::routes())
        .merge(search::routes())
}

mod auth;
mod library;
mod series;
mod book;
mod search;