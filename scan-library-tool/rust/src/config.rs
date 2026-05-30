use std::env;

#[derive(Clone)]
pub struct Config {
    pub komga_url: String,
    pub komga_user: String,
    pub komga_password: String,
    pub pg_host: String,
    pub pg_port: String,
    pub pg_db: String,
    pub pg_user: String,
    pub pg_password: String,
    pub dry_run: bool,
    pub max_api_retries: u32,
    pub export_dir: String,
    pub scan_threads: usize,
    pub hash_cache_dir: String,
    pub hasher_threads: usize,
    pub port: u16,
    pub version: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            komga_url: env::var("KOMGA_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into()),
            komga_user: env::var("KOMGA_USER").unwrap_or_else(|_| "admin@example.com".into()),
            komga_password: env::var("KOMGA_PASSWORD").unwrap_or_default(),
            pg_host: env::var("PG_HOST").unwrap_or_else(|_| "localhost".into()),
            pg_port: env::var("PG_PORT").unwrap_or_else(|_| "5432".into()),
            pg_db: env::var("PG_DB").unwrap_or_else(|_| "komga".into()),
            pg_user: env::var("PG_USER").unwrap_or_else(|_| "komga".into()),
            pg_password: env::var("PG_PASSWORD").unwrap_or_default(),
            dry_run: env::var("DRY_RUN").unwrap_or_default().to_lowercase() == "true",
            max_api_retries: env::var("MAX_API_RETRIES")
                .unwrap_or_else(|_| "3".into())
                .parse()
                .unwrap_or(3),
            export_dir: env::var("EXPORT_DIR").unwrap_or_else(|_| "/exports".into()),
            scan_threads: env::var("SCAN_THREADS")
                .unwrap_or_else(|_| "0".into())
                .parse()
                .unwrap_or(0),
            hash_cache_dir: env::var("HASH_CACHE_DIR").unwrap_or_else(|_| "/exports".into()),
            hasher_threads: env::var("HASHER_THREADS")
                .unwrap_or_else(|_| "4".into())
                .parse()
                .unwrap_or(4),
            port: env::var("PORT")
                .unwrap_or_else(|_| "5050".into())
                .parse()
                .unwrap_or(5050),
            version: include_str!("../VERSION").trim().to_string(),
        }
    }

    pub fn pg_conn_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.pg_user, self.pg_password, self.pg_host, self.pg_port, self.pg_db
        )
    }
}
