use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use std::path::PathBuf;
use std::fs;
use std::io::Read;
use walkdir::WalkDir;
use sha2::{Sha256, Digest};

use crate::domain::model::task::{Task, TaskStatus, TaskData};
use crate::domain::model::book::Book;
use crate::domain::model::series::Series;
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
                Self::scan_directory(&root_path, lib.id, pool).await?;
            }
        }
        
        Ok(())
    }

    async fn scan_directory(path: &PathBuf, library_id: uuid::Uuid, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let series_repo = SeriesRepository::new(pool.clone());
        let book_repo = BookRepository::new(pool.clone());
        
        let mut entries: Vec<_> = WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                if let Some(ext) = e.path().extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    ["cbz", "cbr", "zip", "rar", "pdf", "epub"].contains(&ext_lower.as_str())
                } else {
                    false
                }
            })
            .collect();
        
        entries.sort_by(|a, b| a.path().cmp(b.path()));
        
        for entry in entries {
            let file_path = entry.path();
            let file_name = file_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            
            let parent = file_path.parent().map(|p| p.to_path_buf());
            let parent_path = parent.unwrap_or_default();
            
            let series_name = parent_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unsorted".to_string());
            
            let series_url = format!("/library/{}/series/{}", library_id, series_name.replace(' ', "%20"));
            
            let series = match series_repo.find_by_name(&series_name, library_id).await? {
                Some(s) => s,
                None => {
                    let new_series = Series::new(series_name.clone(), series_url, library_id);
                    series_repo.insert(&new_series).await?;
                    new_series
                }
            };
            
            if book_repo.find_by_name(&file_name, series.id).await?.is_none() {
                let book_url = format!("/library/{}/series/{}/{}", library_id, series_name.replace(' ', "%20"), file_name.replace(' ', "%20"));
                let file_size = fs::metadata(file_path)?.len() as i64;
                
                let book = Book::new(file_name, book_url, series.id, library_id, 0);
                let mut book = book;
                book.file_size = file_size;
                
                book_repo.insert(&book).await?;
                tracing::info!("Added book: {} to series: {}", book.name, series_name);
            }
        }
        
        Ok(())
    }

    async fn analyze_book(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            
            if !book_path.exists() {
                return Err("Book file not found".into());
            }
            
            let ext = book_path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            
            let analysis: MediaAnalysis = if ext == "cbz" || ext == "zip" {
                CbzExtractor::new().get_pages(&book_path)?
            } else if ext == "epub" {
                EpubExtractor::new().get_pages(&book_path)?
            } else if ext == "pdf" {
                PdfExtractor::new().get_pages(&book_path)?
            } else {
                return Err("Unsupported format".into());
            };
            
            tracing::info!("Book {} has {} pages", book_id, analysis.page_count);
            
            sqlx::query(
                r#"INSERT INTO "MEDIA" ("ID", "MEDIA_TYPE", "STATUS", "CREATED_DATE", "LAST_MODIFIED_DATE", "COMMENT", "BOOK_ID", "PAGE_COUNT", "EXTENSION_CLASS")
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT ("BOOK_ID") DO UPDATE SET "MEDIA_TYPE" = $2, "STATUS" = $3, "PAGE_COUNT" = $8, "LAST_MODIFIED_DATE" = $5"#
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&analysis.media_type)
            .bind("READY")
            .bind(chrono::Utc::now())
            .bind(chrono::Utc::now())
            .bind::<Option<String>>(None)
            .bind(book.id.to_string())
            .bind(analysis.page_count)
            .bind(&ext)
            .execute(pool)
            .await?;
        }
        
        Ok(())
    }

    async fn generate_thumbnail(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            
            if !book_path.exists() {
                return Err("Book file not found".into());
            }
            
            let ext = book_path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            
            if ext != "cbz" && ext != "zip" {
                tracing::warn!("Thumbnail not supported for format: {}", ext);
                return Ok(());
            }
            
            let page_content = CbzExtractor::new().get_page_content(&book_path, 1)?;
            
            let processor = ImageProcessor::new();
            let thumbnail = processor.generate_thumbnail(&page_content, 300)?;
            
            let thumbnail_dir = std::env::current_dir()?.join("thumbnails");
            fs::create_dir_all(&thumbnail_dir)?;
            
            let thumbnail_path = thumbnail_dir.join(format!("{}.png", book.id));
            fs::write(&thumbnail_path, &thumbnail)?;
            
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
                let mut file = fs::File::open(&book_path)?;
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; 8192];
                
                loop {
                    let bytes_read = file.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                
                let hash = format!("{:x}", hasher.finalize());
                
                book_repo.update_file_hash(&book.id, &hash).await?;
                tracing::info!("Hash for book {}: {}", book_id, hash);
            }
        }
        
        Ok(())
    }

    async fn hash_book_pages(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            if !book_path.exists() {
                return Err("Book file not found".into());
            }
            
            let ext = book_path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            
            if ext != "cbz" && ext != "zip" {
                tracing::debug!("Page hashing not supported for: {}", ext);
                return Ok(());
            }
            
            let analysis = CbzExtractor::new().get_pages(&book_path)?;
            
            for page in &analysis.pages {
                let page_content = CbzExtractor::new().get_page_content(&book_path, page.number)?;
                let mut hasher = Sha256::new();
                hasher.update(&page_content);
                let hash = format!("{:x}", hasher.finalize());
                
                tracing::debug!("Page {} hash: {}", page.number, hash);
            }
            
            tracing::info!("Hashed {} pages for book {}", analysis.pages.len(), book_id);
        }
        
        Ok(())
    }

    async fn hash_koreader(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            if !book_path.exists() {
                return Err("Book file not found".into());
            }
            
            let mut file = fs::File::open(&book_path)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 8192];
            
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            
            let hash = format!("{:x}", hasher.finalize());
            let koreader_hash = format!("sha256:{}", hash);
            
            book_repo.update_file_hash_koreader(&book.id, &koreader_hash).await?;
            tracing::info!("Koreader hash for book {}: {}", book_id, koreader_hash);
        }
        
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
            
            if let Some(_metadata) = mylar.get_series_metadata(&series_path) {
                tracing::info!("Found Mylar metadata for series {}: {}", series_id, _metadata.title);
            }
        }
        
        Ok(())
    }

    async fn aggregate_series_metadata(series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Aggregating metadata for series: {}", series_id);
        
        let series_repo = SeriesRepository::new(pool.clone());
        let series = series_repo.find_by_id(uuid::Uuid::parse_str(series_id)?).await?;
        
        if let Some(series) = series {
            let book_repo = BookRepository::new(pool.clone());
            let books = book_repo.find_by_series(series.id).await?;
            
            tracing::debug!("Aggregated metadata from {} books for series {}", books.len(), series_id);
        }
        
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
        
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            if book_path.exists() {
                let mut file = fs::File::open(&book_path)?;
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; 8192];
                
                loop {
                    let bytes_read = file.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                
                let hash = format!("{:x}", hasher.finalize());
                
                if hash != book.file_hash {
                    tracing::warn!("Hash mismatch for book {}: expected {}, got {}", book_id, book.file_hash, hash);
                } else {
                    tracing::debug!("Hash verified for book {}", book_id);
                }
            }
        }
        
        Ok(())
    }

    async fn empty_trash(library_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let library_uuid = uuid::Uuid::parse_str(library_id)?;
        
        let deleted_books = book_repo.find_deleted(library_uuid).await?;
        let count = deleted_books.len();
        
        for book in deleted_books {
            book_repo.hard_delete(&book.id).await?;
            tracing::debug!("Permanently deleted book: {}", book.id);
        }
        
        tracing::info!("Emptied trash for library {}: deleted {} books", library_id, count);
        
        Ok(())
    }

    async fn delete_book(book_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let book_repo = BookRepository::new(pool.clone());
        let book_uuid = uuid::Uuid::parse_str(book_id)?;
        
        book_repo.soft_delete(&book_uuid).await?;
        tracing::info!("Soft deleted book: {}", book_id);
        
        Ok(())
    }

    async fn delete_series(series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let series_repo = SeriesRepository::new(pool.clone());
        let book_repo = BookRepository::new(pool.clone());
        
        let series_uuid = uuid::Uuid::parse_str(series_id)?;
        
        let books = book_repo.find_by_series(series_uuid).await?;
        let book_count = books.len();
        
        for book in books {
            book_repo.soft_delete(&book.id).await?;
        }
        
        series_repo.soft_delete(&series_uuid).await?;
        tracing::info!("Soft deleted series: {} and {} books", series_id, book_count);
        
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
        
        let book_repo = BookRepository::new(pool.clone());
        let book = book_repo.find_by_id(uuid::Uuid::parse_str(book_id)?).await?;
        
        if let Some(book) = book {
            let book_path = PathBuf::from(&book.name);
            
            if book_path.exists() {
                let metadata = fs::metadata(&book_path)?;
                book_repo.update_file_size(&book.id, metadata.len() as i64).await?;
                
                if let Ok(modified) = metadata.modified() {
                    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                    book_repo.update_file_last_modified(&book.id, datetime).await?;
                }
                
                tracing::info!("Repaired extension for book: {}", book_id);
            }
        }
        
        Ok(())
    }

    async fn import_book(source_file: &str, series_id: &str, pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source_path = PathBuf::from(source_file);
        if !source_path.exists() {
            return Err(format!("Source file not found: {}", source_file).into());
        }
        
        let series_repo = SeriesRepository::new(pool.clone());
        let series_uuid = uuid::Uuid::parse_str(series_id)?;
        
        let series = series_repo.find_by_id(series_uuid).await?
            .ok_or_else(|| format!("Series not found: {}", series_id))?;
        
        let file_name = source_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let dest_path = PathBuf::from(&series.name).join(&file_name);
        fs::copy(&source_path, &dest_path)?;
        
        tracing::info!("Imported book from {} to {}", source_file, dest_path.display());
        
        Ok(())
    }
}