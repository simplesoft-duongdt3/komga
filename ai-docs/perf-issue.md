# pg_stat_activity
4991 00:00:00.589249 "WITH candidate AS (
  SELECT "ID"
  FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR "GROUP_ID" NOT IN (
        SELECT "GROUP_ID"
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
          AND "GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
)
UPDATE "TASK"
SET "OWNER" = $1
WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"" active
4527 00:00:00.588639 "WITH candidate AS (
  SELECT "ID"
  FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR "GROUP_ID" NOT IN (
        SELECT "GROUP_ID"
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
          AND "GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
)
UPDATE "TASK"
SET "OWNER" = $1
WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"" active
4171 00:00:00.588209 "WITH candidate AS (
  SELECT "ID"
  FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR "GROUP_ID" NOT IN (
        SELECT "GROUP_ID"
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
          AND "GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
)
UPDATE "TASK"
SET "OWNER" = $1
WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"" active
4857 00:00:00.586031 "WITH candidate AS (
  SELECT "ID"
  FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR "GROUP_ID" NOT IN (
        SELECT "GROUP_ID"
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
          AND "GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
)
UPDATE "TASK"
SET "OWNER" = $1
WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"" active
4989 00:00:00.585161 "WITH candidate AS (
  SELECT "ID"
  FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR "GROUP_ID" NOT IN (
        SELECT "GROUP_ID"
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
          AND "GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
)
UPDATE "TASK"
SET "OWNER" = $1
WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"" active
4964 00:00:00.476251 "WITH candidate AS (
  SELECT "ID"
  FROM "TASK"
  WHERE "OWNER" IS NULL
    AND (
      "GROUP_ID" IS NULL
      OR "GROUP_ID" NOT IN (
        SELECT "GROUP_ID"
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
          AND "GROUP_ID" IS NOT NULL
      )
    )
  ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
  LIMIT 1
)
UPDATE "TASK"
SET "OWNER" = $1
WHERE "ID" = (SELECT "ID" FROM candidate)
RETURNING "CLASS", "PAYLOAD"" active
5057 00:00:00.256561 delete from "TASK" where "TASK"."ID" = $1 active
4699 00:00:00.170051 delete from "TASK" where "TASK"."ID" = $1 active
4471 00:00:00.093412 delete from "TASK" where "TASK"."ID" = $1 active
5032 00:00:00.000811 insert into "TASK" ("ID", "PRIORITY", "GROUP_ID", "CLASS", "SIMPLE_TYPE", "PAYLOAD") values ($1, $2, $3, $4, $5, $6) on conflict ("ID") do update set "GROUP_ID" = $7, "PRIORITY" = $8, "CLASS" = $9, "SIMPLE_TYPE" = $10, "PAYLOAD" = $11, "LAST_MODIFIED_DATE" = cast($12 as timestamp(0)) active
5314 -00:00:00.02547 "SELECT pid, now() - query_start AS duration, query, state
FROM pg_stat_activity
WHERE state != 'idle' ORDER BY duration DESC" active


Dựa trên snapshot `pg_stat_activity` bạn vừa gửi, chúng ta đã tìm thấy **"tâm chấn"** của vấn đề. Đây không còn là dự đoán nữa mà là bằng chứng xác thực về việc hệ thống đang bị nghẽn ở tầng Database Transaction.

### 1. Phân tích "Hiện trường" SQL

Bạn hãy nhìn vào danh sách các PID (4991, 4527, 4171, 4857...). Tất cả đang cùng chạy một câu lệnh `UPDATE "TASK"`. Đây là logic của một **Job Queue** (Hàng đợi công việc).

**Vấn đề cực kỳ nghiêm trọng ở đây là:**

