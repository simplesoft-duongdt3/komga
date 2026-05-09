# Komga Production DB — Performance Verification

## Setup

```bash
pip3 install -r requirements.txt
```

Requires Python 3.8+ and network access to the production PostgreSQL instance.

Connection details are hardcoded in each script (`DB_CONFIG` / `DB` dict) — edit them if the target host changes.

## Tools

### `prod_check.py` — One-shot CLI report

Runs all checks once and prints results to stdout.

```bash
python3 prod_check.py
```

Covers:
- Index existence (idx_task_queue, idx_task_owner_group)
- Query execution plan (EXPLAIN) for the `takeFirst()` claim query
- PostgreSQL configuration vs. recommended values
- Active queries (pg_stat_activity)
- Lock contention
- Table row counts, dead tuples, vacuum history

### `dashboard.py` — Real-time web UI

Starts a lightweight HTTP server with an auto-refreshing dashboard.

```bash
python3 dashboard.py
# → http://localhost:8080
```

- Refreshes every 3 seconds
- No extra dependencies beyond psycopg2-binary
- Zero writes — read-only DB user is sufficient
- Ctrl+C to stop

## Production DB credentials

| Field | Value |
|---|---|
| Host | 192.168.1.169 |
| Port | 5433 |
| Database | komga |
| User | ai_readonly |
| Password | ai_readonly_pass |
