use reqwest::Client;

use crate::config::Config;
use crate::models::*;

#[derive(Clone)]
pub struct KomgaClient {
    client: Client,
    base_url: String,
    user: String,
    password: String,
    max_retries: u32,
}

impl KomgaClient {
    pub fn new(config: &Config) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: config.komga_url.trim_end_matches('/').to_string(),
            user: config.komga_user.clone(),
            password: config.komga_password.clone(),
            max_retries: config.max_api_retries,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.url(path);
        let mut last_err = None;
        for attempt in 1..=self.max_retries {
            match self
                .client
                .post(&url)
                .basic_auth(&self.user, Some(&self.password))
                .json(body)
                .send()
                .await
            {
                Ok(resp) => return Ok(resp.error_for_status()?),
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "POST retry");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap())
    }

    async fn delete(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.url(path);
        let mut last_err = None;
        for attempt in 1..=self.max_retries {
            match self
                .client
                .delete(&url)
                .basic_auth(&self.user, Some(&self.password))
                .send()
                .await
            {
                Ok(resp) => return Ok(resp.error_for_status()?),
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "DELETE retry");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap())
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.url(path);
        self.client
            .get(&url)
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await
            .map(|r| r.error_for_status())?
    }

    pub async fn list_libraries(&self) -> Result<Vec<Library>, reqwest::Error> {
        let resp = self.get("/api/v1/libraries").await?;
        let data: serde_json::Value = resp.json().await?;
        let libs: Vec<Library> = if data.is_object() {
            if let Some(content) = data.get("content") {
                serde_json::from_value(content.clone()).unwrap_or_default()
            } else {
                serde_json::from_value(data).unwrap_or_default()
            }
        } else {
            serde_json::from_value(data).unwrap_or_default()
        };
        Ok(libs)
    }

    pub async fn create_series(
        &self,
        request: &CreateSeriesRequest,
    ) -> Result<SeriesDto, reqwest::Error> {
        let resp = self.post("/api/v1/series", request).await?;
        resp.json().await
    }

    pub async fn delete_series(&self, series_id: &str) -> Result<(), reqwest::Error> {
        self.delete(&format!("/api/v1/series/{}/file", series_id))
            .await?;
        Ok(())
    }

    pub async fn get_series_books(
        &self,
        series_id: &str,
    ) -> Result<Vec<BookResponseDto>, reqwest::Error> {
        let resp = self
            .get(&format!("/api/v1/series/{}/books", series_id))
            .await?;
        let page: BooksPage = resp.json().await?;
        Ok(page.content)
    }

    pub async fn empty_trash(&self, library_id: &str) -> Result<(), reqwest::Error> {
        self.post(
            &format!("/api/v1/libraries/{}/empty-trash", library_id),
            &serde_json::Value::Null,
        )
        .await?;
        Ok(())
    }

    pub async fn analyze_book(&self, book_id: &str) -> Result<(), reqwest::Error> {
        self.post(
            &format!("/api/v1/books/{}/analyze", book_id),
            &serde_json::Value::Null,
        )
        .await?;
        Ok(())
    }

    pub async fn refresh_book_metadata(&self, book_id: &str) -> Result<(), reqwest::Error> {
        self.post(
            &format!("/api/v1/books/{}/metadata/refresh", book_id),
            &serde_json::Value::Null,
        )
        .await?;
        Ok(())
    }

    pub async fn refresh_series_metadata(&self, series_id: &str) -> Result<(), reqwest::Error> {
        self.post(
            &format!("/api/v1/series/{}/metadata/refresh", series_id),
            &serde_json::Value::Null,
        )
        .await?;
        Ok(())
    }
}
