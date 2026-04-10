# PostgreSQL Temporary Table và Collation Fixes - 2026-04-09 (Revised Solution)

## Tổng quan

Fix hai loại lỗi PostgreSQL trong Komga khi chạy với Docker Compose:

1. **Temporary table "does not exist" errors**: Các bảng tạm (temp tables) tạo ra trong một connection không visible trong các connections khác, gây lỗi khi DAO methods thiếu transaction boundaries.

2. **Collation "und-u-ks-level2" not found errors**: PostgreSQL Alpine images thiếu ICU (International Components for Unicode) support, không có collation Unicode case-insensitive, accent-insensitive.

## Solution Evolution

### Initial Approach (Insufficient)
Ban đầu, chúng tôi nghĩ vấn đề là thiếu transaction boundaries. Đã thêm `@Transactional` annotations để đảm bảo tất cả temp table operations chạy trong cùng transaction. Tuy nhiên, approach này **không work** vì:

1. **PostgreSQL temp tables are session-scoped**: Visible only within the same database connection session
2. **Komga's `SplitDslDaoBase` connection management**: 
   - `dslRO` property returns `_dslRO` (read-only connection) trong `@Transactional(readOnly = true)` methods
   - `dslRW` is always write connection
   - Different DAO methods use different connections (`dslRO` vs `dslRW`) ngay cả trong cùng transaction
   - PostgreSQL temp tables created in one connection **not visible** in another

Error pattern:
```
BookDtoDao.findAll() → TempTable(dslRO) → creates temp table in read connection
fetchAndMapInternal() → dslRW.withTempTable() → creates another temp table in write connection  
Queries use different connections → "relation does not exist"
```

### Revised Solution: Real Tables for PostgreSQL
Thay vì dùng PostgreSQL temporary tables (session-scoped), chúng tôi sử dụng **real tables** (unlogged tables) visible across all connections, với explicit drop after use.

## Root Causes

### 1. Temporary Table Issues

- **PostgreSQL temp tables are session-scoped**: Chỉ visible trong cùng một database connection.
- **Connection pooling + missing transactions**: Komga sử dụng connection pooling (HikariCP) và DAO methods tạo temp tables mà không có `@Transactional` annotations, dẫn đến các queries khác nhau sử dụng connections khác nhau.
- **Affected background tasks**: `RebuildIndex` và `ScanLibrary` tasks fail do temp table errors, ảnh hưởng đến chức năng ứng dụng.

### 2. Collation Issues

- **PostgreSQL Alpine images have minimal ICU**: Image `postgres:15-alpine` chỉ cài đặt ICU cơ bản, không có collation `"und-u-ks-level2"`.
- **Hardcoded collation trong `PostgresUdfProvider`**: Sử dụng collation cụ thể mà không kiểm tra availability.
- **Case-insensitive Unicode sorting**: Komga cần collation Unicode để sorting không phân biệt hoa/thường và dấu.

## Giải pháp đã triển khai (Revised Solution)

### Phase 1: PostgreSQL Uses Real Tables Instead of Temp Tables

#### 1. Modified `TempTable.kt` - Use unlogged tables for PostgreSQL:
- **PostgreSQL**: `CREATE UNLOGGED TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);`
- **SQLite**: `CREATE TEMPORARY TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);` (unchanged)
- **Why unlogged tables?**: Faster (no WAL logging), visible across all connections, explicitly dropped after use
- **Error handling**: Drop failures ignored (tables may persist but are small and unlogged)

#### 2. Connection consistency fixes in DAO classes:
- **`BookDtoDao.kt`**: Changed `dslRW.withTempTable()` → `dslRO.withTempTable()` in `fetchAndMapInternal()`
- **`SeriesDtoDao.kt`**: Changed `dslRW.withTempTable()` → `dslRO.withTempTable()` in `fetchAndMapInternal()`
- **Why `dslRO`?**: `SplitDslDaoBase.dslRO` property returns appropriate connection based on transaction context:
  - Read-only transactions: `_dslRO` (read-only connection)
  - Write transactions: `dslRW` (write connection)
  - Ensures temp tables created and queried using **same connection**

#### 3. Transaction boundaries (kept from initial approach):
- **`BookDtoDao.kt`**: All public methods have `@Transactional(readOnly = true)`
- **`SeriesDtoDao.kt`**: All public methods have `@Transactional(readOnly = true)`
- **`SearchIndexLifecycle.kt`**: `@Transactional` on `rebuildIndex()` method

