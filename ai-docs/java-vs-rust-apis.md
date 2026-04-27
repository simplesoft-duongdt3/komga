Now I have enough information to provide a comprehensive comparison. Here is the summary:

---

## Comprehensive Java vs Rust API Comparison for Komga

### 1. JAVA API ENDPOINTS (Kotlin/Spring Boot)

#### **BookController** (`api/v1/books`)
| Method | Endpoint | Description | Repositories/Services Used |
|--------|----------|-------------|---------------------------|
| GET | `/api/v1/books` | List books (deprecated) | `BookDtoRepository.findAll()` |
| POST | `/api/v1/books/list` | List books with search | `BookDtoRepository.findAll()` |
| GET | `/api/v1/books/latest` | Latest books | `BookDtoRepository.findAll()` |
| GET | `/api/v1/books/ondeck` | On-deck books | `BookDtoRepository.findAllOnDeck()` |
| GET | `/api/v1/books/duplicates` | Duplicate books (ADMIN) | `BookDtoRepository.findAllDuplicates()` |
| GET | `/api/v1/books/{bookId}` | Get book by ID | `BookDtoRepository.findByIdOrNull()` |
| GET | `/api/v1/books/{bookId}/previous` | Previous book in series | `BookDtoRepository.findPreviousInSeriesOrNull()` |
| GET | `/api/v1/books/{bookId}/next` | Next book in series | `BookDtoRepository.findNextInSeriesOrNull()` |
| GET | `/api/v1/books/{bookId}/readlists` | Book's readlists | `ReadListRepository.findAllContainingBookId()` |
| GET | `/api/v1/books/{bookId}/thumbnail` | Book poster | `BookLifecycle.getThumbnailBytes()` |
| GET | `/api/v1/books/{bookId}/thumbnails/{thumbnailId}` | Specific poster | `BookLifecycle.getThumbnailBytesByThumbnailId()` |
| GET | `/api/v1/books/{bookId}/thumbnails` | List posters | `ThumbnailBookRepository.findAllByBookId()` |
| POST | `/api/v1/books/{bookId}/thumbnails` | Add poster (ADMIN) | `BookLifecycle.addThumbnailForBook()` |
| PUT | `/api/v1/books/{bookId}/thumbnails/{thumbnailId}/selected` | Mark poster selected (ADMIN) | `ThumbnailBookRepository.markSelected()` |
| DELETE | `/api/v1/books/{bookId}/thumbnails/{thumbnailId}` | Delete poster (ADMIN) | `BookLifecycle.deleteThumbnailForBook()` |
| GET | `/api/v1/books/{bookId}/pages` | List book pages | `BookRepository.findById()`, `MediaRepository.findById()` |
| GET | `/api/v1/books/{bookId}/pages/{pageNumber}` | Get page image | `CommonBookController.getBookPageInternal()` |
| GET | `/api/v1/books/{bookId}/pages/{pageNumber}/thumbnail` | Page thumbnail (300px) | `BookLifecycle.getBookPage()` |
| GET | `/api/v1/books/{bookId}/manifest` | WebPub manifest | `WebPubGenerator` |
| GET | `/api/v1/books/{bookId}/positions` | Epub positions | `MediaRepository.findExtensionByIdOrNull()` |
| GET | `/api/v1/books/{bookId}/manifest/epub` | Epub manifest | `CommonBookController.getWebPubManifestEpubInternal()` |
| GET | `/api/v1/books/{bookId}/manifest/pdf` | PDF manifest | `CommonBookController.getWebPubManifestPdfInternal()` |
| GET | `/api/v1/books/{bookId}/manifest/divina` | DiViNa manifest | `CommonBookController.getWebPubManifestDivinaInternal()` |
| POST | `/api/v1/books/{bookId}/analyze` | Analyze book (ADMIN) | `BookRepository.findByIdOrNull()`, `TaskEmitter.analyzeBook()` |
| POST | `/api/v1/books/{bookId}/metadata/refresh` | Refresh metadata (ADMIN) | `BookRepository.findByIdOrNull()`, `TaskEmitter.refreshBookMetadata()` |
| PATCH | `/api/v1/books/{bookId}/metadata` | Update metadata (ADMIN) | `BookMetadataRepository.findByIdOrNull()`, `BookMetadataRepository.update()` |
| PATCH | `/api/v1/books/metadata` | Bulk update metadata (ADMIN) | `BookMetadataRepository.findByIdOrNull()`, `BookMetadataRepository.update()` |
| PATCH | `/api/v1/books/{bookId}/read-progress` | Mark read progress | `BookRepository.findByIdOrNull()`, `BookLifecycle.markReadProgress()` |
| DELETE | `/api/v1/books/{bookId}/read-progress` | Mark unread | `BookRepository.findByIdOrNull()`, `BookLifecycle.deleteReadProgress()` |
| POST | `/api/v1/books/import` | Import books (ADMIN) | `TaskEmitter.importBook()` |
| DELETE | `/api/v1/books/{bookId}/file` | Delete book file (ADMIN) | `TaskEmitter.deleteBook()` |
| PUT | `/api/v1/books/thumbnails` | Regenerate posters (ADMIN) | `TaskEmitter.findBookThumbnailsToRegenerate()` |

