#!/usr/bin/env python3
"""Komga production DB — live performance dashboard. Serves static files + API."""
import json
import os
import sys
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse

try:
    import psycopg2
except ImportError:
    print("ERROR: pip3 install psycopg2-binary")
    sys.exit(1)

DB = dict(
    host="192.168.1.169", port=5433, dbname="komga",
    user="ai_readonly", password="ai_readonly_pass", connect_timeout=5,
)
LISTEN_HOST, LISTEN_PORT, REFRESH_SEC = "0.0.0.0", 8080, 5
DIR = os.path.dirname(os.path.abspath(__file__))


def query(sql, params=None):
    try:
        conn = psycopg2.connect(**DB)
        conn.set_session(autocommit=True)
        with conn.cursor() as cur:
            cur.execute(sql, params)
            if cur.description:
                cols = [d[0] for d in cur.description]
                return [dict(zip(cols, row)) for row in cur.fetchall()]
            return []
    except psycopg2.Error:
        return None
    finally:
        try: conn.close()
        except Exception: pass


def build_stats():
    t0 = datetime.now(timezone.utc)
    data = {"now": t0.isoformat(), "took_ms": 0}

    r = query("SELECT indexname FROM pg_indexes WHERE schemaname='public' AND tablename='TASK'")
    if r is None: return {"error": "DB connection failed"}
    names = {x["indexname"] for x in r}
    data["idxs"] = {"idx_task_queue": "idx_task_queue" in names, "idx_task_owner_group": "idx_task_owner_group" in names}

    r = query('SELECT COUNT(*) AS cnt FROM "TASK" WHERE "OWNER" IS NULL')
    data["queue_pending"] = r[0]["cnt"] if r else 0
    r = query('SELECT COUNT(*) AS cnt FROM "TASK" WHERE "OWNER" IS NOT NULL')
    data["queue_running"] = r[0]["cnt"] if r else 0
    r = query('SELECT COUNT(*) AS cnt FROM "TASK"')
    data["task_rows"] = r[0]["cnt"] if r else 0
    r = query('SELECT COUNT(*) AS cnt FROM "TASK_EXECUTION"')
    data["task_exec_rows"] = r[0]["cnt"] if r else 0

    data["group_queue"] = query("""
        SELECT COALESCE("GROUP_ID", '(null)') AS group_id,
               "SIMPLE_TYPE",
               COUNT(*) FILTER (WHERE "OWNER" IS NULL) AS pending,
               COUNT(*) FILTER (WHERE "OWNER" IS NOT NULL) AS running,
               COUNT(*) AS total
        FROM "TASK"
        GROUP BY "GROUP_ID", "SIMPLE_TYPE"
        ORDER BY pending DESC, running DESC
        LIMIT 30
    """) or []

    data["running_detail"] = query("""
        SELECT "SIMPLE_TYPE", COALESCE("GROUP_ID", '(null)') AS group_id,
               "OWNER",
               EXTRACT(epoch FROM NOW() - "CREATED_DATE")::int AS age_sec,
               EXTRACT(epoch FROM NOW() - "LAST_MODIFIED_DATE")::int AS stale_sec
        FROM "TASK"
        WHERE "OWNER" IS NOT NULL
        ORDER BY "CREATED_DATE"
    """) or []

    data["concurrency_ceiling"] = query("""
        SELECT COUNT(DISTINCT "GROUP_ID") AS groups
        FROM "TASK"
        WHERE "OWNER" IS NULL AND "GROUP_ID" IS NOT NULL
    """)

    data["type_stats"] = query("""
        SELECT "SIMPLE_TYPE", COUNT(*) AS n,
               ROUND(AVG("DURATION_MILLIS"))::int AS avg_ms,
               PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY "DURATION_MILLIS")::int AS p50,
               PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY "DURATION_MILLIS")::int AS p95,
               PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY "DURATION_MILLIS")::int AS p99,
               MAX("DURATION_MILLIS")::int AS max_ms,
               COUNT(*) FILTER (WHERE "SUCCESS") AS ok,
               COUNT(*) FILTER (WHERE NOT "SUCCESS") AS fail
        FROM "TASK_EXECUTION" WHERE "DURATION_MILLIS" IS NOT NULL
        GROUP BY "SIMPLE_TYPE" ORDER BY avg_ms DESC
    """) or []

    data["throughput"] = query("""
        SELECT date_trunc('hour', "START_DATE") AS hour, COUNT(*) AS cnt
        FROM "TASK_EXECUTION"
        WHERE "START_DATE" > NOW() - INTERVAL '24 hours'
        GROUP BY hour ORDER BY hour
    """) or []

    data["queue_delay"] = query("""
        SELECT "SIMPLE_TYPE",
               ROUND(AVG(EXTRACT(epoch FROM "END_DATE" - "START_DATE") * 1000))::int AS avg_wall_ms,
               ROUND(AVG("DURATION_MILLIS"))::int AS avg_work_ms, COUNT(*) AS n
        FROM "TASK_EXECUTION"
        WHERE "DURATION_MILLIS" IS NOT NULL AND "END_DATE" IS NOT NULL
          AND "START_DATE" > NOW() - INTERVAL '1 hour'
        GROUP BY "SIMPLE_TYPE" ORDER BY n DESC
    """) or []

    data["recent_failures"] = query("""
        SELECT "SIMPLE_TYPE", "START_DATE", "ERROR_MESSAGE", "DURATION_MILLIS", "LIBRARY_ID", "BOOK_ID"
        FROM "TASK_EXECUTION" WHERE "SUCCESS" = FALSE ORDER BY "START_DATE" DESC LIMIT 10
    """) or []

    data["recent"] = query("""
        SELECT "SIMPLE_TYPE", "START_DATE", "DURATION_MILLIS", "SUCCESS"
        FROM "TASK_EXECUTION" ORDER BY "START_DATE" DESC LIMIT 20
    """) or []

    data["activity"] = query("""
        SELECT pid, state, wait_event_type, wait_event,
               LEFT(query, 80) AS q,
               EXTRACT(epoch FROM NOW() - query_start)::int AS elapsed_sec
        FROM pg_stat_activity
        WHERE state IS NOT NULL AND datname = current_database() AND pid <> pg_backend_pid()
        ORDER BY query_start LIMIT 20
    """) or []

    b = query("""SELECT COUNT(*) AS cnt FROM pg_stat_activity blocked
        JOIN pg_locks bl ON blocked.pid = bl.pid
        JOIN pg_locks bll ON bl.locktype = bll.locktype AND bl.relation = bll.relation AND bl.pid <> bll.pid
        JOIN pg_stat_activity blocking ON bll.pid = blocking.pid
        WHERE NOT bl.granted AND blocked.datname = current_database()""")
    data["blocked"] = b[0]["cnt"] if b else 0
    lw = query("""SELECT COUNT(*) AS cnt FROM pg_stat_activity
        WHERE wait_event_type = 'Lock' AND datname = current_database() AND pid <> pg_backend_pid()""")
    data["lock_waiters"] = lw[0]["cnt"] if lw else 0

    data["table_health"] = query("""
        SELECT relname, n_live_tup, n_dead_tup,
               COALESCE(last_vacuum,last_autovacuum)::text AS last_vc,
               COALESCE(last_analyze,last_autoanalyze)::text AS last_an
        FROM pg_stat_user_tables WHERE relname ILIKE '%task%'
    """) or []

    data["pg_config"] = query("""
        SELECT name, setting, unit FROM pg_settings
        WHERE name IN ('shared_buffers','effective_cache_size','work_mem',
                       'maintenance_work_mem','random_page_cost','max_wal_size')
        ORDER BY name
    """) or []

    plan = query("""
        EXPLAIN (COSTS OFF, FORMAT TEXT)
        SELECT "ID" FROM "TASK"
        WHERE "OWNER" IS NULL
          AND ("GROUP_ID" IS NULL OR NOT EXISTS (
            SELECT 1 FROM "TASK" t2
            WHERE t2."GROUP_ID" = "TASK"."GROUP_ID"
              AND t2."OWNER" IS NOT NULL AND t2."GROUP_ID" IS NOT NULL
          ))
        ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE" LIMIT 1
    """)
    plan_text = " ".join(list(r.values())[0] for r in plan) if plan else "N/A"
    data["explain"] = plan_text
    data["uses_index"] = any(x in plan_text for x in ("Index Scan", "Bitmap Index Scan"))

    data["took_ms"] = int((datetime.now(timezone.utc) - t0).total_seconds() * 1000)
    return data


