# Komga Backend Performance - ScanLibrary

## Muc tieu

Tai lieu nay tap trung rieng vao `ScanLibrary` de tra loi 3 cau hoi:
- `ScanLibrary` dang cham o dau?
- Nen toi uu theo thu tu nao?
- Can do gi de xac nhan bottleneck tren du lieu that?

Cap nhat: 2026-04-16

---

## 1) Ket luan nhanh

Sau cac thay doi da implement, `ScanLibrary` da duoc cai thien o 3 diem:

1. Bo compare O(n^2) trong doi chieu sach theo URL.
2. Cat N+1 query theo tung series bang preload series va batch load books.
3. Cat N+1 lookup theo tung sidecar bang preload map series/book theo URL.
4. Dua phan verify hash truong hop `same size + existing hash` ra khoi hot path bang task async `VerifyBookHash`.

Hien tai, `ScanLibrary` con cham chu yeu o 3 nhom viec:

1. Filesystem walk chay tuan tu.
2. Van con mot so DB work va reconciliation cost trong scan, du N+1 lon nhat da duoc cat.
3. Van co nhung truong hop mark `OUTDATED` va update metadata ngay trong scan.
4. Sau scan con fan-out them nhieu task phu tro, nen nguoi van hanh thuong thay "scan cham" trong khi mot phan thoi gian that ra dang nam o pipeline sau scan.

Neu chi duoc toi uu mot it viec, thu tu hop ly nhat la:

1. Bo sung metrics theo phase de do lai sau refactor vua lam.
2. Danh gia va giam phan filesystem walk tuan tu.
3. Xem xet tiep cac update metadata/media co the day sang async neu can.
4. Sau do moi can nhac parallel filesystem walk.

---

## 2) Luong `ScanLibrary` hien tai

Task flow:
1. `Task.ScanLibrary` duoc worker lay tu queue.
2. `TaskHandler` goi `libraryContentLifecycle.scanRootFolder(library, scanDeep)`.
3. `scanRootFolder()`:
   - walk filesystem
   - build `scannedSeries` va `sidecars`
   - soft delete series/books khong con ton tai
   - reconcile series moi / series cu
   - compare books trong moi series
   - danh dau `Media.Status.OUTDATED` neu file doi
   - refresh sort / metadata / artwork task sau scan
4. Sau khi `scanRootFolder()` xong, `TaskHandler` moi fan-out:
   - analyze unknown/outdated books
   - repair extensions
   - find convert candidates
   - find missing page hashes
   - hash books
   - verify hash cho mot nhom book da doi `modified time` nhung giu nguyen `fileSize`

He qua quan trong:
- Neu `ScanLibrary` chua xong, `AnalyzeBook` hang loat chua duoc enqueue.
- Vi vay production co the co rat nhieu book `UNKNOWN/OUTDATED`, nhung tasks DB van chua co `AnalyzeBook`.

---

## 3) Diem nghen da xac nhan

### 3.1 Filesystem walk dang chay 1 luong

`FileSystemScanner.scanRootFolder()` dung `Files.walkFileTree(...)` va `FileVisitor` tuan tu.

Tac dong:
- Neu storage la NAS/SMB/NFS, latency filesystem co the chiem da so wall-clock time.
- Cang nhieu folder/file, cang ro bottleneck nay.

Danh gia:
- Day la bottleneck nen, nhung khong phai luc nao cung la bottleneck dau tien can sua trong code.
- Neu DB round-trip va hash sync van con, parallel walk co the khong cho loi ich xung dang voi do phuc tap.

### 3.2 N+1 query theo tung series

Trong `LibraryContentLifecycle.scanRootFolder()`:
- Moi scanned series lai goi `seriesRepository.findNotDeletedByLibraryIdAndUrlOrNull(...)`.
- Neu series ton tai va can xu ly tiep, lai goi `bookRepository.findAllBySeriesId(...)`.

Tac dong:
- Library lon voi nhieu series se tao ra rat nhieu query nho.
- DB latency va object mapping tang dang ke.

