#!/usr/bin/env python3
"""Verify Komga production DB performance fixes from perf-solutions.md."""
import sys
import os
from datetime import datetime

try:
    import psycopg2
    import psycopg2.extras
except ImportError:
    print("ERROR: psycopg2-binary required. Run: pip3 install psycopg2-binary")
    sys.exit(1)

from typing import Any

DB_CONFIG = {
    "host": "192.168.1.169",
    "port": 5433,
    "dbname": "komga",
    "user": "ai_readonly",
    "password": "ai_readonly_pass",
    "connect_timeout": 10,
}

SEPARATOR = "=" * 70


def connect() -> psycopg2.extensions.connection:
    conn = psycopg2.connect(**DB_CONFIG)
    conn.set_session(autocommit=True)
    return conn


def run_query(
    conn: psycopg2.extensions.connection,
    sql: str,
    params: tuple | None = None,
    fetchall: bool = True,
) -> list[tuple[Any, ...]] | None:
    try:
        with conn.cursor() as cur:
            cur.execute(sql, params)
            if cur.description:
                return cur.fetchall() if fetchall else cur.fetchone()
            return None
    except psycopg2.Error as e:
        print(f"  ERROR: {e}")
        return None


def section(title: str):
    print(f"\n{SEPARATOR}")
    print(f"  {title}")
    print(SEPARATOR)


# ── 1. Connection Info ──────────────────────────────────────────────
def check_connection(conn: psycopg2.extensions.connection):
    section("1. Connection & Version")
    rows = run_query(conn, "SELECT version()")
    if rows:
        print(f"  PostgreSQL: {rows[0][0][:80]}...")
    rows = run_query(conn, "SELECT current_database(), inet_server_addr(), inet_server_port()")
    if rows:
        db, host, port = rows[0]
        print(f"  Database: {db}  Host: {host}  Port: {port}")


# ── 2. Recommended Indexes ‐─────────────────────────────────────────
def check_indexes(conn: psycopg2.extensions.connection):
    section("2. Recommended Indexes (Fix: perf-solutions.md §6)")

    idx_sql = """
        SELECT indexname, indexdef
        FROM pg_indexes
        WHERE schemaname = 'public' AND tablename = 'TASK'
        ORDER BY indexname
    """
    rows = run_query(conn, idx_sql) or []

    required = [
        ("idx_task_queue", "Partial index on (OWNER, PRIORITY DESC, LAST_MODIFIED_DATE) WHERE OWNER IS NULL"),
        ("idx_task_owner_group", "Partial index on (OWNER, GROUP_ID) WHERE OWNER IS NOT NULL AND GROUP_ID IS NOT NULL"),
    ]

    existing_names = {r[0] for r in rows}
    for name, desc in required:
        status = "EXISTS" if name in existing_names else "MISSING"
        icon = "[OK]" if status == "EXISTS" else "[!!]"
        print(f"  {icon} {name}")
        print(f"      Purpose: {desc}")
        if status == "EXISTS":
            defn = next((r[1] for r in rows if r[0] == name), "")
            print(f"      Definition: {defn[:110]}...")
        print()

    if rows:
        additional = [r for r in rows if r[0] not in dict(required)]
        if additional:
            print("  Other indexes on TASK:")
            for name, defn in additional:
                print(f"    - {name}")