#### **SeriesController** (`api/v1/series`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/series` | List series (deprecated) |
| POST | `/api/v1/series/list` | List series with search |
| GET | `/api/v1/series/alphabetical-groups` | Alphabetical groups (deprecated) |
| POST | `/api/v1/series/list/alphabetical-groups` | Alphabetical groups |
| GET | `/api/v1/series/latest` | Latest series |
| GET | `/api/v1/series/new` | New series |
| GET | `/api/v1/series/updated` | Updated series |
| GET | `/api/v1/series/{seriesId}` | Get series by ID |
| GET | `/api/v1/series/{seriesId}/thumbnail` | Series poster |
| GET | `/api/v1/series/{seriesId}/thumbnails/{thumbnailId}` | Specific poster |
| GET | `/api/v1/series/{seriesId}/thumbnails` | List posters |
| POST | `/api/v1/series/{seriesId}/thumbnails` | Add poster (ADMIN) |
| PUT | `/api/v1/series/{seriesId}/thumbnails/{thumbnailId}/selected` | Mark selected (ADMIN) |
| DELETE | `/api/v1/series/{seriesId}/thumbnails/{thumbnailId}` | Delete poster (ADMIN) |
| GET | `/api/v1/series/{seriesId}/books` | Series' books (deprecated) |
| GET | `/api/v1/series/{seriesId}/collections` | Series' collections |
| POST | `/api/v1/series/{seriesId}/analyze` | Analyze series (ADMIN) |
| POST | `/api/v1/series/{seriesId}/metadata/refresh` | Refresh metadata (ADMIN) |
| PATCH | `/api/v1/series/{seriesId}/metadata` | Update metadata (ADMIN) |
| POST | `/api/v1/series/{seriesId}/read-progress` | Mark series as read |
| DELETE | `/api/v1/series/{seriesId}/read-progress` | Mark series as unread |
| GET | `/api/v2/series/{seriesId}/read-progress/tachiyomi` | Mihon progress |
| PUT | `/api/v2/series/{seriesId}/read-progress/tachiyomi` | Update Mihon progress |
| GET | `/api/v1/series/{seriesId}/file` | Download series as ZIP |
| DELETE | `/api/v1/series/{seriesId}/file` | Delete series files (ADMIN) |

#### **LibraryController** (`api/v1/libraries`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/libraries` | List all libraries |
| GET | `/api/v1/libraries/{libraryId}` | Get library by ID |
| POST | `/api/v1/libraries` | Create library (ADMIN) |
| PATCH | `/api/v1/libraries/{libraryId}` | Update library (ADMIN) |
| DELETE | `/api/v1/libraries/{libraryId}` | Delete library (ADMIN) |
| POST | `/api/v1/libraries/{libraryId}/scan` | Scan library (ADMIN) |
| POST | `/api/v1/libraries/{libraryId}/analyze` | Analyze library (ADMIN) |
| POST | `/api/v1/libraries/{libraryId}/metadata/refresh` | Refresh metadata (ADMIN) |
| POST | `/api/v1/libraries/{libraryId}/empty-trash` | Empty trash (ADMIN) |

#### **ReadListController** (`api/v1/readlists`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/readlists` | List readlists |
| GET | `/api/v1/readlists/{id}` | Get readlist by ID |
| GET/POST/PUT/DELETE | `/api/v1/readlists/{id}/thumbnails*` | Thumbnail management |
| POST | `/api/v1/readlists` | Create readlist (ADMIN) |
| POST | `/api/v1/readlists/match/comicrack` | Match ComicRack list (ADMIN) |
| PATCH | `/api/v1/readlists/{id}` | Update readlist (ADMIN) |
| DELETE | `/api/v1/readlists/{id}` | Delete readlist (ADMIN) |
| GET | `/api/v1/readlists/{id}/books` | Readlist's books |
| GET | `/api/v1/readlists/{id}/books/{bookId}/previous` | Previous in readlist |
| GET | `/api/v1/readlists/{id}/books/{bookId}/next` | Next in readlist |
| GET/PUT | `/api/v1/readlists/{id}/read-progress/tachiyomi` | Mihon progress |
| GET | `/api/v1/readlists/{id}/file` | Download as ZIP |

