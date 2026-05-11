CREATE TABLE LIBRARY_SCAN_EXECUTION
(
    ID                          varchar   NOT NULL PRIMARY KEY,
    TASK_EXECUTION_ID           varchar   NOT NULL,
    LIBRARY_ID                  varchar   NOT NULL,
    SCAN_DEEP                   integer   NOT NULL DEFAULT 0,
    START_DATE                  datetime  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    END_DATE                    datetime  NULL,

    -- Counters
    SCANNED_SERIES              integer   NOT NULL DEFAULT 0,
    SCANNED_BOOKS               integer   NOT NULL DEFAULT 0,
    SCANNED_SIDECARS            integer   NOT NULL DEFAULT 0,
    EXISTING_SERIES             integer   NOT NULL DEFAULT 0,
    EXISTING_SCANNED_SERIES     integer   NOT NULL DEFAULT 0,
    PRELOADED_BOOKS             integer   NOT NULL DEFAULT 0,
    DELETED_SERIES              integer   NOT NULL DEFAULT 0,
    DELETED_BOOKS               integer   NOT NULL DEFAULT 0,
    CREATED_SERIES              integer   NOT NULL DEFAULT 0,
    UPDATED_SERIES              integer   NOT NULL DEFAULT 0,
    ADDED_BOOKS                 integer   NOT NULL DEFAULT 0,
    DEFERRED_HASH_BOOKS         integer   NOT NULL DEFAULT 0,
    OUTDATED_BOOKS              integer   NOT NULL DEFAULT 0,
    SERIES_REFRESH_QUEUED       integer   NOT NULL DEFAULT 0,
    CHANGED_SIDECARS            integer   NOT NULL DEFAULT 0,
    DELETED_SIDECARS            integer   NOT NULL DEFAULT 0,

    -- Fan-out counters (Phase 2)
    ANALYZE_BOOK_COUNT          integer   NOT NULL DEFAULT 0,
    HASH_BOOK_COUNT             integer   NOT NULL DEFAULT 0,
    HASH_BOOK_KOREADER_COUNT    integer   NOT NULL DEFAULT 0,
    REPAIR_EXTENSION_COUNT      integer   NOT NULL DEFAULT 0,

    -- Phase timings (ms)
    TOTAL_MS                    bigint    NOT NULL DEFAULT 0,
    FILESYSTEM_SCAN_MS          bigint    NOT NULL DEFAULT 0,
    CLEAR_UNAVAILABLE_MS        bigint    NOT NULL DEFAULT 0,
    LOAD_EXISTING_MS            bigint    NOT NULL DEFAULT 0,
    DELETE_MISSING_SERIES_MS    bigint    NOT NULL DEFAULT 0,
    DELETE_MISSING_BOOKS_MS     bigint    NOT NULL DEFAULT 0,
    RECONCILE_SERIES_BOOKS_MS   bigint    NOT NULL DEFAULT 0,
    SORT_AND_REFRESH_MS         bigint    NOT NULL DEFAULT 0,
    RECONCILE_SIDECARS_MS       bigint    NOT NULL DEFAULT 0,
    CLEANUP_SIDECARS_MS         bigint    NOT NULL DEFAULT 0,
    CLEANUP_MS                  bigint    NOT NULL DEFAULT 0,

    -- Error tracking
    SUCCESS                     integer   NOT NULL DEFAULT 1,
    ERROR_MESSAGE               text      NULL
);

CREATE INDEX idx__lib_scan_exec__library_id ON LIBRARY_SCAN_EXECUTION (LIBRARY_ID);
CREATE INDEX idx__lib_scan_exec__start_date ON LIBRARY_SCAN_EXECUTION (START_DATE DESC);
CREATE INDEX idx__lib_scan_exec__success    ON LIBRARY_SCAN_EXECUTION (SUCCESS);