* **Race Condition & Lock Contention:** Có ít nhất 6 thread đang cùng chạy câu lệnh `WITH candidate AS ... UPDATE`. Vì câu lệnh `SELECT` bên trong CTE (`candidate`) không có cơ chế khóa hàng (row locking), các thread này có khả năng rất cao là **cùng chọn ra 1 ID duy nhất** để update.
* **Hậu quả:** Thread đầu tiên sẽ chiếm được Lock của hàng đó, các thread còn lại (4527, 4171...) sẽ rơi vào trạng thái **"Waiting"** (chờ đợi) cho đến khi thread đầu tiên Commit hoặc Rollback. Đây chính là lý do tại sao bạn thấy nhiều câu lệnh "active" nhưng API không phản hồi: **Các kết nối DB đang đứng xếp hàng để đợi nhau.**

### 2. Điểm yếu trong câu Query của Komga

Câu Query này có một đoạn "hủy diệt" hiệu năng khi bảng `TASK` lớn:

```sql
OR "GROUP_ID" NOT IN (
    SELECT "GROUP_ID" FROM "TASK" 
    WHERE "OWNER" IS NOT NULL AND "GROUP_ID" IS NOT NULL
)

```

Việc thực hiện `NOT IN` với một subquery trên cùng một bảng chính là nguyên nhân đẩy chỉ số **`wa` (I/O Wait)** lên cao. Mỗi khi một thread muốn tìm task mới, nó phải quét lại bảng `TASK` để kiểm tra `GROUP_ID`, gây áp lực khủng khiếp lên ổ cứng NAS.

---

### 3. Giải pháp tối ưu "Hardcore" cho Senior Engineer

Nếu bạn có thể can thiệp vào SQL hoặc cấu hình của Komga, đây là các bước để "giải cứu" backend:

#### A. Sử dụng `FOR UPDATE SKIP LOCKED` (Tiêu chuẩn vàng cho Queue)

Nếu câu lệnh được viết lại như sau, hiệu năng sẽ tăng gấp 10-100 lần:

```sql
WITH candidate AS (
  SELECT "ID" FROM "TASK"
  WHERE "OWNER" IS NULL ...
  ORDER BY "PRIORITY" DESC
  LIMIT 1
  FOR UPDATE SKIP LOCKED -- <--- THÊM DÒNG NÀY
)
UPDATE "TASK" SET "OWNER" = $1 WHERE "ID" = (SELECT "ID" FROM candidate)

```

* **SKIP LOCKED:** Giúp các thread không còn tranh giành nhau. Thread nào thấy hàng đã bị khóa sẽ bỏ qua và tìm hàng tiếp theo ngay lập tức. Điều này triệt tiêu hoàn toàn tình trạng "xếp hàng" chờ Lock.

#### B. Kiểm tra Index

Với câu query trên, bạn cần đảm bảo đã có Index cho các cột điều kiện. Hãy chạy lệnh này trên Postgres để kiểm tra:

```sql
CREATE INDEX IF NOT EXISTS idx_task_queue ON "TASK" ("OWNER", "PRIORITY" DESC, "LAST_MODIFIED_DATE") WHERE "OWNER" IS NULL;

```

#### C. Cấu hình lại Postgres trên Docker

Vì bạn đang chạy trên NAS, hãy đảm bảo Postgres có đủ "không gian thở" để xử lý các update liên tục này. Trong `docker-compose.yml` của Postgres, hãy thử điều chỉnh:

```yaml
command: postgres -c shared_buffers=512MB -c effective_cache_size=1GB -c work_mem=16MB -c maintenance_work_mem=128MB

```

---

### 4. Tại sao API lại chết hoàn toàn?

Khi bạn scan thư viện, hàng nghìn task được insert vào bảng `TASK`.

1. **Insert/Update liên tục** làm chỉ số I/O của NAS đạt đỉnh.
2. **Connection Pool bị chiếm dụng:** Các thread API của Spring Boot cũng cần kết nối DB để lấy thông tin. Tuy nhiên, toàn bộ kết nối (thường mặc định là 10) đã bị các thread `UPDATE "TASK"` chiếm giữ và đang đứng đợi Lock.
3. **Hệ quả:** Request từ trình duyệt gửi đến backend sẽ bị treo ở tầng `HikariCP` vì không còn kết nối nào rảnh để phục vụ.