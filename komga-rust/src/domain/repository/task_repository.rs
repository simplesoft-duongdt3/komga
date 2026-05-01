use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::task::{Task, TaskType, TaskStatus, TaskData};

pub struct TaskRepository {
    pool: PgPool,
}

impl TaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, task: &Task) -> Result<Task, sqlx::Error> {
        let row = sqlx::query(
            r#"INSERT INTO "TASK" 
            ("ID", "TYPE", "STATUS", "PRIORITY", "CREATED_DATE", "DATA")
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *"#
        )
        .bind(&task.id)
        .bind(task.task_type.as_str())
        .bind(task.status.as_str())
        .bind(task.priority)
        .bind(task.created_date)
        .bind(serde_json::to_string(&task.data).unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_task(row))
    }

    pub async fn get_next_task(&self) -> Result<Option<Task>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "TASK" 
            WHERE "STATUS" = 'QUEUED' 
            ORDER BY "PRIORITY" DESC, "CREATED_DATE" ASC 
            LIMIT 1"#
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_task))
    }

    pub async fn find_all(&self, status: Option<&str>, limit: usize) -> Result<Vec<Task>, sqlx::Error> {
        let query = if let Some(status) = status {
            sqlx::query(
                r#"SELECT * FROM "TASK" WHERE "STATUS" = $1 ORDER BY "CREATED_DATE" DESC LIMIT $2"#
            )
            .bind(status)
            .bind(limit as i64)
        } else {
            sqlx::query(
                r#"SELECT * FROM "TASK" ORDER BY "CREATED_DATE" DESC LIMIT $1"#
            )
            .bind(limit as i64)
        };

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_task).collect())
    }

    pub async fn update_status(&self, id: &str, status: &TaskStatus) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "TASK" SET "STATUS" = $2, "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP WHERE "ID" = $1"#
        )
        .bind(id)
        .bind(status.as_str())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Task>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "TASK" WHERE "ID" = $1"#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_task))
    }

    pub async fn delete_completed(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "TASK" WHERE "STATUS" IN ('COMPLETED', 'CANCELLED')"#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn row_to_task(row: sqlx::postgres::PgRow) -> Task {
    let task_type_str: String = row.get("TYPE");
    let status_str: String = row.get("STATUS");
    let data_str: String = row.get::<Option<String>, _>("DATA").unwrap_or_default();
    
    let task_type = match task_type_str.as_str() {
        "ScanLibrary" => TaskType::ScanLibrary,
        "FindBooksToConvert" => TaskType::FindBooksToConvert,
        "FindBooksWithMissingPageHash" => TaskType::FindBooksWithMissingPageHash,
        "FindDuplicatePagesToDelete" => TaskType::FindDuplicatePagesToDelete,
        "EmptyTrash" => TaskType::EmptyTrash,
        "AnalyzeBook" => TaskType::AnalyzeBook,
        "VerifyBookHash" => TaskType::VerifyBookHash,
        "GenerateBookThumbnail" => TaskType::GenerateBookThumbnail,
        "RefreshBookMetadata" => TaskType::RefreshBookMetadata,
        "HashBook" => TaskType::HashBook,
        "HashBookPages" => TaskType::HashBookPages,
        "HashBookKoreader" => TaskType::HashBookKoreader,
        "RefreshSeriesMetadata" => TaskType::RefreshSeriesMetadata,
        "AggregateSeriesMetadata" => TaskType::AggregateSeriesMetadata,
        "RefreshBookLocalArtwork" => TaskType::RefreshBookLocalArtwork,
        "RefreshSeriesLocalArtwork" => TaskType::RefreshSeriesLocalArtwork,
        "ImportBook" => TaskType::ImportBook,
        "ConvertBook" => TaskType::ConvertBook,
        "RepairExtension" => TaskType::RepairExtension,
        "RemoveHashedPages" => TaskType::RemoveHashedPages,
        "RebuildIndex" => TaskType::RebuildIndex,
        "UpgradeIndex" => TaskType::UpgradeIndex,
        "DeleteBook" => TaskType::DeleteBook,
        "DeleteSeries" => TaskType::DeleteSeries,
        "FindBookThumbnailsToRegenerate" => TaskType::FindBookThumbnailsToRegenerate,
        _ => TaskType::ScanLibrary,
    };
    
    let status = match status_str.as_str() {
        "QUEUED" => TaskStatus::Queued,
        "RUNNING" => TaskStatus::Running,
        "COMPLETED" => TaskStatus::Completed,
        "FAILED" => TaskStatus::Failed,
        "CANCELLED" => TaskStatus::Cancelled,
        _ => TaskStatus::Queued,
    };
    
    let data: TaskData = serde_json::from_str(&data_str).unwrap_or(TaskData::ScanLibrary { 
        library_id: String::new(), 
        scan_deep: false 
    });

    Task {
        id: row.get("ID"),
        task_type,
        status,
        priority: row.get("PRIORITY"),
        created_date: row.get("CREATED_DATE"),
        scheduled_date: row.get("SCHEDULED_DATE"),
        execution_start_date: row.get("EXECUTION_START_DATE"),
        execution_end_date: row.get("EXECUTION_END_DATE"),
        result: None,
        data,
    }
}