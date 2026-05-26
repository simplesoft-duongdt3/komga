"""Komga REST API client."""

import requests
from urllib.parse import urljoin
from config import KOMGA_URL, KOMGA_USER, KOMGA_PASSWORD, MAX_API_RETRIES

_session = None


def _get_session():
    global _session
    if _session is None:
        _session = requests.Session()
        _session.auth = (KOMGA_USER, KOMGA_PASSWORD)
        _session.headers.update({"Content-Type": "application/json"})
    return _session


def _post(path: str, json_data: dict | None = None, **kwargs):
    s = _get_session()
    url = urljoin(KOMGA_URL, path)
    for attempt in range(1, MAX_API_RETRIES + 1):
        try:
            resp = s.post(url, json=json_data, timeout=300, **kwargs)
            resp.raise_for_status()
            return resp
        except requests.RequestException as e:
            if attempt == MAX_API_RETRIES:
                raise
            print(f"  Retry {attempt}/{MAX_API_RETRIES}: {e}")


def _delete(path: str):
    s = _get_session()
    url = urljoin(KOMGA_URL, path)
    for attempt in range(1, MAX_API_RETRIES + 1):
        try:
            resp = s.delete(url, timeout=120)
            resp.raise_for_status()
            return resp
        except requests.RequestException as e:
            if attempt == MAX_API_RETRIES:
                raise
            print(f"  Retry {attempt}/{MAX_API_RETRIES}: {e}")


def list_libraries() -> list[dict]:
    resp = _get_session().get(
        urljoin(KOMGA_URL, "/api/v1/libraries")
    )
    resp.raise_for_status()
    data = resp.json()
    if isinstance(data, dict):
        data = data.get("content", list(data.values()))
    if not isinstance(data, list):
        raise ValueError(f"Unexpected library response: {type(data)}")
    return data


def create_series(library_id: str, name: str, url: str,
                  file_last_modified: str,
                  books: list[dict]) -> dict:
    """Create series + register its books. Returns the created SeriesDto."""
    return _post("/api/v1/series", {
        "libraryId": library_id,
        "name": name,
        "url": url,
        "fileLastModified": file_last_modified,
        "books": [
            {
                "name": b["name"],
                "url": b["url"],
                "fileSize": b["file_size"],
                "fileLastModified": b["file_last_modified"],
                "fileHash": b.get("file_hash", ""),
            }
            for b in books
        ],
    }).json()


def delete_book(book_id: str):
    _delete(f"/api/v1/books/{book_id}/file")


def delete_series(series_id: str):
    _delete(f"/api/v1/series/{series_id}/file")


def get_series_books(series_id: str) -> list[dict]:
    """Return all books for a series."""
    return _get_session().get(
        urljoin(KOMGA_URL, f"/api/v1/series/{series_id}/books"),
        params={"unpaged": "true"},
    ).json().get("content", [])


def empty_trash(library_id: str):
    _post(f"/api/v1/libraries/{library_id}/empty-trash")


def analyze_book(book_id: str):
    _post(f"/api/v1/books/{book_id}/analyze")


def refresh_book_metadata(book_id: str):
    _post(f"/api/v1/books/{book_id}/metadata/refresh")


def refresh_series_metadata(series_id: str):
    _post(f"/api/v1/series/{series_id}/metadata/refresh")