Day la bottleneck quan trong nhat phia application/database trong scan hien tai.

Trang thai:
- Da giam. Hien tai series duoc preload theo library va books duoc batch load theo `findAllBySeriesIds(...)`.
- Vong lap scan khong con query series/book theo tung series nhu truoc.

### 3.3 Hash duoc tinh ngay trong scan hot path

Khi thay `fileLastModified` thay doi, code co the goi `hasher.computeHash(newBook.path)` de xac nhan file co thuc su thay doi noi dung hay khong.

Tac dong:
- Bien scan metadata thanh scan co doc noi dung file.
- Tren file lon hoac storage cham, chi phi rat cao.
- Neu co nhieu file bi cham `modified time`, scan co the rat lau du queue task khac khong lon.

Ket luan:
- Day la noi rat dang de tach ra thanh asynchronous verification hoac deep verification mode.

Trang thai:
- Da giam mot phan. Truong hop `same file size + existing hash` khong con `computeHash(...)` ngay trong scan.
- Scan gio enqueue task `VerifyBookHash`, task nay moi goi `BookLifecycle.verifyHashAndPersist(...)` de xac nhan hash sau.
- Truong hop file size doi van mark `OUTDATED` ngay trong scan nhu cu.

### 3.4 Sidecar reconciliation co dau hieu N+1

Scan hien tai:
- load `existingSidecars = sidecarRepository.findAll()`
- voi moi scanned sidecar lai lookup series/book theo parent URL

Tac dong:
- De load rong hon muc can thiet.
- Query lookup lap lai tren moi sidecar.
- Library co nhieu artwork/metadata sidecar se ton them round-trip va memory.

Trang thai:
- Da giam mot phan. Sidecar parent lookup gio dung `reconciledSeriesByUrl` va `reconciledBooksByUrl` thay vi query tung row.
- `existingSidecars` cung da duoc loc theo library hien tai truoc khi reconcile.

### 3.5 Sorting va refresh sau scan mo rong tong thoi gian pipeline

Sau khi reconcile xong, moi series thay doi lai bi:
- `seriesLifecycle.sortBooks(it)`
- `taskEmitter.refreshSeriesMetadata(it.id)`

Tac dong:
- Nguoi van hanh nhin toan canh se cam thay "scan cham" du mot phan chi phi thuc te nam o post-scan work.

---

## 4) Diem nghen da giam duoc

Nhung viec da lam xong trong code:

1. Compare sach theo URL trong series da duoc doi tu O(n^2) sang lookup bang map.
2. Series duoc preload theo library va books duoc batch load theo `findAllBySeriesIds(...)`.
3. Sidecar parent lookup da chuyen sang map preload thay vi query theo tung sidecar.
4. Verify hash cho truong hop `same size + existing hash` da duoc doi sang async task `VerifyBookHash`.

Y nghia:
- Da cat duoc phan lon nhat cua bottleneck application-side trong `LibraryContentLifecycle`.
- Wall-clock cua `ScanLibrary` se giam ro nhat trong workload co nhieu series, nhieu sidecar, va nhieu file bi doi `mtime` nhung giu nguyen size.
- Nhung `ScanLibrary` van co the cham neu filesystem walk la bottleneck chinh tren storage that.

---

## 5) Thu tu toi uu de xuat

## Phase 1 - Cat round-trip DB lon nhat

Muc tieu:
- Giam N+1 query trong scan.

Can lam:
1. Preload toan bo series cua library thanh map `url -> series`.
2. Xac dinh tap `existingSeriesIds` can xu ly.
3. Batch load books cho nhieu series bang `findAllBySeriesIds(...)`.
4. Tao map:
   - `seriesByUrl`
   - `booksBySeriesId`
   - neu can, `bookByUrlWithinSeries`

Loi ich:
- Cat rat nhieu query nho.
- Giu logic nghiep vu gan nhu khong doi.

Rui ro:
- Can can doi memory neu library qua lon, nhung van hop ly hon N+1 cho phan lon workload production.

