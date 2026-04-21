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
}

mod auth;
mod library;
mod series;
mod book;
mod search;
mod readlist;
mod collection;
mod task;