pub mod api;
pub mod application;
pub mod domain;
pub mod infrastructure;

use std::net::SocketAddr;
use std::str::FromStr;

use axum::Router;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::infrastructure::db::Database;

pub async fn run() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database = Database::new().await;
    database.run_migrations().await.unwrap();

    let pool = database.pool();

    let mut worker = crate::application::task_worker::TaskWorker::new(pool.clone());
    worker.start().await;

    let app = Router::new()
        .merge(api::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let addr = SocketAddr::from_str("0.0.0.0:8080").unwrap();
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}