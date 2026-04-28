use sqlx::PgPool;
use sqlx::Row;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Thumbnail {
    pub id: String,
    pub url: Option<String>,
    pub selected: bool,
    pub data: Option<Vec<u8>>,
    pub thumbnail_type: String,
    pub created_date: DateTime<Utc>,
    pub last_modified_date: DateTime<Utc>,
    pub entity_id: String,
    pub width: i32,
    pub height: i32,
    pub media_type: String,
    pub file_size: i64,
}

pub struct ThumbnailRepository {
    pool: PgPool,
}

impl ThumbnailRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn find_by_table(&self, table: &str, entity_id: Uuid) -> Result<Vec<Thumbnail>, sqlx::Error> {
        let query = format!(
            r#"SELECT * FROM "{}" WHERE "{}_ID" = $1 ORDER BY "CREATED_DATE" DESC"#,
            table, table.replace("THUMBNAIL_", "")
        );
        let rows = sqlx::query(&query)
            .bind(entity_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| Thumbnail {
            id: r.get("ID"),
            url: r.get("URL"),
            selected: r.get("SELECTED"),
            data: r.get("THUMBNAIL"),
            thumbnail_type: r.get("TYPE"),
            created_date: r.get("CREATED_DATE"),
            last_modified_date: r.get("LAST_MODIFIED_DATE"),
            entity_id: entity_id.to_string(),
            width: r.get("WIDTH"),
            height: r.get("HEIGHT"),
            media_type: r.get("MEDIA_TYPE"),
            file_size: r.get("FILE_SIZE"),
        }).collect())
    }

    async fn find_by_id_in_table(&self, table: &str, thumbnail_id: Uuid) -> Result<Option<Thumbnail>, sqlx::Error> {
        let query = format!(r#"SELECT * FROM "{}" WHERE "ID" = $1"#, table);
        let result = sqlx::query(&query)
            .bind(thumbnail_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.map(|r| Thumbnail {
            id: r.get("ID"),
            url: r.get("URL"),
            selected: r.get("SELECTED"),
            data: r.get("THUMBNAIL"),
            thumbnail_type: r.get("TYPE"),
            created_date: r.get("CREATED_DATE"),
            last_modified_date: r.get("LAST_MODIFIED_DATE"),
            entity_id: String::new(),
            width: r.get("WIDTH"),
            height: r.get("HEIGHT"),
            media_type: r.get("MEDIA_TYPE"),
            file_size: r.get("FILE_SIZE"),
        }))
    }

    async fn mark_selected_in_table(&self, table: &str, entity_id: Uuid, thumbnail_id: Uuid) -> Result<(), sqlx::Error> {
        let col = table.replace("THUMBNAIL_", "");
        let query_unselect = format!(
            r#"UPDATE "{}" SET "SELECTED" = false WHERE "{}_ID" = $1"#,
            table, col
        );
        sqlx::query(&query_unselect)
            .bind(entity_id.to_string())
            .execute(&self.pool)
            .await?;

        let query_select = format!(
            r#"UPDATE "{}" SET "SELECTED" = true WHERE "ID" = $1"#,
            table
        );
        sqlx::query(&query_select)
            .bind(thumbnail_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_in_table(&self, table: &str, thumbnail_id: Uuid) -> Result<(), sqlx::Error> {
        let query = format!(r#"DELETE FROM "{}" WHERE "ID" = $1"#, table);
        sqlx::query(&query)
            .bind(thumbnail_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn find_book_thumbnails(&self, book_id: Uuid) -> Result<Vec<Thumbnail>, sqlx::Error> {
        self.find_by_table("THUMBNAIL_BOOK", book_id).await
    }

    pub async fn find_series_thumbnails(&self, series_id: Uuid) -> Result<Vec<Thumbnail>, sqlx::Error> {
        self.find_by_table("THUMBNAIL_SERIES", series_id).await
    }

    pub async fn find_readlist_thumbnails(&self, readlist_id: Uuid) -> Result<Vec<Thumbnail>, sqlx::Error> {
        self.find_by_table("THUMBNAIL_READLIST", readlist_id).await
    }

    pub async fn find_collection_thumbnails(&self, collection_id: Uuid) -> Result<Vec<Thumbnail>, sqlx::Error> {
        self.find_by_table("THUMBNAIL_COLLECTION", collection_id).await
    }

    pub async fn find_book_thumbnail_by_id(&self, id: Uuid) -> Result<Option<Thumbnail>, sqlx::Error> {
        self.find_by_id_in_table("THUMBNAIL_BOOK", id).await
    }

    pub async fn mark_book_thumbnail_selected(&self, book_id: Uuid, thumbnail_id: Uuid) -> Result<(), sqlx::Error> {
        self.mark_selected_in_table("THUMBNAIL_BOOK", book_id, thumbnail_id).await
    }

    pub async fn delete_book_thumbnail(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.delete_in_table("THUMBNAIL_BOOK", id).await
    }

    pub async fn mark_series_thumbnail_selected(&self, series_id: Uuid, thumbnail_id: Uuid) -> Result<(), sqlx::Error> {
        self.mark_selected_in_table("THUMBNAIL_SERIES", series_id, thumbnail_id).await
    }

    pub async fn delete_series_thumbnail(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.delete_in_table("THUMBNAIL_SERIES", id).await
    }

    pub async fn mark_readlist_thumbnail_selected(&self, readlist_id: Uuid, thumbnail_id: Uuid) -> Result<(), sqlx::Error> {
        self.mark_selected_in_table("THUMBNAIL_READLIST", readlist_id, thumbnail_id).await
    }

    pub async fn delete_readlist_thumbnail(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.delete_in_table("THUMBNAIL_READLIST", id).await
    }

    pub async fn mark_collection_thumbnail_selected(&self, collection_id: Uuid, thumbnail_id: Uuid) -> Result<(), sqlx::Error> {
        self.mark_selected_in_table("THUMBNAIL_COLLECTION", collection_id, thumbnail_id).await
    }

    pub async fn delete_collection_thumbnail(&self, id: Uuid) -> Result<(), sqlx::Error> {
        self.delete_in_table("THUMBNAIL_COLLECTION", id).await
    }
}