# ── 3. Table Structure ─────────────────────────────────────────────
def check_tables(conn: psycopg2.extensions.connection):
    section("3. Table Structure & Row Counts")

    tables = run_query(conn, """
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public' AND tablename ILIKE '%task%'
        ORDER BY tablename
    """) or []

    if not tables:
        # fallback: list ALL tables
        tables = run_query(conn, """
            SELECT tablename
            FROM pg_tables
            WHERE schemaname = 'public'
            ORDER BY tablename
        """) or []

    for (tname,) in tables:
        row = run_query(conn, f"SELECT count(*) FROM \"{tname}\"", fetchall=False)
        size = run_query(conn, f"""
            SELECT pg_size_pretty(pg_total_relation_size('public."{tname}"'))
        """, fetchall=False)
        n_rows = row[0] if row else "?"
        sz = size[0] if size else "?"
        print(f"  {tname}: {n_rows} rows, {sz} total")

        # Show columns for TASK and TASK_EXECUTION
        if tname in ("TASK", "TASK_EXECUTION"):
            cols = run_query(conn, f"""
                SELECT column_name, data_type, is_nullable
                FROM information_schema.columns
                WHERE table_schema = 'public' AND table_name = %s
                ORDER BY ordinal_position
            """, (tname,))
            if cols:
                for col, dtype, nullable in cols:
                    pk = ""
                    print(f"      {col:30s} {dtype:15s} {'NULL' if nullable == 'YES' else 'NOT NULL'}{pk}")


# ── 4. PostgreSQL Configuration ─────────────────────────────────────
RECOMMENDED_CONFIG = {
    "shared_buffers": "512MB",
    "effective_cache_size": "1GB",
    "work_mem": "16MB",
    "maintenance_work_mem": "128MB",
    "random_page_cost": "1.1",
    "max_wal_size": "2GB",
}


def check_pg_config(conn: psycopg2.extensions.connection):
    section("4. PostgreSQL Configuration (Fix: perf-solutions.md §7)")

    settings = run_query(conn, """
        SELECT name, setting, unit
        FROM pg_settings
        WHERE name = ANY(%s)
        ORDER BY name
    """, (list(RECOMMENDED_CONFIG.keys()),)) or []

    current = {name: (setting, unit) for name, setting, unit in settings}

    print(f"  {'Setting':<28s} {'Current':>12s} {'Recommend':>12s} {'Match':>8s}")
    print(f"  {'-'*28} {'-'*12} {'-'*12} {'-'*8}")
    for name, recommended in RECOMMENDED_CONFIG.items():
        if name in current:
            setting, unit = current[name]
            if unit:
                current_str = f"{setting}{unit}"
            else:
                current_str = setting
            match = "[OK]" if _config_matches(name, setting, recommended) else "[!!]"
        else:
            current_str = "N/A"
            match = "[??]"
        print(f"  {name:<28s} {current_str:>12s} {recommended:>12s} {match:>8s}")

    # Also show max_connections
    mc = run_query(conn, "SELECT setting FROM pg_settings WHERE name='max_connections'", fetchall=False)
    if mc:
        print(f"\n  max_connections: {mc[0]}")


def _config_matches(name: str, current: str, recommended: str) -> bool:
    """Compare config values accounting for unit suffixes like MB, GB."""
    import re

    def _to_bytes(val: str) -> float:
        val = val.upper()
        multipliers = {"KB": 1024, "MB": 1024**2, "GB": 1024**3, "TB": 1024**4}
        for unit, mult in multipliers.items():
            if val.endswith(unit):
                return float(val.replace(unit, "")) * mult
        return float(val)

    try:
        return abs(_to_bytes(current) - _to_bytes(recommended)) / _to_bytes(recommended) < 0.01
    except (ValueError, ZeroDivisionError):
        return current == recommended


# ── 5. Current Activity ────────────────────────────────────────────
def check_activity(conn: psycopg2.extensions.connection):
    section("5. Current Query Activity (pg_stat_activity)")

    rows = run_query(conn, """
        SELECT pid,
               state,
               wait_event_type,
               wait_event,
               LEFT(query, 120) AS query_preview,
               age(now(), query_start)::text AS duration
        FROM pg_stat_activity
        WHERE state IS NOT NULL
          AND datname = current_database()
          AND pid <> pg_backend_pid()
        ORDER BY query_start
    """) or []

    if not rows:
        print("  No active queries (all threads idle).")
        return

    print(f"  {'PID':<8s} {'State':<12s} {'Wait Type':<12s} {'Wait Event':<20s} {'Duration':<15s} Query")
    print(f"  {'-'*8} {'-'*12} {'-'*12} {'-'*20} {'-'*15} {'-'*40}")
    for pid, state, wait_type, wait_event, query, duration in rows:
        wait_type = wait_type or ""
        wait_event = wait_event or ""
        query = (query or "").replace("\n", " ")[:100]
        print(f"  {str(pid):<8s} {state:<12s} {wait_type:<12s} {wait_event:<20s} {duration:<15s} {query}")


