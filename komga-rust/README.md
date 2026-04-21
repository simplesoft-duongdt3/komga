# Komga Rust

Rust implementation of Komga media server for comics/manga.

## Requirements

- Rust 1.70+
- PostgreSQL 14+
- (Optional) Redis for session/cache

## Setup

1. Copy `.env.example` to `.env` and configure:
   ```bash
   cp .env.example .env
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run database migrations:
   ```bash
   cargo run -- migrate
   ```

4. Start the server:
   ```bash
   cargo run --release
   ```

## API Endpoints

### Authentication
- `POST /api/v1/users` - Register new user
- `POST /api/v1/users/login` - Login
- `GET /api/v1/users/me` - Get current user

### Libraries
- `GET /api/v1/libraries` - List all libraries
- `POST /api/v1/libraries` - Create library
- `GET /api/v1/libraries/{id}` - Get library
- `DELETE /api/v1/libraries/{id}` - Delete library

### Series
- `GET /api/v1/libraries/{libraryId}/series` - List series in library
- `GET /api/v1/series/{id}` - Get series

### Books
- `GET /api/v1/series/{seriesId}/books` - List books in series
- `GET /api/v1/books/{id}` - Get book

## Development

Run tests:
```bash
cargo test
```

Run with debug logging:
```bash
RUST_LOG=debug cargo run
```