# Metrics Tasks List Implementation Outline

## Current Status

Da implement xong scope basic cho feature nay:

- backend co `GET /api/v1/tasks`
- backend ho tro pagination
- backend ho tro filter theo `status` va `simpleType`
- frontend service `komga-tasks.service.ts` da goi duoc API moi
- trang `/settings/metrics` da co section bang tasks
- bang tasks co pagination, sort co ban, refresh, filter `status`, filter `simpleType`, va cot duration

## Validation Status

### Passed

- focused backend tests:
  - `./gradlew :komga:test --tests org.gotson.komga.infrastructure.jooq.tasks.TasksDaoTest --tests org.gotson.komga.interfaces.api.rest.TaskControllerTest`
- backend assemble:
  - `./gradlew :komga:assemble -x test`
- frontend lint:
  - `cd /Users/duong/Documents/GitHub/komga/komga-webui && npm run lint`

### Not Fully Confirmed Yet

- frontend production build chua co ket qua thanh cong cuoi cung trong session nay
- command `npm run build` o `komga-webui` bi timeout sau 120s
- log thu duoc chi thay Sass deprecation warnings tu Vuetify, chua thay stack trace loi compile lien quan truc tiep den feature moi

Ket luan hien tai: backend da o muc deployable basic. Frontend thay doi da qua lint, nhung chua co bang chung day du trong session nay rang production build da ket thuc thanh cong.

## Proposed API

Them endpoint moi:

`GET /api/v1/tasks`

Endpoint nay la `ADMIN` only, tra ve danh sach task dang ton tai theo paging va sorting chuan cua Spring.

### Supported Query Behavior

- `page`
- `size`
- `sort`
- `status`
- `simpleType`

Ban dau nen gioi han `sort` vao cac field co san trong bang `TASK`:

- `priority`
- `simpleType`
- `owner`
- `createdDate`
- `lastModifiedDate`
- `id`

Khong nen sort theo `duration` neu `duration` la field computed.

### Implemented Query Behavior

- `status=QUEUED|RUNNING`
- `simpleType=AnalyzeBook&simpleType=HashBookPages`

Neu khong truyen filter, API tra ve tat ca tasks dang ton tai.

## Backend Design

### Read Model

Khong nen dung truc tiep `TasksRepository.findAll()` de phuc vu API nay vi domain `Task` hien tai khong mang theo `owner`, trong khi UI can phan biet queued/running. Thay vao do, can them read model rieng doc truc tiep metadata tu bang `TASK`.

### DTO Shape

Response item de xuat:

```json
{
  "id": "SCAN_LIBRARY_xxx",
  "simpleType": "ScanLibrary",
  "status": "RUNNING",
  "owner": "taskProcessor-1",
  "priority": 4,
  "groupId": null,
  "createdDate": "2026-04-16T10:30:00Z",
  "lastModifiedDate": "2026-04-16T10:31:10Z",
  "durationMillis": 70000
}
```

### Status Mapping

- `owner == null` => `QUEUED`
- `owner != null` => `RUNNING`

### Duration Rule

Ban dau nen tinh `duration = now - createdDate`.

Ly do:

- don gian
- de giai thich voi nguoi dung
- khong can branch logic theo queued/running

### Data Access

`TasksDao` can them query phan trang truc tiep tu bang `TASK`, gom:

- select metadata can thiet
- count query cho `totalElements`
- mapping tu `owner` sang `status`
- allow-list sort fields

Khong can deserialize full `payload` neu UI chi hien thi metadata co ban.

Implementation hien tai dung read model rieng `TaskDtoDao`, khong dua tren `TasksRepository.findAll()`.

## Frontend Design

### Service Layer

Mo rong `komga-webui/src/services/komga-tasks.service.ts` bang method giong pattern cua history service:

- `getAll(pageRequest?: PageRequest): Promise<Page<TaskDto>>`
- dung `qs.stringify(params, { indices: false })`

Implementation hien tai da support them filter object de gui `status` va `simpleType`.

### Types

Them `TaskDto` interface moi trong `komga-webui/src/types`, voi cac field phu hop response backend. Tai su dung `Page<T>` va `PageRequest` hien co.

### Metrics Page UI

Them mot `v-card` full-width vao cuoi `MetricsView.vue`, ben trong la `v-data-table` server-side pagination.

Cot de xuat:

- type
- status
- owner
- priority
- created date
- updated date
- duration

Implementation hien tai da co them 2 control filter tren section nay:

- status filter
- simpleType filter

Pattern implementation nen reuse tu `HistoryView.vue`:

- `options.sync`
- watch `options`
- `server-items-length`
- `loading`
- `footer.prepend` refresh button

### Duration Formatting

Backend tra raw duration. Frontend format ve chuoi de doc, vi du:

- `< 60s` => `45s`
- `< 60m` => `12m 10s`
- lon hon => `2h 03m`

Neu can don gian hon ban dau, co the hien thi theo phut/giay rounded.

### Empty And Loading States

Can xu ly:

- khong co task nao
- dang loading page dau
- refresh lai bang
- owner null

## Security

Task listing nen giu cung scope voi task management hien tai:

- `@PreAuthorize("hasRole('ADMIN')")`

## Tests To Add

### Backend

- pagination tra dung so luong phan tu
- sort theo field hop le
- mapping queued/running theo `owner`
- authorization chi cho `ADMIN`
- empty result
- filter theo `status`
- filter theo `simpleType`

### Frontend

- service gui query params dung format
- `MetricsView` reload khi doi page/size/sort
- table render dung cot va total count
- duration formatting dung
- filter controls trigger reload dung params

## Recommended First Iteration

Ban dau chi can lam:

1. `GET /api/v1/tasks`
2. `TaskDto` metadata only
3. metrics table co paging, sort, refresh
4. `duration` non-sortable

Day la scope nho nhat de co gia tri su dung ngay, va khong mo rong API qua muc can thiet.

## Deployment Readiness Summary

O muc basic:

- backend: san sang
- frontend code path moi: da qua lint
- frontend production artifact: chua confirm xong trong session nay

Neu can chot deploy voi do tin cay cao hon, buoc tiep theo nen la rerun `npm run build` voi timeout dai hon hoac tren CI de lay ket qua cuoi cung ro rang.