# ── 6. Execution Plan (EXPLAIN ANALYZE) ─────────────────────────────
def check_explain_plans(conn: psycopg2.extensions.connection):
    section("6. Query Execution Plans (EXPLAIN without ANALYZE)")

    # The takeFirst() query pattern from TasksDao.kt — with LIMIT
    take_first_sql = """
        EXPLAIN (COSTS OFF, FORMAT TEXT)
        SELECT "ID" FROM "TASK"
        WHERE "OWNER" IS NULL
          AND ("GROUP_ID" IS NULL
               OR NOT EXISTS (
                 SELECT 1 FROM "TASK" t2
                 WHERE t2."GROUP_ID" = "TASK"."GROUP_ID"
                   AND t2."OWNER" IS NOT NULL
                   AND t2."GROUP_ID" IS NOT NULL
               ))
        ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
        LIMIT 1
    """

    print("  --- takeFirst() query plan (post-fix) ---")
    rows = run_query(conn, take_first_sql)
    if rows:
        for (line,) in rows:
            print(f"    {line}")

    # Check if plan mentions index scans
    plan_text = " ".join(r[0] for r in (rows or []))
    print()
    if "Index" in plan_text or "Index Scan" in plan_text:
        print("  Index usage detected in plan.")
    elif "Seq Scan" in plan_text or "Parallel Seq Scan" in plan_text:
        print("  WARNING: Sequential scan detected — indexes may be missing or not used.")
        _suggest_index_creation(conn)
    else:
        print("  Plan analysis inconclusive (no Index/Seq Scan reference).")

    # Also check the actual query timing via EXPLAIN ANALYZE (limited safety)
    print()
    print("  --- takeFirst() with EXPLAIN ANALYZE (BUFFERS) ---")
    analyze_sql = """
        EXPLAIN (ANALYZE, BUFFERS, COSTS OFF, TIMING OFF, SUMMARY OFF, FORMAT TEXT)
        SELECT "ID" FROM "TASK"
        WHERE "OWNER" IS NULL
          AND ("GROUP_ID" IS NULL
               OR NOT EXISTS (
                 SELECT 1 FROM "TASK" t2
                 WHERE t2."GROUP_ID" = "TASK"."GROUP_ID"
                   AND t2."OWNER" IS NOT NULL
                   AND t2."GROUP_ID" IS NOT NULL
               ))
        ORDER BY "PRIORITY" DESC, "LAST_MODIFIED_DATE"
        LIMIT 1
    """
    rows2 = run_query(conn, analyze_sql)
    if rows2:
        for (line,) in rows2:
            print(f"    {line}")


def _suggest_index_creation(conn: psycopg2.extensions.connection):
    print()
    print("  --- Suggested index creation (run as admin) ---")
    print("""
    CREATE INDEX IF NOT EXISTS idx_task_queue
      ON "TASK" ("OWNER", "PRIORITY" DESC, "LAST_MODIFIED_DATE")
      WHERE "OWNER" IS NULL;

    CREATE INDEX IF NOT EXISTS idx_task_owner_group
      ON "TASK" ("OWNER", "GROUP_ID")
      WHERE "OWNER" IS NOT NULL AND "GROUP_ID" IS NOT NULL;
    """)


