use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::readlist::ReadList;

pub struct ReadListRepository {
    pool: PgPool,
}

impl ReadListRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, readlist: &ReadList) -> Result<ReadList, sqlx::Error> {
        let row = sqlx::query(
            r#"INSERT INTO "READLIST" 
            ("ID", "NAME", "BOOK_COUNT", "CREATED_DATE", "LAST_MODIFIED_DATE", "SUMMARY", "ORDERED")
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *"#
        )
        .bind(readlist.id.to_string())
        .bind(&readlist.name)
        .bind(readlist.book_count)
        .bind(readlist.created_date)
        .bind(readlist.last_modified_date)
        .bind(&readlist.summary)
        .bind(readlist.ordered)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_readlist(row))
    }

    pub async fn find_all(&self) -> Result<Vec<ReadList>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "READLIST" ORDER BY "NAME""#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_readlist).collect())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<ReadList>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "READLIST" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_readlist))
    }

    pub async fn update(&self, readlist: &ReadList) -> Result<ReadList, sqlx::Error> {
        let row = sqlx::query(
            r#"UPDATE "READLIST" 
            SET "NAME" = $2, "SUMMARY" = $3, "ORDERED" = $4, "BOOK_COUNT" = $5, "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP
            WHERE "ID" = $1
            RETURNING *"#
        )
        .bind(readlist.id.to_string())
        .bind(&readlist.name)
        .bind(&readlist.summary)
        .bind(readlist.ordered)
        .bind(readlist.book_count)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_readlist(row))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "READLIST" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_book(&self, readlist_id: Uuid, book_id: Uuid, number: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "READLIST_BOOK" ("READLIST_ID", "BOOK_ID", "NUMBER") VALUES ($1, $2, $3)"#
        )
        .bind(readlist_id.to_string())
        .bind(book_id.to_string())
        .bind(number)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_book(&self, readlist_id: Uuid, book_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "READLIST_BOOK" WHERE "READLIST_ID" = $1 AND "BOOK_ID" = $2"#
        )
        .bind(readlist_id.to_string())
        .bind(book_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_books(&self, readlist_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "BOOK_ID" FROM "READLIST_BOOK" WHERE "READLIST_ID" = $1 ORDER BY "NUMBER""#
        )
        .bind(readlist_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.get::<String, _>("BOOK_ID")).collect())
    }
}

fn row_to_readlist(row: sqlx::postgres::PgRow) -> ReadList {
    ReadList {
        id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
        name: row.get::<String, _>("NAME"),
        book_count: row.get::<i32, _>("BOOK_COUNT"),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
        summary: row.get::<String, _>("SUMMARY"),
        ordered: row.get::<bool, _>("ORDERED"),
    }
}