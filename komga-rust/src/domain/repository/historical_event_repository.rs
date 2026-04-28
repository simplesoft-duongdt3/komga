use sqlx::PgPool;
use sqlx::Row;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct HistoricalEvent {
    pub id: String,
    pub event_type: String,
    pub book_id: Option<String>,
    pub series_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub properties: Vec<(String, String)>,
}

pub struct HistoricalEventRepository {
    pool: PgPool,
}

impl HistoricalEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_all(&self, limit: i64) -> Result<Vec<HistoricalEvent>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "HISTORICAL_EVENT" ORDER BY "TIMESTAMP" DESC LIMIT $1"#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let id: String = row.get("ID");
            let props = self.get_properties(&id).await?;
            events.push(HistoricalEvent {
                id: id.clone(),
                event_type: row.get("TYPE"),
                book_id: row.get("BOOK_ID"),
                series_id: row.get("SERIES_ID"),
                timestamp: row.get("TIMESTAMP"),
                properties: props,
            });
        }

        Ok(events)
    }

    async fn get_properties(&self, event_id: &str) -> Result<Vec<(String, String)>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT "KEY", "VALUE" FROM "HISTORICAL_EVENT_PROPERTIES" WHERE "ID" = $1"#
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| {
            (r.get::<String, _>("KEY"), r.get::<String, _>("VALUE"))
        }).collect())
    }
}
