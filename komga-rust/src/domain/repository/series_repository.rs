use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::series::Series;

pub struct SeriesRepository {
    pool: PgPool,
}

impl SeriesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_library(&self, library_id: Uuid) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL ORDER BY "NAME""#
        )
        .bind(library_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Series>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_series))
    }

    pub async fn find_by_name(&self, name: &str, library_id: Uuid) -> Result<Option<Series>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "NAME" = $1 AND "LIBRARY_ID" = $2"#
        )
        .bind(name)
        .bind(library_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_series))
    }

    pub async fn find_by_url(&self, url: &str) -> Result<Option<Series>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "URL" = $1"#
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_series))
    }

    pub async fn insert(&self, series: &Series) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "SERIES" ("ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "FILE_LAST_MODIFIED", "NAME", "URL", "LIBRARY_ID", "BOOK_COUNT", "DELETED_DATE", "ONESHOT")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#
        )
        .bind(series.id.to_string())
        .bind(series.created_date)
        .bind(series.last_modified_date)
        .bind(series.file_last_modified)
        .bind(&series.name)
        .bind(&series.url)
        .bind(series.library_id.to_string())
        .bind(series.book_count)
        .bind(series.deleted_date)
        .bind(series.oneshot)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_book_count(&self, id: &Uuid, count: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "SERIES" SET "BOOK_COUNT" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(count)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn soft_delete(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "SERIES" SET "DELETED_DATE" = $1, "LAST_MODIFIED_DATE" = $1 WHERE "ID" = $2"#
        )
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn hard_delete(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(r#"DELETE FROM "SERIES" WHERE "ID" = $1"#)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_cover(&self, id: &Uuid, cover_path: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "SERIES" SET "COVER_FILE_NAME" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(cover_path)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn row_to_series(row: sqlx::postgres::PgRow) -> Series {
    Series {
        id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
        file_last_modified: row.get::<Option<DateTime<Utc>>, _>("FILE_LAST_MODIFIED"),
        name: row.get::<String, _>("NAME"),
        url: row.get::<String, _>("URL"),
        library_id: Uuid::parse_str(&row.get::<String, _>("LIBRARY_ID")).unwrap_or_default(),
        book_count: row.get::<i32, _>("BOOK_COUNT"),
        deleted_date: row.get::<Option<DateTime<Utc>>, _>("DELETED_DATE"),
        oneshot: row.get::<bool, _>("ONESHOT"),
        metadata: None,
        cover_file_name: row.get::<Option<String>, _>("COVER_FILE_NAME"),
    }
}