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
    }
}