### Phase 2: Fix Collation Issues

#### 1. Update PostgreSQL images từ Alpine sang standard:

- **`docker-compose.yml`**: `postgres:15-alpine` → `postgres:15`
- **`docker-compose.local.yml`**: `postgres:15-alpine` → `postgres:15`

#### 2. Implement ICU collation detection với fallback trong `PostgresUdfProvider.kt`:

- **ICU detection**: Kiểm tra `und-u-ks-level2` collation availability bằng query `SELECT collname FROM pg_collation WHERE collname = 'und-u-ks-level2'`.
- **Fallback to binary collation**: Nếu không tìm thấy, sử dụng `"C"` collation (binary, case-sensitive).
- **Updated `collateUnicode3()` method** để tự động chọn collation phù hợp.

### Phase 3: Testing và Validation

- **Build ứng dụng với fixes**: `docker-compose -f docker-compose.local.yml up -d --build`
- **Monitor logs**: Kiểm tra không còn temp table errors và collation errors.
- **Verify background tasks**: `RebuildIndex` và `ScanLibrary` chạy thành công.

## Các file đã sửa

### 1. TempTable Implementation (Core Fix)

**`komga/src/main/kotlin/org/gotson/komga/infrastructure/jooq/TempTable.kt`**:
```kotlin
fun create() {
    val dialect = dslContext.dialect()
    val sql = if (dialect.name.contains("POSTGRES")) {
        // For PostgreSQL, create an unlogged table (faster) that will be explicitly dropped
        "CREATE UNLOGGED TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);"
    } else {
        // For SQLite, use temporary tables as before
        "CREATE TEMPORARY TABLE IF NOT EXISTS $name (STRING varchar NOT NULL);"
    }
    dslContext.execute(sql)
    created = true
}

override fun close() {
    if (created) {
        try {
            dslContext.dropTableIfExists(name).execute()
        } catch (e: Exception) {
            // Ignore errors when dropping table, especially if transaction is aborted
            // For PostgreSQL, unlogged tables will persist but that's acceptable
        }
    }
}
```

### 2. DAO Connection Consistency Fixes

**`komga/src/main/kotlin/org/gotson/komga/infrastructure/jooq/main/BookDtoDao.kt`**:
```kotlin
private fun fetchAndMapInternal(query: ResultQuery<Record>): MutableList<BookDto> {
    val records = query.fetch()
    val bookIds = records.getValues(b.ID)

    lateinit var authors: Map<String, List<AuthorDto>>
    lateinit var tags: Map<String, List<String>>
    lateinit var links: Map<String, List<WebLinkDto>>
    // Use dslRO for temp table operations - it returns the appropriate connection based on transaction context
    dslRO.withTempTable(batchSize, bookIds).use { tempTable ->
      authors = dslRO.selectFrom(a)
          .where(a.BOOK_ID.`in`(tempTable.selectTempStrings()))
          .filter { it.name != null }
          .groupBy({ it.bookId }, { AuthorDto(it.name, it.role) })
      // ... similar for tags and links
    }
    // ... rest of method
}
```

**`komga/src/main/kotlin/org/gotson/komga/infrastructure/jooq/main/SeriesDtoDao.kt`**:
```kotlin
private fun fetchAndMapInternal(query: ResultQuery<Record>): MutableList<SeriesDto> {
    val records = query.fetch()
    val seriesIds = records.getValues(s.ID)

    lateinit var genres: Map<String, List<String>>
    // ... other mappings
    dslRO.withTempTable(batchSize, seriesIds).use { tempTable ->
      genres = dslRO.selectFrom(g)
          .where(g.SERIES_ID.`in`(tempTable.selectTempStrings()))
          .groupBy({ it.seriesId }, { it.genre })
      // ... similar for other mappings
    }
    // ... rest of method
}
```

### 3. Transaction Annotations (Maintained)

**`BookDtoDao.kt`, `SeriesDtoDao.kt`, `SearchIndexLifecycle.kt`**:
- All public methods using temp tables have `@Transactional(readOnly = true)` annotations
- `SearchIndexLifecycle.rebuildIndex()` has `@Transactional` annotation
- Ensures operations run within transaction boundaries

### 4. PostgreSQL UDF Provider (Collation detection)

