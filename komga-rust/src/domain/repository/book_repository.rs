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

    pub async fn find_by_library(&self, library_id: Uuid) -> Result<Vec<Book>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL ORDER BY "NAME""#
        )
        .bind(library_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_book).collect())
    }

    pub async fn find_by_name(&self, name: &str, series_id: Uuid) -> Result<Option<Book>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "NAME" = $1 AND "SERIES_ID" = $2"#
        )
        .bind(name)
        .bind(series_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_book))
    }

    pub async fn find_deleted(&self, library_id: Uuid) -> Result<Vec<Book>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NOT NULL"#
        )
        .bind(library_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_book).collect())
    }

    pub async fn insert(&self, book: &Book) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "BOOK" ("ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "FILE_LAST_MODIFIED", "NAME", "URL", "SERIES_ID", "FILE_SIZE", "NUMBER", "LIBRARY_ID", "FILE_HASH", "DELETED_DATE", "ONESHOT", "FILE_HASH_KOREADER")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"#
        )
        .bind(book.id.to_string())
        .bind(book.created_date)
        .bind(book.last_modified_date)
        .bind(book.file_last_modified)
        .bind(&book.name)
        .bind(&book.url)
        .bind(book.series_id.to_string())
        .bind(book.file_size)
        .bind(book.number)
        .bind(book.library_id.to_string())
        .bind(&book.file_hash)
        .bind(book.deleted_date)
        .bind(book.oneshot)
        .bind(&book.file_hash_koreader)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_file_hash(&self, id: &Uuid, hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "FILE_HASH" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(hash)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_file_hash_koreader(&self, id: &Uuid, hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "FILE_HASH_KOREADER" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(hash)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_file_size(&self, id: &Uuid, size: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "FILE_SIZE" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(size)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_file_last_modified(&self, id: &Uuid, modified: DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "FILE_LAST_MODIFIED" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(modified)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn soft_delete(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "DELETED_DATE" = $1, "LAST_MODIFIED_DATE" = $1 WHERE "ID" = $2"#
        )
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn restore(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "DELETED_DATE" = NULL, "LAST_MODIFIED_DATE" = $1 WHERE "ID" = $2"#
        )
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn hard_delete(&self, id: &Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(r#"DELETE FROM "BOOK" WHERE "ID" = $1"#)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_cover(&self, id: &Uuid, cover_path: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "BOOK" SET "COVER_FILE_NAME" = $1, "LAST_MODIFIED_DATE" = $2 WHERE "ID" = $3"#
        )
        .bind(cover_path)
        .bind(Utc::now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
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
        cover_file_name: row.get::<Option<String>, _>("COVER_FILE_NAME"),
    }
}