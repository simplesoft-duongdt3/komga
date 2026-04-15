# Komga Backend Performance - Scan Book & Thumbnail

## Muc tieu

Tai lieu nay tong hop:
- Hien trang luong xu ly scan/analyze/thumbnail trong backend.
- Cac diem nghen performance quan trong.
- Giai phap toi uu ngan han (khong doi code).
- Giai phap trung han (can thay doi code).
- Checklist benchmark truoc/sau de do hieu qua.

## 1) Kien truc task hien tai

Task flow:
1. `TaskEmitter` day task vao queue (`TasksRepository`).
2. `TaskProcessor` doc queue va chay task bang thread pool.
3. `TaskHandler` xu ly logic tung loai task (scan, analyze, thumbnail, hash...).

File lien quan:
- `komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskEmitter.kt`
- `komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskProcessor.kt`
- `komga/src/main/kotlin/org/gotson/komga/application/tasks/TaskHandler.kt`
- `komga/src/main/kotlin/org/gotson/komga/infrastructure/jooq/tasks/TasksDao.kt`

Luong scan:
- `Task.ScanLibrary` -> `LibraryContentLifecycle.scanRootFolder(...)`
- Sau scan se fan-out them cac task:
  - analyze book
  - generate thumbnail
  - refresh metadata
  - hash file/hash pages
  - conversion/repair...

File lien quan:
- `komga/src/main/kotlin/org/gotson/komga/domain/service/LibraryContentLifecycle.kt`
- `komga/src/main/kotlin/org/gotson/komga/domain/service/FileSystemScanner.kt`
- `komga/src/main/kotlin/org/gotson/komga/domain/service/BookLifecycle.kt`
- `komga/src/main/kotlin/org/gotson/komga/domain/service/BookAnalyzer.kt`

## 2) Diem nghen performance

### 2.1 Task queue bottleneck
- `taskPoolSize` mac dinh = 1 -> xu ly task gan nhu tuan tu.
- `TaskProcessor` goi `hasAvailable()` lap lai -> tang round-trip DB khi queue lon.
- `TasksDao.takeFirst()` dang theo kieu select roi update owner tach roi (khong atomic claim).

### 2.2 Index queue chua toi uu cho truy van hot path
Bang `TASK` hien co index:
- `(OWNER, GROUP_ID)`

Trong khi truy van co:
- dieu kien owner/group
- `ORDER BY PRIORITY DESC, LAST_MODIFIED_DATE`

Can index phu hop hon cho pattern nay.

File migration:
- `komga/src/flyway/resources/tasks/migration/sqlite/V20231013114850__tasks.sql`

### 2.3 Scan va doi chieu DB chi phi cao
- `FileSystemScanner` di cay thu muc theo 1 luong.
- Trong `LibraryContentLifecycle`, co doan loop + tim kiem trong list co nguy co O(n^2) tren dataset lon.

### 2.4 Thumbnail/hash la CPU + IO intensive
- Thumbnail generation: decode + resize anh.
- Hash file/page: doc du lieu lon tren disk/NAS.
- Tang thread qua nhanh co the chuyen bottleneck sang IO wait.

### 2.5 Rui ro cau hinh Postgres chua dong bo cho tasks DB
Script `run-local-with-postgres.sh` hien set `KOMGA_DATABASE_*` cho main DB.
Mac dinh `tasks-db` van tro ve sqlite neu khong set rieng:
- `komga/src/main/resources/application.yml` (`komga.tasks-db.file`)

=> Co the main DB la Postgres nhung task queue van nam o SQLite.

## 3) Giai phap ngan han (khong doi code)

1. Tang `taskPoolSize` theo buoc:
   - 1 -> 2 -> 4 (do sau moi lan)
2. Chay scan/deep scan/regenerate thumbnails vao gio thap diem.
3. Han che bat cung luc cac task nang:
   - `hashPages`
   - `hashFiles`
   - full thumbnail regeneration
4. Neu dung Postgres cho main DB, uu tien chuyen ca tasks DB sang Postgres.

## 4) Giai phap trung han (can doi code)

1. Atomic task claim trong `TasksDao.takeFirst()`
   - Tranh race/contension khi worker cao.
2. Bo sung index cho queue hot path
   - Theo owner/group + priority + last_modified_date.
3. Giam O(n^2) trong scan lifecycle
   - Dung map tra cuu url->entity thay vi tim tuyen tinh trong loop.
4. Tach limiter theo nhom task
   - Nhom CPU-bound (thumbnail/hash) va nhom IO/meta khong tranh chap nhau.
5. Giam logging payload lon
   - Tranh log danh sach task qua dai trong `submitTasks()`.

## 5) KPI benchmark de do truoc/sau

Theo tung profile `taskPoolSize`: 1 / 2 / 4

Can do:
- Queue drain time (thoi gian xuong 0 task)
- Throughput (book/phut)
- p50/p95 task duration theo type
- Ty le task fail
- CPU user/system, IO wait, disk throughput

Metric co san:
- `komga.tasks.execution`
- `komga.tasks.failure`

File:
- `komga/src/main/kotlin/org/gotson/komga/interfaces/scheduler/MetricsPublisherController.kt`

## 6) Ke hoach thuc thi de xuat

### Phase 1 - Baseline
- Chay scan thu vien mau.
- Thu so lieu queue, duration, CPU/IO.

### Phase 2 - Tuning khong doi code
- Tang `taskPoolSize` len 2, do lai.
- Tang len 4, do lai.
- Chon muc toi uu theo thong luong va do on dinh.

### Phase 3 - Refactor uu tien cao
- Atomic claim + index queue.
- Tinh gon loop scan/compare.
- Bo sung limiter theo nhom task nang.

### Phase 4 - Verify
- Re-run benchmark cung dataset.
- Bao cao truoc/sau (throughput, p95, fail rate, CPU/IO).

## 7) Ghi chu van hanh

- Sau moi thay doi, test tren dataset dai dien (khong chi test nho).
- Neu storage la NAS, theo doi do tre filesystem vi day la bottleneck pho bien.
- Uu tien thay doi tung buoc nho de de rollback va de quan sat tac dong.

---

Cap nhat: 2026-04-13