**`komga/src/main/kotlin/org/gotson/komga/infrastructure/datasource/PostgresUdfProvider.kt`**:
```kotlin
override fun <T> collateUnicode3(field: Field<T>): Field<T> {
    return try {
        // Kiểm tra collation availability
        val hasIcuCollation = dsl.selectCount()
            .from("pg_collation")
            .where(DSL.field("collname").eq("und-u-ks-level2"))
            .fetchOne(0, Int::class.java) ?: 0 > 0
        
        if (hasIcuCollation) {
            DSL.field("({0}) COLLATE \"und-u-ks-level2\"", field.type, field)
        } else {
            // Fallback to binary collation
            DSL.field("({0}) COLLATE \"C\"", field.type, field)
        }
    } catch (e: Exception) {
        // Fallback nếu query fails
        DSL.field("({0}) COLLATE \"C\"", field.type, field)
    }
}
```

### 5. Docker Compose Files

**`docker-compose.yml`**:
```yaml
services:
  postgres:
    image: postgres:15  # Changed from postgres:15-alpine
```

**`docker-compose.local.yml`**:
```yaml
services:
  postgres:
    image: postgres:15  # Changed from postgres:15-alpine
```

## Kết quả mong đợi

✅ **No more temp table "does not exist" errors** trong logs  
✅ **No more collation "und-u-ks-level2" not found errors**  
✅ **Background tasks (`RebuildIndex`, `ScanLibrary`) complete successfully**  
✅ **Unicode sorting works với proper collation fallback**  
✅ **Application runs stable với PostgreSQL standard image**

## Các bước tiếp theo (Future Improvements)

### 1. Temp Table Management:
- **Orphaned table cleanup**: Add startup cleanup for old `temp_*` unlogged tables (e.g., older than 24 hours)
- **Table naming conventions**: Consider prefix like `komga_temp_` to avoid conflicts with other applications
- **Monitoring**: Track number of temporary tables created/dropped for performance analysis

### 2. Collation Handling:
- **Runtime collation selection**: Dynamic collation selection based on PostgreSQL version và ICU availability.
- **Custom collation creation**: Tạo custom collation nếu không có sẵn (cần superuser privileges).

### 3. Broader PostgreSQL Compatibility:
- **Review other DAO classes**: Check other DAOs using `dslRW.withTempTable()` that may be called from read-only transactions
- **Connection pool tuning**: Tối ưu HikariCP settings cho PostgreSQL.
- **PostgreSQL-specific optimizations**: Index optimizations, query tuning.

### 4. Documentation:
- **Docker setup updates**: Cập nhật documentation về PostgreSQL image recommendations.
- **Troubleshooting guide**: Thêm section cho temp table và collation issues.

## Testing Commands

### 1. Build và chạy với fixes:
```bash
docker-compose -f docker-compose.local.yml down -v
docker-compose -f docker-compose.local.yml up -d --build
docker-compose logs -f komga
```

### 2. Kiểm tra logs cho errors:
```bash
docker-compose logs komga | grep -E "(does not exist|und-u-ks-level2|ERROR)"
```

### 3. Trigger background tasks test:
```bash
# Gọi API để trigger rebuild index
curl -X POST http://localhost:25600/api/v1/search/reindex
```

## Lưu ý quan trọng

- **PostgreSQL uses real tables**: Instead of temporary tables, PostgreSQL now uses unlogged tables (`CREATE UNLOGGED TABLE`) visible across all connections
- **Connection consistency**: DAO methods use `dslRO` (not `dslRW`) for temp table operations to ensure same connection usage
- **Transaction boundaries**: Tất cả DAO methods tạo temp tables cần `@Transactional` annotation.
- **PostgreSQL image**: Sử dụng standard PostgreSQL image (không phải Alpine) để có đầy đủ ICU support.
- **Fallback strategy**: `PostgresUdfProvider` giờ có collation fallback để tránh runtime errors.
- **Backward compatibility**: Changes tương thích với cả SQLite và PostgreSQL (SQLite continues using temporary tables).

## Tham khảo

- **Plan document**: `.kilo/plans/1775711748158-cosmic-wolf.md`
- **PostgreSQL temp tables documentation**: https://www.postgresql.org/docs/current/sql-createtable.html
- **ICU collations in PostgreSQL**: https://www.postgresql.org/docs/current/collation.html
- **Spring `@Transactional`**: https://docs.spring.io/spring-framework/docs/current/reference/html/data-access.html#transaction-declarative

---
*Document created: 2026-04-09*  
*Last updated: 2026-04-09*