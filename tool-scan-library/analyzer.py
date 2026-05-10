import os
from typing import Any

import fitz  # PyMuPDF


class PdfAnalyzer:
    def __init__(self, library_root: str):
        self._library_root = library_root.rstrip("/")

    def _resolve_path(self, docker_path: str) -> str:
        if docker_path.startswith("file:"):
            return docker_path[5:]
        return docker_path

    @staticmethod
    def get_page_count(path: str) -> int:
        doc = fitz.open(path)
        try:
            return doc.page_count
        finally:
            doc.close()

    @staticmethod
    def get_all_pages(path: str) -> list[dict[str, Any]]:
        doc = fitz.open(path)
        try:
            pages = []
            for i in range(doc.page_count):
                page = doc[i]
                crop = page.cropbox
                pages.append({
                    "number": i + 1,
                    "file_name": str(i + 1),
                    "width": int(crop.width),
                    "height": int(crop.height),
                })
            return pages
        finally:
            doc.close()

    @staticmethod
    def compute_file_hash(path: str) -> str:
        import hashlib
        sha = hashlib.sha256()
        with open(path, "rb") as f:
            while True:
                chunk = f.read(65536)
                if not chunk:
                    break
                sha.update(chunk)
        return sha.hexdigest()

    def analyze(self, docker_path: str, skip_dimensions: bool = False, skip_hash: bool = False) -> dict[str, Any]:
        real_path = self._resolve_path(docker_path)
        if not os.path.isfile(real_path):
            raise FileNotFoundError(f"File not found: {real_path}")

        page_count = self.get_page_count(real_path)
        result: dict[str, Any] = {
            "page_count": page_count,
            "pages": [],
            "file_hash": "",
        }

        if not skip_dimensions:
            result["pages"] = self.get_all_pages(real_path)

        if not skip_hash:
            result["file_hash"] = self.compute_file_hash(real_path)

        return result
