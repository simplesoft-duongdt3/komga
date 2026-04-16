# Komga Backend Performance - Scan Book & Thumbnail

## Muc tieu

Tai lieu nay duoc review lai dua tren code hien tai de:
- Xac nhan dung/sai cua cac nhan dinh trong task pipeline scan/analyze/thumbnail/hash.
- Xep hang bottleneck theo muc do anh huong va rui ro trien khai.
- De xuat solution draft uu tien theo phase, co the implement truc tiep.
- Dinh nghia benchmark truoc/sau de do hieu qua.

Cap nhat: 2026-04-16

---

## 1) Ket luan review nhanh

Sau khi doi chieu voi code, co 2 diem nghen can uu tien cao nhat:

1. `TasksDao.takeFirst()` dang claim task bang 2 buoc `SELECT` roi `UPDATE`, khong atomic.
   - Khi `taskPoolSize >= 2`, worker co the tranh chap task duoi tai.
   - Day la van de dung/sai truoc khi la van de throughput.

2. `LibraryContentLifecycle.scanRootFolder()` co logic doi chieu sach dang O(n^2).
   - `existingBooks.find { ... }` trong vong lap `newBooks`.
   - `existingBooksUrls.contains(...)` tren `List` cung la O(n^2).
   - Day la bottleneck ro nhat tren library lon va deep scan.

Cac nhan dinh khac duoc xac nhan mot phan hoac dung nhung chua phai uu tien dau:
- Index task queue hien tai chua phu hop hot path claim task.
- `taskPoolSize` mac dinh = 1 nen queue gan nhu chay tuan tu.
- `submitTasks()` dang log toan bo collection task, co the gay log I/O lon khi fan-out nhieu task.
- `FileSystemScanner` dang di cay thu muc 1 luong, nhung day chua phai diem toi uu dau tien nen lam.
- Tasks DB hien tai co migration rieng cho SQLite; chua thay migration cho PostgreSQL.

---

## 2) Hien trang kien truc lien quan

Task flow hien tai:
1. `TaskEmitter` tao task va luu vao `TasksRepository`.
2. `TaskProcessor` lang nghe `TaskAddedEvent`, fan-out theo `taskPoolSize`.
3. Moi worker goi `tasksRepository.takeFirst()`.
4. `TaskHandler` xu ly task va xoa khoi queue sau khi chay xong.

Code path chinh:
- `TaskEmitter` -> emit task va batch task.
- `TaskProcessor` -> thread pool chung cho moi loai task.
- `TasksDao` -> query/claim/delete task trong bang `TASK`.
- `LibraryContentLifecycle` -> scan filesystem, doi chieu series/book, tao follow-up tasks.
- `BookLifecycle` / `BookAnalyzer` -> analyze, thumbnail, hash.

Scan fan-out hien tai sau `Task.ScanLibrary`:
- analyze unknown/outdated books
- repair extension
- find books to convert
- find books with missing page hash
- find duplicate pages to delete
- hash books without file hash
- hash books without koreader hash

Nhu vay, scan la cua vao cho rat nhieu task CPU + IO intensive. Neu khau claim queue hoac scan compare khong toi uu, toan bo pipeline se cham.

---

## 3) Root causes da duoc xac nhan

### 3.1 Claim task khong atomic

`TasksDao.takeFirst()` hien tai lam:
- `SELECT ... ORDER BY PRIORITY DESC, LAST_MODIFIED_DATE LIMIT 1`
- deserialize payload
- `UPDATE TASK SET OWNER = ? WHERE ID = ?`

Van de:
- Do la 2 statement tach roi.
- Khi co nhieu worker, du lieu doc duoc o buoc `SELECT` co the khong con hop le den luc `UPDATE`.
- Rui ro lon nhat la duplicate claim hoac group ordering bi tranh chap khi tang `taskPoolSize`.

Tac dong:
- Khong nen tang concurrency truoc khi sua claim path.
- Benchmark voi `taskPoolSize > 1` co the khong dang tin neu claim van race-prone.

### 3.2 Compare book trong scan la O(n^2)

Trong `LibraryContentLifecycle`:
- Moi `newBook` lai `find` trong `existingBooks` theo `url`.
- Sau do lai tao `existingBooksUrls` dang `List` roi `contains()` tren tung `newBook`.

Tac dong:
- Deep scan library lon se ton CPU khong can thiet.
- Cang nhieu sach trong series, chi phi cang tang nhanh.
- Day la bottleneck co kha nang mang lai loi ich lon nhat voi thay doi code nho.

### 3.3 Index queue chua cover hot path

Index hien tai:
- `(OWNER, GROUP_ID)`

Truy van claim task can:
- loc task chua co owner
- loai bo group dang duoc worker khac giu
- `ORDER BY PRIORITY DESC, LAST_MODIFIED_DATE`

Tac dong:
- Index hien tai khong giup phan `ORDER BY`.
- Queue lon se ton them scan/sort trong DB.