Trang thai:
- Hoan thanh.
- Da preload series theo library, batch load books cho cac series duoc scan, va dung map lookup trong vong lap reconcile.

## Phase 2 - Dua hash ra khoi hot path

Muc tieu:
- Scan metadata khong phai doc noi dung file ngay lap tuc.

Huong de xuat:
1. Khi `fileLastModified` hoac `fileSize` thay doi, update sach thanh state can verify.
2. Day mot task rieng de compute hash sau scan.
3. Chi khi hash xac nhan file thuc su doi thi moi danh dau `OUTDATED` hoac reset metadata phu hop.

Loi ich:
- Giam wall-clock time cua `ScanLibrary`.
- Giam I/O sync tren storage cham.

Trade-off:
- Tinh chinh xac khong mat, nhung xac nhan doi noi dung se bi doi thanh asynchronous.

Trang thai:
- Hoan thanh mot phan co chu dich.
- Da tach verify hash async cho truong hop `same size + existing hash`.
- Chua doi hanh vi cho truong hop `fileSize` thay doi, vi truong hop nay da la tin hieu manh va nen mark `OUTDATED` ngay.

## Phase 3 - Toi uu sidecar path

Muc tieu:
- Khong query va scan sidecar rong hon muc can thiet.

Huong de xuat:
1. Chi load sidecar cua library dang scan, khong load toan bo neu repository hien tai cho phep.
2. Preload map series/book theo URL de lookup sidecar parent khong query tung lan.
3. Neu so luong sidecar lon, can nhac split phase sidecar refresh thanh task rieng.

Trang thai:
- Hoan thanh mot phan.
- Parent lookup da dung map thay vi query tung sidecar.
- Chua co repository method rieng de chi load sidecar theo library tu DB; hien tai van `findAll()` roi filter theo library trong memory.

## Phase 4 - Parallel filesystem walk neu benchmark van cho thay can thiet

Chi nen lam khi:
- Da cat N+1 query.
- Da dua hash ra khoi scan hot path.
- Benchmark van cho thay phan lon thoi gian nam o filesystem walk.

Huong de xuat:
- Parallelize theo subtree hoac top-level directories.
- Giu merge result o cuoi de tranh pha vo behavior hien tai.

Rui ro:
- Phuc tap hon nhieu.
- Kho debug hon.
- Co the tang peak memory va random I/O.

Trang thai:
- Chua lam.
- Day la buoc can profile lai sau cac refactor vua implement.

---

## 5.1 Viec da hoan thanh

1. Refactor compare books theo URL sang map lookup.
2. Batch preload series va books trong `scanRootFolder()`.
3. Batch map sidecar parent lookup trong `scanRootFolder()`.
4. Them task `VerifyBookHash` va `BookLifecycle.verifyHashAndPersist()` de doi verify hash sang async.
5. Da chay va pass test `LibraryContentLifecycleTest` va `BookLifecycleTest` sau cac thay doi.

## 5.2 Viec can lam tiep

1. Them metrics/log timing theo phase cho `ScanLibrary` de do lai sau refactor.
2. Danh gia xem filesystem walk dang chiem bao nhieu % wall-clock tren du lieu production.
3. Can nhac them repository method de load `Sidecar` theo `libraryId` thay vi `findAll()`.
4. Neu metrics cho thay can thiet, xem xet parallel filesystem walk.
5. Neu post-scan queue van la bottleneck, tiep tuc toi uu queue/index/task fan-out.

---

## 6) Phan can do de xac nhan bottleneck

De tranh toi uu mu, can tach metric theo phase:

1. `filesystem_walk_ms`
2. `soft_delete_series_ms`
3. `soft_delete_books_ms`
4. `reconcile_series_ms`
5. `book_compare_ms`
6. `hash_verification_ms`
7. `sidecar_reconcile_ms`
8. `sort_and_refresh_ms`

