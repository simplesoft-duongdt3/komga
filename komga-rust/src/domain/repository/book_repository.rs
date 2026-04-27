use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::book::{Book, BookMetadata};

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

    pub async fn update_metadata(&self, id: &Uuid, metadata: &BookMetadata) -> Result<(), sqlx::Error> {
        let metadata_json = serde_json::to_string(metadata).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO "BOOK_METADATA" ("ID", "BOOK_ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "NUMBER", "NUMBER_LOCK", "NUMBER_SORT", "NUMBER_SORT_LOCK", "RELEASE_DATE", "RELEASE_DATE_LOCK", "SUMMARY", "SUMMARY_LOCK", "TITLE", "TITLE_LOCK", "AUTHORS", "AUTHORS_LOCK", "TAGS", "TAGS_LOCK", "ISBN", "ISBN_LOCK", "LINKS", "LINKS_LOCK", "METADATA_JSON")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
            ON CONFLICT ("BOOK_ID") DO UPDATE SET "LAST_MODIFIED_DATE" = $4, "NUMBER" = $5, "NUMBER_SORT" = $7, "SUMMARY" = $11, "TITLE" = $13, "AUTHORS" = $15, "TAGS" = $17, "ISBN" = $19, "LINKS" = $21, "METADATA_JSON" = $23"#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(id.to_string())
        .bind(metadata.created_date)
        .bind(metadata.last_modified_date)
        .bind(&metadata.number)
        .bind(metadata.number_lock)
        .bind(metadata.number_sort)
        .bind(metadata.number_sort_lock)
        .bind(metadata.release_date)
        .bind(metadata.release_date_lock)
        .bind(&metadata.summary)
        .bind(metadata.summary_lock)
        .bind(&metadata.title)
        .bind(metadata.title_lock)
        .bind(serde_json::to_string(&metadata.authors).unwrap_or("[]".to_string()))
        .bind(metadata.authors_lock)
        .bind(serde_json::to_string(&metadata.tags).unwrap_or("[]".to_string()))
        .bind(metadata.tags_lock)
        .bind(&metadata.isbn)
        .bind(metadata.isbn_lock)
        .bind(serde_json::to_string(&metadata.links).unwrap_or("[]".to_string()))
        .bind(metadata.links_lock)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Book>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "DELETED_DATE" IS NULL ORDER BY "LAST_MODIFIED_DATE" DESC LIMIT $1 OFFSET $2"#
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_book).collect())
    }

    pub async fn find_latest(&self, limit: usize) -> Result<Vec<Book>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "DELETED_DATE" IS NULL ORDER BY "LAST_MODIFIED_DATE" DESC LIMIT $1"#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_book).collect())
    }

    pub async fn find_previous_in_series(&self, series_id: &Uuid, current_number: i32) -> Result<Option<Book>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "SERIES_ID" = $1 AND "NUMBER" < $2 AND "DELETED_DATE" IS NULL ORDER BY "NUMBER" DESC LIMIT 1"#
        )
        .bind(series_id.to_string())
        .bind(current_number)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_book))
    }

    pub async fn find_next_in_series(&self, series_id: &Uuid, current_number: i32) -> Result<Option<Book>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "BOOK" WHERE "SERIES_ID" = $1 AND "NUMBER" > $2 AND "DELETED_DATE" IS NULL ORDER BY "NUMBER" ASC LIMIT 1"#
        )
        .bind(series_id.to_string())
        .bind(current_number)
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
        cover_file_name: row.get::<Option<String>, _>("COVER_FILE_NAME"),
    }
}