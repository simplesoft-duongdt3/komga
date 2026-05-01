use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::series::{Series, SeriesMetadata};

pub struct SeriesRepository {
    pool: PgPool,
}

impl SeriesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_library(&self, library_id: Uuid) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL ORDER BY "NAME" ASC"#
        )
        .bind(library_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn find_by_library_paginated(&self, library_id: Uuid, limit: usize, offset: usize) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL ORDER BY "NAME" ASC LIMIT $2 OFFSET $3"#
        )
        .bind(library_id.to_string())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64, sqlx::Error> {
        let row = sqlx::query(
            r#"SELECT COUNT(*) as count FROM "SERIES" WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL"#
        )
        .bind(library_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get::<i64, _>("count"))
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
            r#"INSERT INTO "SERIES" ("ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "FILE_LAST_MODIFIED", "NAME", "URL", "LIBRARY_ID", "BOOK_COUNT", "DELETED_DATE", "oneshot")
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

    pub async fn update_metadata(&self, id: &Uuid, metadata: &SeriesMetadata) -> Result<(), sqlx::Error> {
        let metadata_json = serde_json::to_string(metadata).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO "SERIES_METADATA" ("ID", "SERIES_ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "STATUS", "TITLE", "TITLE_LOCK", "PUBLISHER", "PUBLISHER_LOCK", "SUMMARY", "SUMMARY_LOCK", "GENRES", "GENRES_LOCK", "TAGS", "TAGS_LOCK", "LANGUAGE", "LANGUAGE_LOCK", "AGE_RATING", "AGE_RATING_LOCK", "READING_DIRECTION", "READING_DIRECTION_LOCK", "TOTAL_BOOK_COUNT", "TOTAL_BOOK_COUNT_LOCK", "METADATA_JSON")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            ON CONFLICT ("SERIES_ID") DO UPDATE SET "LAST_MODIFIED_DATE" = $4, "STATUS" = $5, "TITLE" = $6, "PUBLISHER" = $8, "SUMMARY" = $10, "GENRES" = $12, "TAGS" = $14, "LANGUAGE" = $16, "AGE_RATING" = $18, "READING_DIRECTION" = $20, "TOTAL_BOOK_COUNT" = $22, "METADATA_JSON" = $24"#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(id.to_string())
        .bind(metadata.created_date)
        .bind(metadata.last_modified_date)
        .bind(&metadata.status)
        .bind(&metadata.title)
        .bind(metadata.title_lock)
        .bind(&metadata.publisher)
        .bind(metadata.publisher_lock)
        .bind(&metadata.summary)
        .bind(metadata.summary_lock)
        .bind(serde_json::to_string(&metadata.genres).unwrap_or("[]".to_string()))
        .bind(metadata.genres_lock)
        .bind(serde_json::to_string(&metadata.tags).unwrap_or("[]".to_string()))
        .bind(metadata.tags_lock)
        .bind(&metadata.language)
        .bind(metadata.language_lock)
        .bind(metadata.age_rating)
        .bind(metadata.age_rating_lock)
        .bind(&metadata.reading_direction)
        .bind(metadata.reading_direction_lock)
        .bind(metadata.total_book_count)
        .bind(metadata.total_book_count_lock)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "DELETED_DATE" IS NULL ORDER BY "NAME" ASC LIMIT $1 OFFSET $2"#
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn find_latest(&self, limit: usize) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "DELETED_DATE" IS NULL ORDER BY "LAST_MODIFIED_DATE" DESC LIMIT $1"#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn find_new(&self, limit: usize) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "DELETED_DATE" IS NULL ORDER BY "CREATED_DATE" DESC LIMIT $1"#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn find_updated(&self, limit: usize) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "DELETED_DATE" IS NULL ORDER BY "LAST_MODIFIED_DATE" DESC LIMIT $1"#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn search_by_name(&self, query: &str, limit: usize) -> Result<Vec<Series>, sqlx::Error> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query(
            r#"SELECT * FROM "SERIES" WHERE "NAME" ILIKE $1 AND "DELETED_DATE" IS NULL ORDER BY "NAME" ASC LIMIT $2"#
        )
        .bind(&pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
    }

    pub async fn find_by_collection(&self, collection_id: Uuid) -> Result<Vec<Series>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT s.* FROM "SERIES" s
            INNER JOIN "COLLECTION_SERIES" cs ON cs."SERIES_ID" = s."ID"
            WHERE cs."COLLECTION_ID" = $1 AND s."DELETED_DATE" IS NULL
            ORDER BY cs."NUMBER""#
        )
        .bind(collection_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_series).collect())
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
        oneshot: row.get::<bool, _>("oneshot"),
        metadata: None,
        cover_file_name: row.get::<Option<String>, _>("COVER_FILE_NAME"),
    }
}