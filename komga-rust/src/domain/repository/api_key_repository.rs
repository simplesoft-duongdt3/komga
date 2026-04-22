use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::user::ApiKey;

pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, api_key: &ApiKey) -> Result<ApiKey, sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "API_KEY" ("ID", "USER_ID", "NAME", "KEY", "CREATED_DATE")
            VALUES ($1, $2, $3, $4, $5)"#
        )
        .bind(&api_key.id)
        .bind(api_key.user_id.to_string())
        .bind(&api_key.name)
        .bind(&api_key.key)
        .bind(api_key.created_date)
        .execute(&self.pool)
        .await?;

        Ok(api_key.clone())
    }

    pub async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "API_KEY" WHERE "KEY" = $1"#
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_api_key))
    }

    pub async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<ApiKey>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "API_KEY" WHERE "USER_ID" = $1"#
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_api_key).collect())
    }

    pub async fn update_last_used(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "API_KEY" SET "LAST_USED_DATE" = $1 WHERE "ID" = $2"#
        )
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(r#"DELETE FROM "API_KEY" WHERE "ID" = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

fn row_to_api_key(row: sqlx::postgres::PgRow) -> ApiKey {
    ApiKey {
        id: row.get::<String, _>("ID"),
        user_id: Uuid::parse_str(&row.get::<String, _>("USER_ID")).unwrap_or_default(),
        name: row.get::<String, _>("NAME"),
        key: row.get::<String, _>("KEY"),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_used_date: row.get::<Option<DateTime<Utc>>, _>("LAST_USED_DATE"),
    }
}