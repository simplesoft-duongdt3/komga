use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::collection::Collection;

pub struct CollectionRepository {
    pool: PgPool,
}

impl CollectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, collection: &Collection) -> Result<Collection, sqlx::Error> {
        let row = sqlx::query(
            r#"INSERT INTO "COLLECTION" 
            ("ID", "NAME", "ORDERED", "SERIES_COUNT", "CREATED_DATE", "LAST_MODIFIED_DATE")
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *"#
        )
        .bind(collection.id.to_string())
        .bind(&collection.name)
        .bind(collection.ordered)
        .bind(collection.series_count)
        .bind(collection.created_date)
        .bind(collection.last_modified_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_collection(row))
    }

    pub async fn find_all(&self) -> Result<Vec<Collection>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "COLLECTION" ORDER BY "NAME""#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_collection).collect())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Collection>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "COLLECTION" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_collection))
    }

    pub async fn update(&self, collection: &Collection) -> Result<Collection, sqlx::Error> {
        let row = sqlx::query(
            r#"UPDATE "COLLECTION" 
            SET "NAME" = $2, "ORDERED" = $3, "SERIES_COUNT" = $4, "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP
            WHERE "ID" = $1
            RETURNING *"#
        )
        .bind(collection.id.to_string())
        .bind(&collection.name)
        .bind(collection.ordered)
        .bind(collection.series_count)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_collection(row))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "COLLECTION" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_series(&self, collection_id: Uuid, series_id: Uuid, number: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO "COLLECTION_SERIES" ("COLLECTION_ID", "SERIES_ID", "NUMBER") VALUES ($1, $2, $3)"#
        )
        .bind(collection_id.to_string())
        .bind(series_id.to_string())
        .bind(number)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_series(&self, collection_id: Uuid, series_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "COLLECTION_SERIES" WHERE "COLLECTION_ID" = $1 AND "SERIES_ID" = $2"#
        )
        .bind(collection_id.to_string())
        .bind(series_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_series(&self, collection_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "SERIES_ID" FROM "COLLECTION_SERIES" WHERE "COLLECTION_ID" = $1 ORDER BY "NUMBER""#
        )
        .bind(collection_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.get::<String, _>("SERIES_ID")).collect())
    }
}

fn row_to_collection(row: sqlx::postgres::PgRow) -> Collection {
    Collection {
        id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
        name: row.get::<String, _>("NAME"),
        ordered: row.get::<bool, _>("ORDERED"),
        series_count: row.get::<i32, _>("SERIES_COUNT"),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
    }
}