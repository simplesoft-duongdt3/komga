WITH SeriesCTE AS (
    SELECT "ID",
           ROW_NUMBER() OVER (
               PARTITION BY "URL" 
               ORDER BY "CREATED_DATE" ASC
           ) as row_num
    FROM public."SERIES"
    WHERE "LIBRARY_ID" = '0Q3CKC76902B7'
)
DELETE FROM public."SERIES"
WHERE "ID" IN (
    SELECT "ID" 
    FROM SeriesCTE 
    WHERE row_num > 1
);

WITH DeletionCTE AS (
    SELECT "ID",
           ROW_NUMBER() OVER (
               PARTITION BY "URL" 
               ORDER BY "CREATED_DATE" ASC, "ID" ASC
           ) as row_num
    FROM public."BOOK"
    WHERE "LIBRARY_ID" = '0Q3CKC76902B7'
)
DELETE FROM public."BOOK"
WHERE "ID" IN (
    SELECT "ID" 
    FROM DeletionCTE 
    WHERE row_num > 1
);

