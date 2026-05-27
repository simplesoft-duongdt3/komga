import os

KOMGA_URL = os.getenv("KOMGA_BASE_URL", "http://localhost:8080")
KOMGA_USER = os.getenv("KOMGA_USER", "admin@example.com")
KOMGA_PASSWORD = os.getenv("KOMGA_PASSWORD", "")

PG_HOST = os.getenv("PG_HOST", "localhost")
PG_PORT = os.getenv("PG_PORT", "5432")
PG_DB = os.getenv("PG_DB", "komga")
PG_USER = os.getenv("PG_USER", "komga")
PG_PASSWORD = os.getenv("PG_PASSWORD", "")

DRY_RUN = os.getenv("DRY_RUN", "false").lower() == "true"
MAX_API_RETRIES = int(os.getenv("MAX_API_RETRIES", "3"))
EXPORT_DIR = os.getenv("EXPORT_DIR", "/exports")
SCAN_THREADS = int(os.getenv("SCAN_THREADS", "0"))
HASH_CACHE = os.getenv("HASH_CACHE", "/exports/hashes.json")

