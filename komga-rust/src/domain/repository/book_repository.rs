use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::book::Book;

pub struct BookRepository {
    pool: PgPool,
}

impl BookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_series(&self, series_id: Uuid) -> Result<Vec<Book>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "SERIES_ID" = $1 AND "DELETED_DATE" IS NULL ORDER BY "NUMBER""#
        )
        .bind(series_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_book).collect())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Book>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_book))
    }
}

fn row_to_book(row: sqlx::postgres::PgRow) -> Book {
    Book {
        id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
        file_last_modified: row.get::<Option<DateTime<Utc>>, _>("FILE_LAST_MODIFIED"),
        name: row.get::<String, _>("NAME"),
        url: row.get::<String, _>("URL"),
        series_id: Uuid::parse_str(&row.get::<String, _>("SERIES_ID")).unwrap_or_default(),
        file_size: row.get::<i64, _>("FILE_SIZE"),
        number: row.get::<i32, _>("NUMBER"),
        library_id: Uuid::parse_str(&row.get::<String, _>("LIBRARY_ID")).unwrap_or_default(),
        file_hash: row.get::<String, _>("FILE_HASH"),
        deleted_date: row.get::<Option<DateTime<Utc>>, _>("DELETED_DATE"),
        oneshot: row.get::<bool, _>("ONESHOT"),
        file_hash_koreader: row.get::<String, _>("FILE_HASH_KOREADER"),
        metadata: None,
        media: None,
    }
}