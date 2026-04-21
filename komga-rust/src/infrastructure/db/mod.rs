use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::migrate::Migrator;
use std::env;

pub use sqlx::Row;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/komga".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database");

        Self { pool }
    }

    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        let migrator = Migrator::new(std::path::Path::new("./migrations"))
            .await
            .unwrap();
        
        migrator.run(&self.pool).await?;
        Ok(())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}