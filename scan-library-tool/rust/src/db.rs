use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::models::{DbBook, DbBookNoThumbnail, DbSeries, DbUnanalyzedBook};

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
}

pub async fn read_series(pool: &PgPool, library_id: &str) -> Result<Vec<DbSeries>, sqlx::Error> {
    sqlx::query_as::<_, DbSeries>(
        r#"SELECT "ID", "NAME", "URL", "FILE_LAST_MODIFIED"
           FROM "SERIES"
           WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL"#,
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
}

pub async fn read_books(pool: &PgPool, library_id: &str) -> Result<Vec<DbBook>, sqlx::Error> {
    sqlx::query_as::<_, DbBook>(
        r#"SELECT "ID", "NAME", "URL", "FILE_LAST_MODIFIED", "FILE_SIZE",
                  "FILE_HASH", "SERIES_ID"
           FROM "BOOK"
           WHERE "LIBRARY_ID" = $1 AND "DELETED_DATE" IS NULL"#,
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
}

pub async fn read_unanalyzed_books(
    pool: &PgPool,
    library_id: &str,
) -> Result<Vec<DbUnanalyzedBook>, sqlx::Error> {
    sqlx::query_as::<_, DbUnanalyzedBook>(
        r#"SELECT b."ID", b."NAME", b."SERIES_ID", m."STATUS"
           FROM "BOOK" b
           LEFT JOIN "MEDIA" m ON b."ID" = m."BOOK_ID"
           WHERE b."LIBRARY_ID" = $1
             AND b."DELETED_DATE" IS NULL
             AND (m."BOOK_ID" IS NULL OR m."STATUS" = 'UNKNOWN')
           ORDER BY b."NAME""#,
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
}

pub async fn read_books_missing_thumbnail(
    pool: &PgPool,
    library_id: &str,
) -> Result<Vec<DbBookNoThumbnail>, sqlx::Error> {
    sqlx::query_as::<_, DbBookNoThumbnail>(
        r#"SELECT b."ID", b."NAME", b."SERIES_ID", m."STATUS"
           FROM "BOOK" b
           INNER JOIN "MEDIA" m ON b."ID" = m."BOOK_ID" AND m."STATUS" = 'READY'
           LEFT JOIN "THUMBNAIL_BOOK" tb ON b."ID" = tb."BOOK_ID" AND tb."SELECTED" = true
           WHERE b."LIBRARY_ID" = $1
             AND b."DELETED_DATE" IS NULL
             AND tb."BOOK_ID" IS NULL
           ORDER BY b."NAME""#,
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
}
