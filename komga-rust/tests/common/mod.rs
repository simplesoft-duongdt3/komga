use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;

pub struct TestContext {
    client: reqwest::Client,
    base_url: String,
    pub pool: PgPool,
    _postgres: Option<testcontainers::ContainerAsync<Postgres>>,
    _redis: Option<testcontainers::ContainerAsync<Redis>>,
}

impl TestContext {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(&self.url(path)).send().await.unwrap()
    }

    pub async fn auth_get(&self, token: &str, path: &str) -> reqwest::Response {
        self.client
            .get(&self.url(path))
            .header("Authorization", format!("Bearer {}", token))
            .send().await.unwrap()
    }

    pub async fn post(&self, path: &str) -> reqwest::Response {
        self.client.post(&self.url(path)).send().await.unwrap()
    }

    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client.post(&self.url(path)).json(body).send().await.unwrap()
    }

    pub async fn auth_post_json(&self, token: &str, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client
            .post(&self.url(path))
            .json(body)
            .header("Authorization", format!("Bearer {}", token))
            .send().await.unwrap()
    }

    pub async fn auth_post(&self, token: &str, path: &str) -> reqwest::Response {
        self.client
            .post(&self.url(path))
            .header("Authorization", format!("Bearer {}", token))
            .send().await.unwrap()
    }

    pub async fn put_json(&self, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client.put(&self.url(path)).json(body).send().await.unwrap()
    }

    pub async fn patch_json(&self, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client.patch(&self.url(path)).json(body).send().await.unwrap()
    }

    pub async fn auth_patch_json(&self, token: &str, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client
            .patch(&self.url(path))
            .json(body)
            .header("Authorization", format!("Bearer {}", token))
            .send().await.unwrap()
    }

    pub async fn auth_put_json(&self, token: &str, path: &str, body: &serde_json::Value) -> reqwest::Response {
        self.client
            .put(&self.url(path))
            .json(body)
            .header("Authorization", format!("Bearer {}", token))
            .send().await.unwrap()
    }

    pub async fn auth_delete(&self, token: &str, path: &str) -> reqwest::Response {
        self.client
            .delete(&self.url(path))
            .header("Authorization", format!("Bearer {}", token))
            .send().await.unwrap()
    }

    pub async fn delete(&self, path: &str) -> reqwest::Response {
        self.client.delete(&self.url(path)).send().await.unwrap()
    }
}

pub async fn setup_test_context() -> TestContext {
    let pg = Postgres::default().start().await.expect("Failed to start PostgreSQL");
    let host = pg.get_host().await.expect("Failed to get host");
    let port = pg.get_host_port_ipv4(5432).await.expect("Failed to get port");
    let db_url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to test PostgreSQL");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let app = komga_rust::create_app(pool.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind test server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = reqwest::Client::builder().build().unwrap();

    TestContext {
        client,
        base_url,
        pool,
        _postgres: Some(pg),
        _redis: None,
    }
}

pub async fn register_and_login(ctx: &TestContext) -> String {
    let resp = ctx
        .post_json(
            "/api/v1/claim",
            &serde_json::json!({"email": "test@test.com", "password": "test123"}),
        )
        .await;
    let body: serde_json::Value = resp.json().await.unwrap();
    body["token"].as_str().unwrap().to_string()
}