#### **SeriesCollectionController** (`api/v1/collections`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/collections` | List collections |
| GET | `/api/v1/collections/{id}` | Get collection by ID |
| GET/POST/PUT/DELETE | `/api/v1/collections/{id}/thumbnails*` | Thumbnail management |
| POST | `/api/v1/collections` | Create collection (ADMIN) |
| PATCH | `/api/v1/collections/{id}` | Update collection (ADMIN) |
| DELETE | `/api/v1/collections/{id}` | Delete collection (ADMIN) |
| GET | `/api/v1/collections/{id}/series` | Collection's series |

#### **UserController** (`api/v2/users`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v2/users/me` | Current user |
| PATCH | `/api/v2/users/me/password` | Update own password |
| GET | `/api/v2/users` | List users (ADMIN) |
| POST | `/api/v2/users` | Create user (ADMIN) |
| DELETE | `/api/v2/users/{id}` | Delete user (ADMIN) |
| PATCH | `/api/v2/users/{id}` | Update user (ADMIN) |
| PATCH | `/api/v2/users/{id}/password` | Update user password |
| GET | `/api/v2/users/me/authentication-activity` | Auth activity (current user) |
| GET | `/api/v2/users/authentication-activity` | Auth activity (ADMIN) |
| GET | `/api/v2/users/{id}/authentication-activity/latest` | Latest auth activity |
| GET/POST/DELETE | `/api/v2/users/me/api-keys*` | API key management |

#### **Other Controllers**
| Controller | Endpoints |
|------------|-----------|
| **SettingsController** | `GET/PATCH /api/v1/settings` |
| **FileSystemController** | `POST /api/v1/filesystem` (directory listing) |
| **TaskController** | `GET/DELETE /api/v1/tasks` |
| **ClaimController** | `GET/POST /api/v1/claim` |
| **ReferentialController** | `GET /api/v1/authors`, `GET /api/v2/authors`, `GET /api/v1/genres`, `GET /api/v1/tags`, `GET /api/v1/languages`, `GET /api/v1/publishers`, `GET /api/v1/age-ratings`, `GET /api/v1/sharing-labels`, `GET /api/v1/series/release-dates` |
| **ClientSettingsController** | `GET/PATCH/DELETE /api/v1/client-settings/global/*`, `GET/PATCH/DELETE /api/v1/client-settings/user/*` |
| **HistoricalEventController** | `GET /api/v1/history` (ADMIN) |
| **AnnouncementController** | `GET/PUT /api/v1/announcements` (ADMIN) |
| **SyncPointController** | `DELETE /api/v1/syncpoints/me` |
| **LoginController** | `GET /api/v1/login/set-cookie` |
| **OAuth2Controller** | OAuth2 endpoints |
| **PageHashController** | `GET /api/v1/books/{bookId}/pages/{pageNumber}/hash` |
| **TransientBooksController** | Transient book management |
| **FontsController** | Font serving |
| **ReleaseController** | Release info |
| **KoboController** | Kobo integration |
| **KoreaderSyncController** | KOReader sync |
| **OPDS Controllers** | OPDS v1/v2 feeds |

---

### 2. RUST IMPLEMENTATION STATUS (`komga-rust/src/api/`)

#### **Currently Implemented**
| Module | Routes | Status |
|--------|--------|--------|
| **library.rs** | GET/POST `/api/v1/libraries`, GET/DELETE `/api/v1/libraries/{id}`, POST `/api/v1/libraries/{id}/scan` | Partial |
| **series.rs** | GET `/api/v1/libraries/{libraryId}/series`, GET `/api/v1/series/{id}`, GET/PUT/DELETE `/api/v1/series/{id}/cover`, PATCH `/api/v1/series/{id}/metadata` | Partial |
| **book.rs** | GET `/api/v1/series/{seriesId}/books`, GET `/api/v1/books/{id}`, GET `/api/v1/books/{bookId}/pages`, GET `/api/v1/books/{bookId}/pages/{pageNumber}`, GET `/api/v1/books/{bookId}/pages/{pageNumber}/thumbnail`, GET/PATCH/DELETE `/api/v1/books/{bookId}/read-progress`, GET/PUT/DELETE `/api/v1/books/{id}/cover`, PATCH `/api/v1/books/{id}/metadata` | Partial |
| **readlist.rs** | GET/POST `/api/v1/readlists`, GET/PATCH/DELETE `/api/v1/readlists/{id}` | Partial |
| **collection.rs** | GET/POST `/api/v1/collections`, GET/PATCH/DELETE `/api/v1/collections/{id}` | Partial |
| **search.rs** | GET `/api/v1/search` | Stub (returns empty) |
| **task.rs** | GET `/api/v1/tasks`, GET/DELETE `/api/v1/tasks/{id}` | Partial |
| **auth.rs** | POST `/api/v1/users`, POST `/api/v1/users/login`, GET `/api/v1/users/me`, GET `/api/v1/claim`, GET `/api/v1/client-settings/global/list` | Stub/Placeholder |
| **apikey.rs** | GET/POST/DELETE `/api/v1/api-keys` | Partial |