### 3.4 Logging batch task qua lon

`TaskEmitter.submitTasks()` dang log:
- `Sending tasks: $tasks`

Tac dong:
- Khi fan-out hang tram/hang nghin task, log payload rat lon.
- Vua ton CPU stringify, vua ton I/O log, vua lam kho doc log that su can thiet.

### 3.5 Global pool qua tho

`taskPoolSize` hien tai ap dung cho tat ca task:
- scan metadata
- thumbnail
- hash
- convert

Tac dong:
- CPU-bound task co the chiem het worker.
- IO/meta task nhe van phai cho.

Day la huong cai tien hop ly, nhung nen lam sau khi sua 2 diem nghen goc o tren.

---

## 4) Solution draft de xuat

## Phase 0 - Baseline va guardrail

Muc tieu:
- Khong toi uu mu.
- Co so lieu truoc/sau cho tung thay doi.

Can lam:
1. Benchmark voi dataset dai dien.
2. Giu `taskPoolSize = 1` trong luc baseline neu claim task chua duoc sua.
3. Thu thap:
   - queue drain time
   - throughput theo book/phut
   - p50/p95 task duration theo type
   - CPU user/system
   - disk throughput / filesystem latency
   - fail rate

Luu y:
- Neu storage la NAS, ghi rieng latency filesystem de tranh danh gia sai bottleneck.
- Khong benchmark tren dataset qua nho.

## Phase 1 - Sua bottleneck lon nhat, rui ro thap

### 4.1 Refactor O(n^2) trong scan thanh O(n)

Phan uu tien cao nhat trong code:
- Tao `existingBooksByUrl` bang `associateBy { it.url }`.
- Tao `existingActiveBookUrls` bang `filter { it.deletedDate == null }.map { it.url }.toHashSet()`.

Huong sua:
- Thay `existingBooks.find { ... }` bang map lookup theo `url`.
- Thay `existingBooksUrls.contains(...)` bang `HashSet.contains(...)`.

Expected outcome:
- Giam ro ret CPU trong scan/deep scan tren series lon.
- De verify va rui ro thap vi khong doi behavior nghiep vu.

Pseudo code:

```kotlin
val existingBooks = bookRepository.findAllBySeriesId(existingSeries.id)
val existingBooksByUrl = existingBooks.filter { it.deletedDate == null }.associateBy { it.url }
val existingActiveBookUrls = existingBooksByUrl.keys

newBooks.forEach { newBook ->
  val existingBook = existingBooksByUrl[newBook.url] ?: return@forEach
  // existing update logic
}

val booksToAdd = newBooks.filterNot { it.url in existingActiveBookUrls }
```

### 4.2 Cat giam logging payload trong batch submit

Sua `submitTasks()` thanh log ngan gon:
- so luong task
- loai task dau tien hoac summary theo simple type

Vi du:

```kotlin
logger.info { "Sending ${tasks.size} tasks" }
```

Neu can them context:

```kotlin
logger.info { "Sending ${tasks.size} tasks, sampleType=${tasks.firstOrNull()?.javaClass?.simpleName}" }
```

Expected outcome:
- Giam log noise va I/O khi scan fan-out lon.

## Phase 2 - Lam queue dung va nhanh hon

### 4.3 Doi `takeFirst()` sang claim mot statement

Muc tieu:
- Khong con kieu `SELECT` roi `UPDATE` tach roi.
- DB tu quyet dinh row nao duoc claim trong cung mot statement.

Vi tasks migration hien tai chi co cho SQLite, solution draft nen toi uu cho SQLite truoc.

Huong de xuat:
- Dung CTE chon candidate task.
- `UPDATE ... WHERE ID IN (candidate)`.
- `RETURNING CLASS, PAYLOAD` de deserialize task vua claim.

Phac thao SQL:

```sql
WITH candidate AS (
  SELECT ID
  FROM TASK
  WHERE OWNER IS NULL
    AND (
      GROUP_ID IS NULL
      OR GROUP_ID NOT IN (
        SELECT GROUP_ID
        FROM TASK
        WHERE OWNER IS NOT NULL
          AND GROUP_ID IS NOT NULL
      )
    )
  ORDER BY PRIORITY DESC, LAST_MODIFIED_DATE
  LIMIT 1
)
UPDATE TASK
SET OWNER = :owner,
    LAST_MODIFIED_DATE = CURRENT_TIMESTAMP
WHERE ID IN (SELECT ID FROM candidate)
RETURNING CLASS, PAYLOAD;
```

Loi ich:
- Giam race window ve mot statement.
- Dung hon khi tang `taskPoolSize`.

Luu y implementation:
- Neu jOOQ render CTE/RETURNING cho SQLite gap han che, co the dung native SQL cho rieng method nay.
- Chi nen tang `taskPoolSize` sau khi claim path nay da xong va benchmark lai.

### 4.4 Bo sung index cho hot path claim

Them migration moi cho tasks SQLite, vi du:

