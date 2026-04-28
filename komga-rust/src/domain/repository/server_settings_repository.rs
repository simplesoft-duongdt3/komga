use sqlx::PgPool;
use sqlx::Row;

pub struct ServerSettingsRepository {
    pool: PgPool,
}

impl ServerSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<Vec<(String, String)>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "KEY", "VALUE" FROM "SERVER_SETTINGS""#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| {
            (r.get::<String, _>("KEY"), r.get::<Option<String>, _>("VALUE").unwrap_or_default())
        }).collect())
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT "VALUE" FROM "SERVER_SETTINGS" WHERE "KEY" = $1"#
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.get::<Option<String>, _>("VALUE").unwrap_or_default()))
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "SERVER_SETTINGS" ("KEY", "VALUE") VALUES ($1, $2)
            ON CONFLICT ("KEY") DO UPDATE SET "VALUE" = $2"#
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "SERVER_SETTINGS" WHERE "KEY" = $1"#
        )
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
