use sqlx::PgPool;
use sqlx::Row;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct PageHash {
    pub hash: String,
    pub size: Option<i64>,
    pub action: String,
    pub delete_count: i32,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
}

pub struct PageHashRepository {
    pool: PgPool,
}

impl PageHashRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_all(&self, limit: i64, offset: i64) -> Result<(Vec<PageHash>, i64), sqlx::Error> {
        let total: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM "PAGE_HASH""#
        )
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            r#"SELECT * FROM "PAGE_HASH" ORDER BY "LAST_MODIFIED_DATE" DESC LIMIT $1 OFFSET $2"#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let hashes = rows.into_iter().map(|r| PageHash {
            hash: r.get("HASH"),
            size: r.get("SIZE"),
            action: r.get("ACTION"),
            delete_count: r.get("DELETE_COUNT"),
            created_date: r.get("CREATED_DATE"),
            last_modified_date: r.get("LAST_MODIFIED_DATE"),
        }).collect();

        Ok((hashes, total.0))
    }

    pub async fn find_by_hash(&self, hash: &str) -> Result<Option<PageHash>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "PAGE_HASH" WHERE "HASH" = $1"#
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| PageHash {
            hash: r.get("HASH"),
            size: r.get("SIZE"),
            action: r.get("ACTION"),
            delete_count: r.get("DELETE_COUNT"),
            created_date: r.get("CREATED_DATE"),
            last_modified_date: r.get("LAST_MODIFIED_DATE"),
        }))
    }

    pub async fn delete_matching(&self, hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "PAGE_HASH" WHERE "HASH" = $1"#
        )
        .bind(hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