```sql
CREATE INDEX idx__tasks__owner_priority_modified
  ON TASK (OWNER, PRIORITY DESC, LAST_MODIFIED_DATE);
```

Co the can nhac them index bo tro cho `GROUP_ID` neu profile query cho thay subquery van nong:

```sql
CREATE INDEX idx__tasks__group_id_owner
  ON TASK (GROUP_ID, OWNER);
```

Ghi chu:
- Khong nen sua migration cu; tao migration moi.
- Index nao giu lai cuoi cung phai duoc quyet dinh bang `EXPLAIN QUERY PLAN` tren dataset that.

## Phase 3 - Tuning concurrency sau khi claim da an toan

Chi bat dau phase nay sau khi:
- claim task da atomic
- scan compare da het O(n^2)

Thu nghiem theo buoc:
1. `taskPoolSize = 2`
2. `taskPoolSize = 4`
3. Neu can, thu muc cao hon tren may benchmark rieng

Nguyen tac:
- Moi buoc deu benchmark lai.
- Neu throughput khong tang nhung CPU wait/I/O wait tang, dung lai.

## Phase 4 - Cai tien trung han neu can them throughput

### 4.5 Tach execution lane theo nhom task

Muc tieu:
- CPU-heavy tasks khong chan metadata/light tasks.

Huong de xuat:
- Mot pool nho cho `thumbnail/hash/convert`.
- Mot pool rieng cho metadata/light refresh.
- Routing co the dua tren `Task` type trong `TaskProcessor` hoac lop dieu phoi moi.

Khong nen lam som hon Phase 1/2 vi:
- code phuc tap hon
- kho benchmark hon
- de che mat bottleneck thuc su

### 4.6 Parallel filesystem scan

Chi nen xem xet neu benchmark cho thay sau Phase 1/2, thoi gian van nam nhieu o filesystem walk.

Ly do chua uu tien:
- Do phuc tap cao hon.
- Rui ro dua them nondeterminism vao scan logic.
- Trong code hien tai, queue claim va O(n^2) compare co ROI cao hon ro ret.

### 4.7 Xem xet tasks DB tren PostgreSQL sau cung

Trang thai hien tai:
- Main DB co the la PostgreSQL.
- Tasks DB co migration rieng cho SQLite.
- Chua thay tasks migration cho PostgreSQL.

Ket luan:
- Khong nen lay migration tasks sang PostgreSQL lam buoc dau tien.
- Can sua logic claim va query truoc; neu khong thi doi DB chi chuyen bottleneck, khong giai quyet goc re.

---

## 5) Thu tu implement de xuat

Thu tu toi uu theo impact/risk:

1. Refactor `LibraryContentLifecycle` de bo O(n^2).
2. Rut gon logging trong `TaskEmitter.submitTasks()`.
3. Sua `TasksDao.takeFirst()` thanh atomic claim mot statement.
4. Them migration index cho hot path task queue.
5. Benchmark lai voi `taskPoolSize = 1`, sau do 2, sau do 4.
6. Chi neu can moi lam task-type pool hoac parallel scan.

Ly do sap xep nhu vay:
- Buoc 1 va 2 la low-risk, de do, de rollback.
- Buoc 3 sua tinh dung cua queue khi co concurrency.
- Buoc 4 giup query claim task on dinh hon tren queue lon.
- Buoc 5 moi la luc tuning concurrency co y nghia.

---

## 6) Checklist benchmark truoc/sau

Moi thay doi deu can cung mot dataset va cung workload.

Can do:
- Thoi gian scan xong library
- Queue drain time den khi ve 0
- So task/s theo tung type
- p50/p95 duration cua:
  - ScanLibrary
  - AnalyzeBook
  - RefreshBookLocalArtwork / thumbnail-related task
  - HashBook / HashBookPages
- CPU user/system
- I/O wait hoac disk saturation
- Peak DB time cua query claim task
- So task fail / retry

Neu duoc, bo sung them:
- do dai queue theo thoi gian
- throughput theo phase scan va phase post-scan

Success criteria goi y:
- scan/deep scan giam thoi gian ro ret tren dataset lon
- khong tang fail rate
- khong xuat hien duplicate task processing khi `taskPoolSize > 1`

---

## 7) Non-goals trong dot toi uu dau tien

Khong dua vao scope dau tien:
- rewrite toan bo task scheduler
- dual backend ho tro day du SQLite/PostgreSQL cho tasks ngay lap tuc
- parallel filesystem walk truoc khi co benchmark chung minh no la bottleneck chinh

---

## 8) Tom tat de ra quyet dinh

Neu chi duoc lam mot it viec, nen lam theo thu tu nay:

1. Bo O(n^2) trong `LibraryContentLifecycle`.
2. Sua atomic claim trong `TasksDao.takeFirst()`.
3. Them index cho claim query.
4. Moi bat dau tune `taskPoolSize`.

Day la huong co kha nang mang lai loi ich lon nhat, it rui ro nhat, va tranh viec toi uu sai tang.