# ── 7. Lock & Contention Analysis ──────────────────────────────────
def check_locks(conn: psycopg2.extensions.connection):
    section("7. Lock Analysis")

    blocked = run_query(conn, """
        SELECT blocked.pid,
               blocked.query AS blocked_query,
               blocking.pid AS blocking_pid,
               blocking.query AS blocking_query
        FROM pg_stat_activity blocked
        JOIN pg_locks blocked_locks ON blocked.pid = blocked_locks.pid
        JOIN pg_locks blocking_locks ON blocked_locks.locktype = blocking_locks.locktype
            AND blocked_locks.relation = blocking_locks.relation
            AND blocked_locks.pid <> blocking_locks.pid
        JOIN pg_stat_activity blocking ON blocking_locks.pid = blocking.pid
        WHERE NOT blocked_locks.granted
          AND blocked.datname = current_database()
        LIMIT 20
    """) or []

    if blocked:
        print(f"  WARNING: {len(blocked)} blocked query(ies) found!")
        for b_pid, b_q, blk_pid, blk_q in blocked:
            print(f"  Blocked PID {b_pid}: {(b_q or '')[:80]}")
            print(f"    Blocking PID {blk_pid}: {(blk_q or '')[:80]}")
    else:
        print("  No blocked queries — no lock contention detected.")

    # Show lock-heavy queries
    heavy = run_query(conn, """
        SELECT pid,
               wait_event_type,
               wait_event,
               LEFT(query, 100)
        FROM pg_stat_activity
        WHERE wait_event_type = 'Lock'
          AND datname = current_database()
          AND pid <> pg_backend_pid()
    """) or []
    if heavy:
        print(f"\n  {len(heavy)} query(ies) waiting on locks:")
        for pid, wt, we, q in heavy:
            print(f"    PID {pid}: {wt}/{we} — {(q or '')[:80]}")


# ── 8. Table Stats ────────────────────────────────────────────────
def check_table_stats(conn: psycopg2.extensions.connection):
    section("8. Table Statistics & Vacuum")

    rows = run_query(conn, """
        SELECT schemaname,
               relname,
               n_live_tup,
               n_dead_tup,
               last_vacuum,
               last_autovacuum,
               last_analyze,
               last_autoanalyze
        FROM pg_stat_user_tables
        WHERE relname ILIKE '%task%'
        ORDER BY relname
    """) or []

    if rows:
        print(f"  {'Table':<25s} {'Live':>8s} {'Dead':>8s} {'Last Vacuum':<20s} {'Last Analyze':<20s}")
        print(f"  {'-'*25} {'-'*8} {'-'*8} {'-'*20} {'-'*20}")
        for schema, rel, live, dead, vac, avac, ana, aana in rows:
            last_vac = str(vac or avac or "never")
            last_ana = str(ana or aana or "never")
            print(f"  {rel:<25s} {str(live):>8s} {str(dead):>8s} {last_vac:<20s} {last_ana:<20s}")

        dead_total = sum(int(r[2]) for r in rows)
        if dead_total > 1000:
            print(f"\n  NOTE: {dead_total} dead tuples detected. Autovacuum should handle these.")
    else:
        print("  No TASK-related tables in pg_stat_user_tables (check schema).")


# ── main ───────────────────────────────────────────────────────────
def main():
    print(f"\n{'#'*70}")
    print(f"#  Komga Production DB — Performance Fix Verification")
    print(f"#  Timestamp: {datetime.now().isoformat()}")
    print(f"#  Host: {DB_CONFIG['host']}:{DB_CONFIG['port']}/{DB_CONFIG['dbname']}")
    print(f"{'#'*70}")

    conn = connect()
    try:
        check_connection(conn)
        check_tables(conn)
        check_indexes(conn)
        check_explain_plans(conn)
        check_pg_config(conn)
        check_activity(conn)
        check_locks(conn)
        check_table_stats(conn)
    finally:
        conn.close()

    print(f"\n{SEPARATOR}")
    print("  Verification complete.")
    print(SEPARATOR + "\n")


if __name__ == "__main__":
    main()
