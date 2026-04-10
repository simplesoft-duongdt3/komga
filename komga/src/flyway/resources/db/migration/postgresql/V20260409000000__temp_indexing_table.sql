-- Create permanent TEMP_INDEXING table for PostgreSQL
-- Replaces the dynamic unlogged table approach in TempTable.kt
-- Each TempTable instance uses a unique INDEX_NAME to isolate its rows
CREATE UNLOGGED TABLE "TEMP_INDEXING" (
    "STRING"     varchar NOT NULL,
    "INDEX_NAME" varchar NOT NULL
);

CREATE INDEX "idx__temp_indexing__index_name" ON "TEMP_INDEXING" ("INDEX_NAME");