Them counter de do:
- so series scanned
- so books scanned
- so series moi
- so books moi
- so books co `fileLastModified` thay doi
- so task `VerifyBookHash` duoc enqueue
- so lan `VerifyBookHash` that su compute hash
- tong bytes/doc tu hash step neu co the
- so sidecar scanned

Neu storage la NAS:
- can them latency filesystem va disk/network throughput.

---

## 7) Giai thich hien tuong production hay gap

Truong hop hay gap:
- production bao rat nhieu book "To be analyzed"
- nhung tasks DB khong thay nhieu `AnalyzeBook`

Giai thich hop ly:
1. UI hien label theo `Media.Status.UNKNOWN/OUTDATED`, khong theo so row trong TASK.
2. `AnalyzeBook` hang loat chi duoc enqueue sau khi `ScanLibrary` hoan tat.
3. Neu `ScanLibrary` dang cham hoac dang ket lau, tasks DB co the chi thay `ScanLibrary` owned lau.

Vi vay, de debug production:
- khong chi nhin tasks DB
- phai doi chieu them so luong book `UNKNOWN/OUTDATED` trong main DB
- va tuoi cua `ScanLibrary` tasks dang owned

---

## 8) Checklist dieu tra production

1. Dem `ScanLibrary` dang owned va tuoi cua chung.
2. Dem so book `UNKNOWN` va `OUTDATED` trong main DB.
3. Do thoi gian scan theo phase neu co log hoac metrics.
4. Xac dinh storage local hay NAS.
5. Do so task `VerifyBookHash` duoc tao trong mot lan scan.
6. Do so lan hash verification thuc su xay ra sau scan.
7. Kiem tra so query DB theo series/sidecar trong scan benchmark.

---

## 8.1) Cach lay log production theo `scanId`

Sau khi them phase log, moi lan `scanRootFolder()` se co:
- 1 dong `scanRootFolder started ... scanId=...`
- nhieu dong `scanRootFolder phase=... scanId=...`
- 1 dong `scanRootFolder completed status=ok|failed scanId=...`

Vi log format mac dinh da co timestamp, chi can grep theo `scanRootFolder` hoac `scanId`.

Vi du tren production:

```bash
grep 'scanRootFolder started' ~/.komga/logs/komga.log*
```

Lenh nay giup tim cac `scanId` gan day. Khi da co `scanId`, grep lai toan bo phien scan:

```bash
grep 'scanId=abcd1234' ~/.komga/logs/komga.log*
```

Neu chi muon xem breakdown theo phase:

```bash
grep 'scanId=abcd1234' ~/.komga/logs/komga.log* | grep 'phase='
```

Neu chi muon dong tong ket cuoi:

```bash
grep 'scanRootFolder completed' ~/.komga/logs/komga.log*
```

Neu log da rotate va nen qua nhieu file, uu tien copy ra mot file rieng de phan tich:

```bash
grep 'scanId=abcd1234' ~/.komga/logs/komga.log* > scan-abcd1234.log
```

Khi doc log, uu tien 3 truong sau:
- `totalMs` de nhin tong thoi gian scan.
- `filesystemScanMs` de biet filesystem walk co chiem da so thoi gian hay khong.
- cac phase `loadExistingMs`, `deleteMissingBooksMs`, `reconcileSeriesBooksMs`, `sortAndRefreshMs`, `reconcileSidecarsMs` de xac dinh bottleneck con lai nam o DB reconcile hay post-scan work.

---

## 9) Tom tat ra quyet dinh

Neu can mot plan thuc dung, nen lam theo thu tu nay:

1. Do lai `ScanLibrary` theo phase sau cac refactor vua lam.
2. Xac dinh xem filesystem walk hay post-scan queue dang la bottleneck chinh con lai.
3. Toi uu tiep phan `SidecarRepository` neu `findAll()` van ton chi phi dang ke.
4. Sau cung moi can nhac parallel filesystem walk.

Day la thu tu phu hop nhat voi trang thai code hien tai: da cat bot bottleneck application-side, nen buoc tiep theo can dua tren so lieu do lai thay vi tiep tuc refactor mu.