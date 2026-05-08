CREATE TABLE "TASK_EXECUTION"
(
    "ID"                 varchar   NOT NULL PRIMARY KEY,
    "SIMPLE_TYPE"        varchar   NOT NULL,
    "TASK_ID"            varchar   NULL,
    "LIBRARY_ID"         varchar   NULL,
    "SERIES_ID"          varchar   NULL,
    "BOOK_ID"            varchar   NULL,
    "START_DATE"         timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "END_DATE"           timestamp NULL,
    "SUCCESS"            boolean   NOT NULL DEFAULT FALSE,
    "ERROR_MESSAGE"      text      NULL,
    "DURATION_MILLIS"    bigint    NULL
);

CREATE INDEX "idx__task_execution__library_id" ON "TASK_EXECUTION" ("LIBRARY_ID");
CREATE INDEX "idx__task_execution__start_date" ON "TASK_EXECUTION" ("START_DATE" DESC);
CREATE INDEX "idx__task_execution__simple_type" ON "TASK_EXECUTION" ("SIMPLE_TYPE");
