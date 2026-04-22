# Komga Rust - Sprint 6: Page Streaming & Media Extraction

## Goal: Fix broken page extraction and implement proper media serving

### Tasks

- [ ] **1. Fix CBZ Page Extraction** (`api/book.rs:82-115`)
  - Connect to existing `CbzExtractor` to extract specific page
  - Return correct media type (image/jpeg or image/png)
  - Handle page range requests properly

- [ ] **2. Fix EPUB Page Extraction**
  - Connect `EpubExtractor::get_page_content()` to API
  - Render EPUB pages to images

- [ ] **3. Fix PDF Page Extraction**
  - Connect `PdfExtractor::get_page_content()` to API  
  - Convert PDF pages to images

- [ ] **4. Fix Thumbnail Serving** (`api/book.rs:122-153`)
  - Store generated thumbnails (file system or Redis)
  - Serve cached thumbnails instead of raw file bytes
  - Connect to `ImageProcessor::generate_thumbnail()`

- [ ] **5. Get Book Pages API** (`api/book.rs:66-79`)
  - Return actual page list from media analysis
  - Connect to existing media container extractors

---

# Komga Rust - Sprint 7: Complete Task Worker

## Goal: Implement all 26 task types with full functionality

### Tasks

- [ ] **1. ScanLibrary** (`task_worker.rs:169-198`)
  - Recursively scan library directories
  - Create Series/Book database entries for new files
  - Handle folder structure (series detection)
  - Support all media formats (CBZ, CBR, ZIP, RAR, EPUB, PDF)

- [ ] **2. AnalyzeBook** (`task_worker.rs:200-221`)
  - Use extractors to analyze book media
  - Store page count, page sizes in database
  - Detect media type and encoding

- [ ] **3. GenerateBookThumbnail** (`task_worker.rs:223-243`)
  - Generate and **save** thumbnail to disk/Redis
  - Store thumbnail reference in database
  - Handle different aspect ratios

- [ ] **4. HashBook** (`task_worker.rs:245-265`)
  - Calculate and store file hash (SHA-256)
  - Update book record with hash

- [ ] **5. HashBookPages**
  - Hash each page for deduplication
  - Store page hashes in database

- [ ] **6. HashBookKoreader**
  - Generate Koreader-compatible hash manifest

- [ ] **7. RefreshBookMetadata**
  - Read metadata from book file
  - Update book metadata fields in database

- [ ] **8. RefreshSeriesMetadata** (`task_worker.rs:282-296`)
  - Actually update series metadata in database
  - Connect Mylar provider to database

- [ ] **9. AggregateSeriesMetadata**
  - Combine metadata from multiple books
  - Calculate series-level metadata

- [ ] **10. RefreshBookLocalArtwork** (`task_worker.rs:303-317`)
  - Save found cover to database
  - Connect LocalArtwork to BookRepository

- [ ] **11. RefreshSeriesLocalArtwork** (`task_worker.rs:319-333`)
  - Save found series covers
  - Update series record

- [ ] **12. EmptyTrash** (`task_worker.rs:340-343`)
  - Delete books marked as deleted from database

- [ ] **13. DeleteBook** (`task_worker.rs:345-348`)
  - Hard delete from database
  - Remove associated files/thumbnails

- [ ] **14. DeleteSeries** (`task_worker.rs:350-353`)
  - Delete series and all associated books

- [ ] **15. RebuildIndex** (`task_worker.rs:355-358`)
  - Rebuild Tantivy search index from database

- [ ] **16. UpgradeIndex** (`task_worker.rs:360-363`)
  - Handle Tantivy index upgrades

---

# Komga Rust - Sprint 8: Metadata & API Keys

## Goal: Add ComicRack metadata and API key authentication

### Tasks

- [ ] **1. ComicRack XML Sidecar Metadata**
  - Parse ComicInfo.xml files
  - Load metadata from `.cbz/ComicInfo.xml`
  - Load metadata from sidecar files

- [ ] **2. Book Metadata DTOs**
  - Add metadata fields to BookDto
  - Create BookMetadata entity

- [ ] **3. Metadata Endpoints**
  - `PATCH /api/v1/books/{id}/metadata`
  - `GET /api/v1/books/{id}/metadata`

- [ ] **4. Series Metadata Endpoints**
  - `PATCH /api/v1/series/{id}/metadata`
  - `GET /api/v1/series/{id}/metadata`

- [ ] **5. API Key Authentication**
  - Create ApiKey model
  - Implement API key validation middleware
  - Support API key in Authorization header

- [ ] **6. User Settings**
  - Endpoint to update user preferences
  - Password change endpoint

---

# Komga Rust - Sprint 9: OPDS & Performance

## Goal: Add OPDS feed support and optimize performance

### Tasks

- [ ] **1. OPDS Catalog Feed**
  - Implement OPDS 1.2 spec
  - Navigation feed for libraries/series
  - Acquisition feed for books

- [ ] **2. OPDS Endpoints**
  - `/opds/Libraries`
  - `/opds/library/{id}`
  - `/opds/series/{id}`
  - `/opds/book/{id}/download`

- [ ] **3. Caching Optimization**
  - Redis caching for series/book lists
  - Cache invalidation strategy
  - ETag support

- [ ] **4. Pagination**
  - Implement proper cursor-based pagination
  - Optimize large result sets

- [ ] **5. Connection Pooling**
  - SQLx connection pool tuning
  - Redis connection pool

---

## Summary

| Sprint | Focus | Key Items |
|--------|-------|-----------|
| 6 | Page Streaming | Fix CBZ/EPUB/PDF extraction, Thumbnail serving |
| 7 | Task Worker | Implement all 26 task types fully |
| 8 | Metadata & API Keys | ComicRack XML, API key auth |
| 9 | OPDS & Performance | OPDS feeds, caching, pagination |