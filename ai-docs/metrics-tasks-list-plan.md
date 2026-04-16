# Metrics Tasks List Plan

## Goal

Bo sung mot section moi tren trang `/settings/metrics` de hien thi danh sach task dang ton tai, co phan trang, sort, va cot duration. Section nay can hien thi ca queued va running tasks.

## Scope

- Hien thi ca queued va running tasks trong cung mot bang.
- Co server-side pagination.
- Co sort cho cac cot backend ho tro thuc te.
- Co cot lifetime duration.
- Hien thi thong tin co ban, khong bao gom filter UI rieng o ban dau.
- Khong bao gom action cancel tung task o ban dau.

## Delivery Plan

1. Backend: bo sung endpoint `GET /api/v1/tasks` trong `TaskController`, giu nguyen `DELETE /api/v1/tasks` hien co.
2. Backend: them read model rieng cho task list thay vi dua tren `TasksRepository.findAll()`.
3. Backend: thiet ke `TaskDto` cho task listing, gom `id`, `simpleType`, `status`, `owner`, `priority`, `groupId`, `createdDate`, `lastModifiedDate`, `duration`.
4. Backend: them query phan trang trong `TasksDao` de doc truc tiep bang `TASK`, ho tro sort co allow-list.
5. Backend: them test cho repository/DAO, controller, security, paging, sort, va status mapping.
6. Frontend: mo rong `komga-tasks.service.ts` bang method `getAll(pageRequest)` theo cung pattern voi history service.
7. Frontend: them type/interface cho `TaskDto`, tai su dung `Page` va `PageRequest` hien co.
8. Frontend: cap nhat `MetricsView.vue` de them section moi dang `v-card` chua `v-data-table` server-side pagination.
9. Frontend: map sort cho cac cot thuc su duoc backend ho tro; disable sort cho `duration` neu day la field computed.
10. Frontend: bo sung i18n keys cho tieu de section, cot bang, va nhan status.
11. Verification: kiem tra layout, empty state, loading state, refresh action, paging, sort, va duration rendering.

## Relevant Files

- `komga/src/main/kotlin/org/gotson/komga/interfaces/api/rest/TaskController.kt`
- `komga/src/main/kotlin/org/gotson/komga/application/tasks/TasksRepository.kt`
- `komga/src/main/kotlin/org/gotson/komga/infrastructure/jooq/tasks/TasksDao.kt`
- `komga/src/main/kotlin/org/gotson/komga/application/tasks/Task.kt`
- `komga-webui/src/views/MetricsView.vue`
- `komga-webui/src/views/HistoryView.vue`
- `komga-webui/src/services/komga-tasks.service.ts`
- `komga-webui/src/services/komga-history.service.ts`
- `komga-webui/src/types/pageable.ts`
- `komga-webui/src/locales/en.json`

## Key Decisions

- Hien thi ca queued va running tasks.
- Dat section moi full-width o cuoi trang metrics de khong pha vo layout hien tai.
- Backend nen tra `duration` dang raw value, frontend se format.
- Ban dau khong expose full payload cua task.
- Ban dau khong auto-refresh theo SSE; uu tien refresh thu cong va reload khi doi pagination/sort.

## Verification Checklist

1. `GET /api/v1/tasks` tra du lieu phan trang dung `totalElements`, `content`, `page`, `size`, `sort`.
2. Status mapping dung: `owner == null` la queued, nguoc lai la running.
3. Metrics page render duoc bang tasks, co paging, sort, refresh.
4. Cot duration hien thi dung khi `owner` null va khi task dang running.
5. Layout van on dinh tren desktop va viewport hep.