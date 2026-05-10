import os
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class DbConfig:
    host: str = "192.168.1.169"
    port: int = 5433
    database: str = "komga"
    user: str = "ai_readonly"
    password: str = "ai_readonly_pass"
    write_user: Optional[str] = None
    write_password: Optional[str] = None
    min_connections: int = 2
    max_connections: int = 10


@dataclass
class LibraryConfig:
    library_id: str = "0Q3CKC76902B7"
    library_root_path: str = "/data/data-books-audiobooks/Manga_Ebook/Manhwa"
    scan_workers: int = 0


@dataclass
class SyncConfig:
    commit_batch_size: int = 5000


@dataclass
class Config:
    db: DbConfig = field(default_factory=DbConfig)
    library: LibraryConfig = field(default_factory=LibraryConfig)
    sync: SyncConfig = field(default_factory=SyncConfig)

    @classmethod
    def from_env(cls) -> "Config":
        cfg = cls()
        cfg.db.host = os.environ.get("KOMGA_DB_HOST", cfg.db.host)
        cfg.db.port = int(os.environ.get("KOMGA_DB_PORT", cfg.db.port))
        cfg.db.database = os.environ.get("KOMGA_DB_NAME", cfg.db.database)
        cfg.db.user = os.environ.get("KOMGA_DB_USER", cfg.db.user)
        cfg.db.password = os.environ.get("KOMGA_DB_PASS", cfg.db.password)
        cfg.db.write_user = os.environ.get("KOMGA_DB_WRITE_USER", cfg.db.write_user)
        cfg.db.write_password = os.environ.get("KOMGA_DB_WRITE_PASS", cfg.db.write_password)
        cfg.db.min_connections = int(os.environ.get("KOMGA_DB_MIN_CONN", cfg.db.min_connections))
        cfg.db.max_connections = int(os.environ.get("KOMGA_DB_MAX_CONN", cfg.db.max_connections))
        cfg.library.library_id = os.environ.get("KOMGA_LIBRARY_ID", cfg.library.library_id)
        cfg.library.library_root_path = os.environ.get("KOMGA_LIBRARY_ROOT", cfg.library.library_root_path)
        cfg.library.scan_workers = int(os.environ.get("KOMGA_SCAN_WORKERS", cfg.library.scan_workers))
        cfg.sync.commit_batch_size = int(os.environ.get("KOMGA_BATCH_SIZE", cfg.sync.commit_batch_size))
        return cfg


BOOK_EXTENSIONS = frozenset({".pdf"})

SERIES_THUMBNAIL_NAMES = frozenset({
    "poster.jpg",
    "cover.jpg",
    "default.jpg",
    "folder.jpg",
    "series.jpg",
})