---

### 3. WHAT'S MISSING IN RUST

#### **Completely Missing Controllers/Features:**
- **UserController** - User CRUD, password management, authentication activity
- **SettingsController** - Server settings GET/PATCH
- **FileSystemController** - Directory listing
- **ReferentialController** - Authors, genres, tags, languages, publishers, age ratings, release dates
- **ClientSettingsController** - Global/user client settings
- **HistoricalEventController** - History events
- **AnnouncementController** - Announcements
- **SyncPointController** - Kobo sync points
- **LoginController** - Session cookie handling
- **OAuth2Controller** - OAuth2 authentication
- **PageHashController** - Page hashing
- **TransientBooksController** - Transient books
- **FontsController** - Font serving
- **ReleaseController** - Release info
- **KoboController** - Full Kobo integration
- **KoreaderSyncController** - KOReader sync
- **OPDS Controllers** - OPDS v1/v2 feed generation

#### **Missing Endpoints in Partially Implemented Controllers:**

**Books:**
- `POST /api/v1/books/list` (search with POST body)
- `GET /api/v1/books/latest`
- `GET /api/v1/books/ondeck`
- `GET /api/v1/books/duplicates`
- `GET /api/v1/books/{bookId}/previous` and `/next`
- `GET /api/v1/books/{bookId}/readlists`
- Thumbnail management (multiple thumbnails per book)
- `GET /api/v1/books/{bookId}/manifest` (WebPub)
- `GET /api/v1/books/{bookId}/positions`
- `POST /api/v1/books/{bookId}/analyze`
- `POST /api/v1/books/{bookId}/metadata/refresh`
- `PATCH /api/v1/books/metadata` (bulk)
- `POST /api/v1/books/import`
- `DELETE /api/v1/books/{bookId}/file`
- `PUT /api/v1/books/thumbnails` (regenerate)

**Series:**
- `POST /api/v1/series/list` (search)
- `POST /api/v1/series/list/alphabetical-groups`
- `GET /api/v1/series/latest`, `/new`, `/updated`
- Thumbnail management (multiple thumbnails)
- `GET /api/v1/series/{seriesId}/collections`
- `POST /api/v1/series/{seriesId}/analyze`
- `POST /api/v1/series/{seriesId}/metadata/refresh`
- `POST/DELETE /api/v1/series/{seriesId}/read-progress`
- Mihon/Tachiyomi endpoints
- `GET /api/v1/series/{seriesId}/file` (download ZIP)
- `DELETE /api/v1/series/{seriesId}/file`

**Libraries:**
- `PATCH /api/v1/libraries/{libraryId}` (update)
- `POST /api/v1/libraries/{libraryId}/analyze`
- `POST /api/v1/libraries/{libraryId}/metadata/refresh`
- `POST /api/v1/libraries/{libraryId}/empty-trash`

**ReadLists:**
- Thumbnail management
- `POST /api/v1/readlists/match/comicrack`
- `GET /api/v1/readlists/{id}/books`
- `GET /api/v1/readlists/{id}/books/{bookId}/previous` and `/next`
- Mihon/Tachiyomi endpoints
- `GET /api/v1/readlists/{id}/file` (download ZIP)

**Collections:**
- Thumbnail management
- `GET /api/v1/collections/{id}/series`

**Tasks:**
- `DELETE /api/v1/tasks` (clear queue - bulk)

---

### 4. KEY DATABASE PATTERNS TO REPLICATE

The Java code uses:
- **jOOQ** for type-safe SQL queries (see `infrastructure/jooq/`)
- **Spring Data JPA** style repositories with interfaces
- **DTO repositories** that join multiple tables for API responses (`BookDtoRepository`, `SeriesDtoRepository`, etc.)
- **SearchCondition/SearchContext** pattern for dynamic query building
- **Pagination** via Spring's `Pageable`/`Page`
- **Content restrictions** based on user permissions (library access, age ratings, sharing labels)

Key tables inferred from repositories:
- `library`, `series`, `book`, `media`, `book_metadata`, `series_metadata`
- `read_progress`, `read_list`, `read_list_book`, `series_collection`, `collection_series`
- `thumbnail_book`, `thumbnail_series`, `thumbnail_readlist`, `thumbnail_collection`
- `komga_user`, `api_key`, `authentication_activity`, `sync_point`
- `historical_event`, `client_settings`, `page_hash`, `referential` (authors, genres, etc.)
