use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::read_progress::ReadProgress;

pub struct ReadProgressRepository {
    pool: PgPool,
}

impl ReadProgressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, progress: &ReadProgress) -> Result<ReadProgress, sqlx::Error> {
        let row = sqlx::query(
            r#"INSERT INTO "READ_PROGRESS" 
            ("BOOK_ID", "USER_ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "PAGE", "COMPLETED", "READ_DATE", "DEVICE_ID", "DEVICE_NAME", "LOCATOR")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT ("BOOK_ID", "USER_ID") DO UPDATE SET
                "PAGE" = EXCLUDED.PAGE,
                "COMPLETED" = EXCLUDED.COMPLETED,
                "READ_DATE" = COALESCE(EXCLUDED.READ_DATE, "READ_PROGRESS"."READ_DATE"),
                "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP,
                "DEVICE_ID" = COALESCE(EXCLUDED.DEVICE_ID, "READ_PROGRESS"."DEVICE_ID"),
                "DEVICE_NAME" = COALESCE(EXCLUDED.DEVICE_NAME, "READ_PROGRESS"."DEVICE_NAME"),
                "LOCATOR" = COALESCE(EXCLUDED.LOCATOR, "READ_PROGRESS"."LOCATOR")
            RETURNING *"#
        )
        .bind(progress.book_id.to_string())
        .bind(progress.user_id.to_string())
        .bind(progress.created_date)
        .bind(progress.last_modified_date)
        .bind(progress.page)
        .bind(progress.completed)
        .bind(progress.read_date)
        .bind(&progress.device_id)
        .bind(&progress.device_name)
        .bind(&progress.locator)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_read_progress(row))
    }

    pub async fn find_by_book_and_user(&self, book_id: Uuid, user_id: Uuid) -> Result<Option<ReadProgress>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "READ_PROGRESS" WHERE "BOOK_ID" = $1 AND "USER_ID" = $2"#
        )
        .bind(book_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_read_progress))
    }

    pub async fn delete(&self, book_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "READ_PROGRESS" WHERE "BOOK_ID" = $1 AND "USER_ID" = $2"#
        )
        .bind(book_id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn row_to_read_progress(row: sqlx::postgres::PgRow) -> ReadProgress {
    ReadProgress {
        book_id: Uuid::parse_str(&row.get::<String, _>("BOOK_ID")).unwrap_or_default(),
        user_id: Uuid::parse_str(&row.get::<String, _>("USER_ID")).unwrap_or_default(),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
        page: row.get::<i32, _>("PAGE"),
        completed: row.get::<bool, _>("COMPLETED"),
        read_date: row.get::<Option<DateTime<Utc>>, _>("READ_DATE"),
        device_id: row.get::<Option<String>, _>("DEVICE_ID"),
        device_name: row.get::<Option<String>, _>("DEVICE_NAME"),
        locator: row.get::<Option<Vec<u8>>, _>("LOCATOR"),
    }
}