def serve_file(path, content_type, handler, substitutions=None):
    full = os.path.join(DIR, path)
    try:
        with open(full, "r", encoding="utf-8") as f:
            body = f.read()
        if substitutions:
            for old, new in substitutions.items():
                body = body.replace(old, new)
        payload = body.encode()
    except OSError:
        handler.send_response(404)
        handler.end_headers()
        return
    handler.send_response(200)
    handler.send_header("Content-Type", content_type)
    handler.send_header("Content-Length", str(len(payload)))
    handler.end_headers()
    handler.wfile.write(payload)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        path = urlparse(self.path).path
        if path == "/api/stats":
            data = build_stats()
            body = json.dumps(data, default=str).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif path in ("/", "/index.html"):
            serve_file("dashboard.html", "text/html; charset=utf-8", self,
                       {"__REFRESH_SEC__": str(REFRESH_SEC)})
        elif path == "/dashboard.js":
            serve_file("dashboard.js", "application/javascript; charset=utf-8", self)
        else:
            self.send_response(404)
            self.end_headers()


def main():
    print(f"Komga DB Dashboard → http://localhost:{LISTEN_PORT}")
    try:
        HTTPServer((LISTEN_HOST, LISTEN_PORT), Handler).serve_forever()
    except KeyboardInterrupt:
        print("\nShutdown.")


if __name__ == "__main__":
    main()
