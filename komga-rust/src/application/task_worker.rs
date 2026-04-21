use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::sync::Arc;

use crate::domain::model::task::{Task, TaskType, TaskStatus, TaskData};
use crate::domain::repository::task_repository::TaskRepository;

pub struct TaskWorker {
    pool: PgPool,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl TaskWorker {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self) {
        let (tx, mut rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(tx);
        
        let pool = self.pool.clone();
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));
            
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = Self::process_next_task(&pool).await {
                            tracing::error!("Task processing error: {}", e);
                        }
                    }
                    _ = rx.recv() => {
                        tracing::info!("Task worker shutting down");
                        break;
                    }
                }
            }
        });
        
        tracing::info!("Task worker started");
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    async fn process_next_task(pool: &PgPool) -> Result<(), sqlx::Error> {
        let repo = TaskRepository::new(pool.clone());
        
        if let Some(mut task) = repo.get_next_task().await? {
            task.status = TaskStatus::Running;
            repo.update_status(&task.id, &TaskStatus::Running).await?;
            
            let result = Self::execute_task(&task).await;
            
            match result {
                Ok(_) => {
                    task.status = TaskStatus::Completed;
                    repo.update_status(&task.id, &TaskStatus::Completed).await?;
                }
                Err(e) => {
                    tracing::error!("Task {} failed: {}", task.id, e);
                    task.status = TaskStatus::Failed;
                    repo.update_status(&task.id, &TaskStatus::Failed).await?;
                }
            }
        }
        
        Ok(())
    }

    async fn execute_task(task: &Task) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &task.data {
            TaskData::AnalyzeBook { book_id } => {
                tracing::info!("Analyzing book: {}", book_id);
            }
            TaskData::GenerateBookThumbnail { book_id } => {
                tracing::info!("Generating thumbnail for: {}", book_id);
            }
            TaskData::HashBook { book_id } => {
                tracing::info!("Hashing book: {}", book_id);
            }
            TaskData::RefreshSeriesMetadata { series_id } => {
                tracing::info!("Refreshing series metadata: {}", series_id);
            }
            TaskData::ScanLibrary { library_id, scan_deep } => {
                tracing::info!("Scanning library: {} (deep: {})", library_id, scan_deep);
            }
            _ => {
                tracing::debug!("Task type not implemented: {:?}", task.task_type);
            }
        }
        
        Ok(())
    }
}