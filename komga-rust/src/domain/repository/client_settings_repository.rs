use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

pub struct ClientSettingsRepository {
    pool: PgPool,
}

impl ClientSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_global_all(&self) -> Result<Vec<(String, String, bool)>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "KEY", "VALUE", "ALLOW_UNAUTHORIZED" FROM "CLIENT_SETTINGS_GLOBAL""#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| {
            (
                r.get::<String, _>("KEY"),
                r.get::<String, _>("VALUE"),
                r.get::<bool, _>("ALLOW_UNAUTHORIZED"),
            )
        }).collect())
    }

    pub async fn set_global(&self, key: &str, value: &str, allow_unauthorized: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "CLIENT_SETTINGS_GLOBAL" ("KEY", "VALUE", "ALLOW_UNAUTHORIZED") VALUES ($1, $2, $3)
            ON CONFLICT ("KEY") DO UPDATE SET "VALUE" = $2, "ALLOW_UNAUTHORIZED" = $3"#
        )
        .bind(key)
        .bind(value)
        .bind(allow_unauthorized)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_global(&self, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "CLIENT_SETTINGS_GLOBAL" WHERE "KEY" = $1"#
        )
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_all(&self, user_id: Uuid) -> Result<Vec<(String, String)>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "KEY", "VALUE" FROM "CLIENT_SETTINGS_USER" WHERE "USER_ID" = $1"#
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| {
            (r.get::<String, _>("KEY"), r.get::<String, _>("VALUE"))
        }).collect())
    }

    pub async fn set_user(&self, user_id: Uuid, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "CLIENT_SETTINGS_USER" ("USER_ID", "KEY", "VALUE") VALUES ($1, $2, $3)
            ON CONFLICT ("KEY", "USER_ID") DO UPDATE SET "VALUE" = $3"#
        )
        .bind(user_id.to_string())
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_user_setting(&self, user_id: Uuid, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "CLIENT_SETTINGS_USER" WHERE "USER_ID" = $1 AND "KEY" = $2"#
        )
        .bind(user_id.to_string())
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
