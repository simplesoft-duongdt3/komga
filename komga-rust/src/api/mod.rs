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
        .merge(readlist::routes())
        .merge(collection::routes())
        .merge(task::routes())
        .merge(apikey::routes())
}

pub mod auth;
pub mod library;
pub mod series;
pub mod book;
pub mod search;
pub mod readlist;
pub mod collection;
pub mod task;
pub mod apikey;