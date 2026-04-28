use sqlx::postgres::PgRow;
use sqlx::PgPool;
use sqlx::Row;
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::user::{User, UserRole};

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, email: &str, password_hash: &str) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query(
            r#"INSERT INTO "USER" ("ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "EMAIL", "PASSWORD", "SHARED_ALL_LIBRARIES")
            VALUES ($1, $2, $3, $4, $5, true)
            RETURNING "ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "EMAIL", "PASSWORD", "SHARED_ALL_LIBRARIES", "AGE_RESTRICTION", "AGE_RESTRICTION_ALLOW_ONLY""#
        )
        .bind(id.to_string())
        .bind(now)
        .bind(now)
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
            created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
            last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
            email: row.get::<String, _>("EMAIL"),
            password: row.get::<String, _>("PASSWORD"),
            shared_all_libraries: row.get::<bool, _>("SHARED_ALL_LIBRARIES"),
            age_restriction: row.get::<Option<i32>, _>("AGE_RESTRICTION"),
            age_restriction_allow_only: row.get::<Option<bool>, _>("AGE_RESTRICTION_ALLOW_ONLY"),
            roles: vec![UserRole::PageViewer],
        })
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT "ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "EMAIL", "PASSWORD", "SHARED_ALL_LIBRARIES", "AGE_RESTRICTION", "AGE_RESTRICTION_ALLOW_ONLY" FROM "USER" WHERE "EMAIL" = $1"#
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| User {
            id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
            created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
            last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
            email: row.get::<String, _>("EMAIL"),
            password: row.get::<String, _>("PASSWORD"),
            shared_all_libraries: row.get::<bool, _>("SHARED_ALL_LIBRARIES"),
            age_restriction: row.get::<Option<i32>, _>("AGE_RESTRICTION"),
            age_restriction_allow_only: row.get::<Option<bool>, _>("AGE_RESTRICTION_ALLOW_ONLY"),
            roles: vec![UserRole::PageViewer],
        }))
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT "ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "EMAIL", "PASSWORD", "SHARED_ALL_LIBRARIES", "AGE_RESTRICTION", "AGE_RESTRICTION_ALLOW_ONLY" FROM "USER" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| User {
            id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
            created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
            last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
            email: row.get::<String, _>("EMAIL"),
            password: row.get::<String, _>("PASSWORD"),
            shared_all_libraries: row.get::<bool, _>("SHARED_ALL_LIBRARIES"),
            age_restriction: row.get::<Option<i32>, _>("AGE_RESTRICTION"),
            age_restriction_allow_only: row.get::<Option<bool>, _>("AGE_RESTRICTION_ALLOW_ONLY"),
            roles: vec![UserRole::PageViewer],
        }))
    }

    pub async fn find_all(&self) -> Result<Vec<User>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "EMAIL", "PASSWORD", "SHARED_ALL_LIBRARIES", "AGE_RESTRICTION", "AGE_RESTRICTION_ALLOW_ONLY" FROM "USER" ORDER BY "EMAIL""#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| User {
            id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
            created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
            last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
            email: row.get::<String, _>("EMAIL"),
            password: row.get::<String, _>("PASSWORD"),
            shared_all_libraries: row.get::<bool, _>("SHARED_ALL_LIBRARIES"),
            age_restriction: row.get::<Option<i32>, _>("AGE_RESTRICTION"),
            age_restriction_allow_only: row.get::<Option<bool>, _>("AGE_RESTRICTION_ALLOW_ONLY"),
            roles: vec![UserRole::PageViewer],
        }).collect())
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "USER" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update(&self, id: Uuid, email: Option<String>, shared_all_libraries: Option<bool>, age_restriction: Option<i32>) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "USER" SET 
            "EMAIL" = COALESCE($2, "EMAIL"), 
            "SHARED_ALL_LIBRARIES" = COALESCE($3, "SHARED_ALL_LIBRARIES"),
            "AGE_RESTRICTION" = COALESCE($4, "AGE_RESTRICTION"),
            "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP
            WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .bind(email)
        .bind(shared_all_libraries)
        .bind(age_restriction)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "USER" SET "PASSWORD" = $2, "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .bind(password_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}