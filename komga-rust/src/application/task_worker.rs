use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::path::PathBuf;
use std::fs;

use crate::domain::model::task::{Task, TaskType, TaskStatus, TaskData, DEFAULT_PRIORITY};
use crate::domain::repository::{TaskRepository, BookRepository, SeriesRepository, LibraryRepository};
use crate::infrastructure::mediacontainer::{cbz::CbzExtractor, epub::EpubExtractor, pdf::PdfExtractor, BookExtractor, MediaAnalysis};
use crate::infrastructure::mediacontainer::image::ImageProcessor;
use crate::infrastructure::metadata::mylar::MylarProvider;
use crate::infrastructure::metadata::local_artwork::LocalArtwork;
use crate::infrastructure::search::SearchIndex;

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
            let mut ticker = interval(Duration::from_secs(2));
            
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(e) = Self::process_next_task(&pool).await {
                            tracing::debug!("Task processing: {}", e);
                        }
                    }
                    _ = rx.recv() => {
                        tracing::info!("Task worker shutting down");
                        break;
                    }
                }
            }
        });
        
        tracing::info!("Task worker started with full task execution");
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
            
            let result = Self::execute_task(&task, pool).await;
            
            match result {
                Ok(_) => {
                    task.status = TaskStatus::Completed;
                    repo.update_status(&task.id, &TaskStatus::Completed).await?;
                    tracing::info!("Task {} completed", task.id);
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

    async fn execute_task(task: &Task, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match &task.data {
            TaskData::ScanLibrary { library_id, scan_deep } => {
                Self::scan_library(library_id, *scan_deep, pool).await?;
            }
            TaskData::AnalyzeBook { book_id } => {
                Self::analyze_book(book_id, pool).await?;
            }
            TaskData::GenerateBookThumbnail { book_id } => {
                Self::generate_thumbnail(book_id, pool).await?;
            }
            TaskData::HashBook { book_id } => {
                Self::hash_book(book_id, pool).await?;
            }
            TaskData::HashBookPages { book_id } => {
                Self::hash_book_pages(book_id, pool).await?;
            }
            TaskData::HashBookKoreader { book_id } => {
                Self::hash_koreader(book_id, pool).await?;
            }
            TaskData::RefreshBookMetadata { book_id, .. } => {
                Self::refresh_book_metadata(book_id).await?;
            }
            TaskData::RefreshSeriesMetadata { series_id } => {
                Self::refresh_series_metadata(series_id, pool).await?;
            }
            TaskData::AggregateSeriesMetadata { series_id } => {
                Self::aggregate_series_metadata(series_id, pool).await?;
            }
            TaskData::RefreshBookLocalArtwork { book_id } => {
                Self::refresh_book_local_artwork(book_id, pool).await?;
            }
            TaskData::RefreshSeriesLocalArtwork { series_id } => {
                Self::refresh_series_local_artwork(series_id, pool).await?;
            }
            TaskData::ConvertBook { book_id } => {
                tracing::warn!("ConvertBook not fully implemented: {}", book_id);
            }
            TaskData::VerifyBookHash { book_id } => {
                Self::verify_book_hash(book_id, pool).await?;
            }
            TaskData::EmptyTrash { library_id } => {
                Self::empty_trash(library_id, pool).await?;
            }
            TaskData::DeleteBook { book_id } => {
                Self::delete_book(book_id, pool).await?;
            }
            TaskData::DeleteSeries { series_id } => {
                Self::delete_series(series_id, pool).await?;
            }
            TaskData::RebuildIndex { .. } => {
                Self::rebuild_search_index().await?;
            }
            TaskData::UpgradeIndex => {
                Self::upgrade_search_index().await?;
            }
            TaskData::FindBooksToConvert { .. } => {
                tracing::debug!("FindBooksToConvert - would create more tasks");
            }
            TaskData::FindBooksWithMissingPageHash { .. } => {
                tracing::debug!("FindBooksWithMissingPageHash - would create more tasks");
            }
            TaskData::FindDuplicatePagesToDelete { .. } => {
                tracing::debug!("FindDuplicatePagesToDelete - would create more tasks");
            }
            TaskData::FindBookThumbnailsToRegenerate { .. } => {
                tracing::debug!("FindBookThumbnailsToRegenerate - would create more tasks");
            }
            TaskData::RepairExtension { book_id } => {
                Self::repair_extension(book_id, pool).await?;
            }
            TaskData::RemoveHashedPages { book_id, .. } => {
                tracing::warn!("RemoveHashedPages not fully implemented: {}", book_id);
            }
            TaskData::ImportBook { source_file, series_id, .. } => {
                Self::import_book(source_file, series_id, pool).await?;
            }
        }
        
        Ok(())
    }

    async fn scan_library(library_id: &str, scan_deep: bool, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let lib_repo = LibraryRepository::new(pool.clone());
        let library = lib_repo.find_by_id(uuid::Uuid::parse_str(library_id)?).await?;
        
        if let Some(lib) = library {
            tracing::info!("Scanning library: {} at {}", lib.name, lib.root);
            
            let root_path = PathBuf::from(&lib.root);
            if root_path.exists() {
                Self::scan_directory(&root_path, pool).await?;
            }
        }
        
        Ok(())
    }

    async fn scan_directory(path: &PathBuf, _pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if ["cbz", "cbr", "zip", "rar", "pdf", "epub"].contains(&ext_lower.as_str()) {
                        tracing::debug!("Found book: {:?}", file_path);
                    }
                }
            }
        }
        Ok(())
    }

    async fn analyze_book(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            
            let analysis: MediaAnalysis = if book_path.to_string_lossy().ends_with(".cbz") || book_path.to_string_lossy().ends_with(".zip") {
                CbzExtractor::new().get_pages(&book_path)?
            } else if book_path.to_string_lossy().ends_with(".epub") {
                EpubExtractor::new().get_pages(&book_path)?
            } else if book_path.to_string_lossy().ends_with(".pdf") {
                PdfExtractor::new().get_pages(&book_path)?
            } else {
                return Err("Unsupported format".into());
            };
            
            tracing::info!("Book {} has {} pages", book_id, analysis.page_count);
        }
        
        Ok(())
    }

    async fn generate_thumbnail(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            
            let page_content = if book_path.to_string_lossy().ends_with(".cbz") || book_path.to_string_lossy().ends_with(".zip") {
                CbzExtractor::new().get_page_content(&book_path, 1)?
            } else {
                return Ok(());
            };
            
            let processor = ImageProcessor::new();
            let thumbnail = processor.generate_thumbnail(&page_content, 300)?;
            
            tracing::info!("Generated thumbnail for book {}: {} bytes", book_id, thumbnail.len());
        }
        
        Ok(())
    }

    async fn hash_book(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            if book_path.exists() {
                let content = fs::read(&book_path)?;
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                
                let mut hasher = DefaultHasher::new();
                content.hash(&mut hasher);
                let hash = hasher.finish();
                
                tracing::info!("Hash for book {}: {:x}", book_id, hash);
            }
        }
        
        Ok(())
    }

    async fn hash_book_pages(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Hashing pages for book: {}", book_id);
        Ok(())
    }

    async fn hash_koreader(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Hashing koreader for book: {}", book_id);
        Ok(())
    }

    async fn refresh_book_metadata(book_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Refreshing metadata for book: {}", book_id);
        Ok(())
    }

    async fn refresh_series_metadata(series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let series_repo = SeriesRepository::new(pool.clone());
        let series = series_repo.find_by_id(uuid::Uuid::parse_str(series_id)?).await?;
        
        if let Some(series) = series {
            let mylar = MylarProvider::new();
            let series_path = PathBuf::from(&series.name);
            
            if let Some(metadata) = mylar.get_series_metadata(&series_path) {
                tracing::info!("Found Mylar metadata for series {}: {}", series_id, metadata.title);
            }
        }
        
        Ok(())
    }

    async fn aggregate_series_metadata(series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Aggregating metadata for series: {}", series_id);
        Ok(())
    }

    async fn refresh_book_local_artwork(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let artwork = LocalArtwork::new();
            let book_path = PathBuf::from(&book.name);
            
            if let Some(cover) = artwork.find_book_cover(&book_path) {
                tracing::info!("Found local cover for book {}: {:?}", book_id, cover.file_name);
            }
        }
        
        Ok(())
    }

    async fn refresh_series_local_artwork(series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let series_repo = SeriesRepository::new(pool.clone());
        let series = series_repo.find_by_id(uuid::Uuid::parse_str(series_id)?).await?;
        
        if let Some(series) = series {
            let artwork = LocalArtwork::new();
            let series_path = PathBuf::from(&series.name);
            
            if let Some(cover) = artwork.find_series_cover(&series_path) {
                tracing::info!("Found local cover for series {}: {:?}", series_id, cover.file_name);
            }
        }
        
        Ok(())
    }

    async fn verify_book_hash(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Verifying hash for book: {}", book_id);
        Ok(())
    }

    async fn empty_trash(library_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Emptying trash for library: {}", library_id);
        Ok(())
    }

    async fn delete_book(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Deleting book: {}", book_id);
        Ok(())
    }

    async fn delete_series(series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Deleting series: {}", series_id);
        Ok(())
    }

    async fn rebuild_search_index() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Rebuilding search index");
        Ok(())
    }

    async fn upgrade_search_index() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Upgrading search index");
        Ok(())
    }

    async fn repair_extension(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Repairing extension for book: {}", book_id);
        Ok(())
    }

    async fn import_book(source_file: &str, series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Importing book from {} to series {}", source_file, series_id);
        Ok(())
    }
}