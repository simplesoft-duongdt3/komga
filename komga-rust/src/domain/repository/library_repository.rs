use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::model::library::{Library, SeriesCover, ScanInterval};

pub struct LibraryRepository {
    pool: PgPool,
}

impl LibraryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, library: &Library) -> Result<Library, sqlx::Error> {
        let row = sqlx::query(
            r#"INSERT INTO "LIBRARY" 
            ("ID", "CREATED_DATE", "LAST_MODIFIED_DATE", "NAME", "ROOT", 
             "IMPORT_COMICINFO_BOOK", "IMPORT_COMICINFO_SERIES", "IMPORT_COMICINFO_COLLECTION",
             "IMPORT_EPUB_BOOK", "IMPORT_EPUB_SERIES", "SCAN_FORCE_MODIFIED_TIME", "SCAN_STARTUP",
             "IMPORT_LOCAL_ARTWORK", "IMPORT_COMICINFO_READLIST", "IMPORT_BARCODE_ISBN",
             "CONVERT_TO_CBZ", "REPAIR_EXTENSIONS", "EMPTY_TRASH_AFTER_SCAN", "IMPORT_MYLAR_SERIES",
             "SERIES_COVER", "UNAVAILABLE_DATE", "HASH_FILES", "HASH_PAGES", "ANALYZE_DIMENSIONS",
             "IMPORT_COMICINFO_SERIES_APPEND_VOLUME", "ONESHOTS_DIRECTORY", "SCAN_CBX", "SCAN_PDF",
             "SCAN_EPUB", "SCAN_INTERVAL", "HASH_KOREADER")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31)
            RETURNING *"#
        )
        .bind(library.id.to_string())
        .bind(library.created_date)
        .bind(library.last_modified_date)
        .bind(&library.name)
        .bind(&library.root)
        .bind(library.import_comicinfo_book)
        .bind(library.import_comicinfo_series)
        .bind(library.import_comicinfo_collection)
        .bind(library.import_epub_book)
        .bind(library.import_epub_series)
        .bind(library.scan_force_modified_time)
        .bind(library.scan_startup)
        .bind(library.import_local_artwork)
        .bind(library.import_comicinfo_readlist)
        .bind(library.import_barcode_isbn)
        .bind(library.convert_to_cbz)
        .bind(library.repair_extensions)
        .bind(library.empty_trash_after_scan)
        .bind(library.import_mylar_series)
        .bind(format!("{:?}", library.series_cover))
        .bind(library.unavailable_date)
        .bind(library.hash_files)
        .bind(library.hash_pages)
        .bind(library.analyze_dimensions)
        .bind(library.import_comicinfo_series_append_volume)
        .bind(&library.oneshots_directory)
        .bind(library.scan_cbx)
        .bind(library.scan_pdf)
        .bind(library.scan_epub)
        .bind(format!("{:?}", library.scan_interval))
        .bind(library.hash_koreader)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_library(row))
    }

    pub async fn find_all(&self) -> Result<Vec<Library>, sqlx::Error> {
        let rows = sqlx::query(
            r#"SELECT * FROM "LIBRARY" ORDER BY "NAME""#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_library).collect())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Library>, sqlx::Error> {
        let result = sqlx::query(
            r#"SELECT * FROM "LIBRARY" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(row_to_library))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM "LIBRARY" WHERE "ID" = $1"#
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update(&self, library: &Library) -> Result<Library, sqlx::Error> {
        let row = sqlx::query(
            r#"UPDATE "LIBRARY" SET
            "NAME" = $2, "ROOT" = $3, "LAST_MODIFIED_DATE" = CURRENT_TIMESTAMP,
            "IMPORT_COMICINFO_BOOK" = $4, "IMPORT_COMICINFO_SERIES" = $5,
            "IMPORT_EPUB_BOOK" = $6, "IMPORT_EPUB_SERIES" = $7,
            "IMPORT_LOCAL_ARTWORK" = $8, "IMPORT_BARCODE_ISBN" = $9,
            "CONVERT_TO_CBZ" = $10, "HASH_FILES" = $11, "HASH_PAGES" = $12,
            "ANALYZE_DIMENSIONS" = $13
            WHERE "ID" = $1
            RETURNING *"#
        )
        .bind(library.id.to_string())
        .bind(&library.name)
        .bind(&library.root)
        .bind(library.import_comicinfo_book)
        .bind(library.import_comicinfo_series)
        .bind(library.import_epub_book)
        .bind(library.import_epub_series)
        .bind(library.import_local_artwork)
        .bind(library.import_barcode_isbn)
        .bind(library.convert_to_cbz)
        .bind(library.hash_files)
        .bind(library.hash_pages)
        .bind(library.analyze_dimensions)
        .fetch_one(&self.pool)
        .await?;

        Ok(row_to_library(row))
    }
}

fn row_to_library(row: sqlx::postgres::PgRow) -> Library {
    Library {
        id: Uuid::parse_str(&row.get::<String, _>("ID")).unwrap_or_default(),
        created_date: row.get::<DateTime<Utc>, _>("CREATED_DATE"),
        last_modified_date: row.get::<DateTime<Utc>, _>("LAST_MODIFIED_DATE"),
        name: row.get::<String, _>("NAME"),
        root: row.get::<String, _>("ROOT"),
        import_comicinfo_book: row.get::<bool, _>("IMPORT_COMICINFO_BOOK"),
        import_comicinfo_series: row.get::<bool, _>("IMPORT_COMICINFO_SERIES"),
        import_comicinfo_collection: row.get::<bool, _>("IMPORT_COMICINFO_COLLECTION"),
        import_epub_book: row.get::<bool, _>("IMPORT_EPUB_BOOK"),
        import_epub_series: row.get::<bool, _>("IMPORT_EPUB_SERIES"),
        scan_force_modified_time: row.get::<bool, _>("SCAN_FORCE_MODIFIED_TIME"),
        scan_startup: row.get::<bool, _>("SCAN_STARTUP"),
        import_local_artwork: row.get::<bool, _>("IMPORT_LOCAL_ARTWORK"),
        import_comicinfo_readlist: row.get::<bool, _>("IMPORT_COMICINFO_READLIST"),
        import_barcode_isbn: row.get::<bool, _>("IMPORT_BARCODE_ISBN"),
        convert_to_cbz: row.get::<bool, _>("CONVERT_TO_CBZ"),
        repair_extensions: row.get::<bool, _>("REPAIR_EXTENSIONS"),
        empty_trash_after_scan: row.get::<bool, _>("EMPTY_TRASH_AFTER_SCAN"),
        import_mylar_series: row.get::<bool, _>("IMPORT_MYLAR_SERIES"),
        series_cover: SeriesCover::try_from(row.get::<String, _>("SERIES_COVER").as_str()).unwrap_or_default(),
        unavailable_date: row.get::<Option<DateTime<Utc>>, _>("UNAVAILABLE_DATE"),
        hash_files: row.get::<bool, _>("HASH_FILES"),
        hash_pages: row.get::<bool, _>("HASH_PAGES"),
        analyze_dimensions: row.get::<bool, _>("ANALYZE_DIMENSIONS"),
        import_comicinfo_series_append_volume: row.get::<bool, _>("IMPORT_COMICINFO_SERIES_APPEND_VOLUME"),
        oneshots_directory: row.get::<Option<String>, _>("ONESHOTS_DIRECTORY"),
        scan_cbx: row.get::<bool, _>("SCAN_CBX"),
        scan_pdf: row.get::<bool, _>("SCAN_PDF"),
        scan_epub: row.get::<bool, _>("SCAN_EPUB"),
        scan_interval: ScanInterval::try_from(row.get::<String, _>("SCAN_INTERVAL").as_str()).unwrap_or_default(),
        hash_koreader: row.get::<bool, _>("HASH_KOREADER"),
    }
}