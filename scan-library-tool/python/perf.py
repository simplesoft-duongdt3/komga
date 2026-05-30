"""Performance logging — tracks per-phase timings during scan."""

import json
import os
import time
import threading
from datetime import datetime, timezone


class ScanTimer:
    def __init__(self, request_id: str, export_dir: str):
        self.request_id = request_id
        self.export_dir = export_dir
        self.start = time.monotonic()
        self.phases: list[dict] = []
        self._current: tuple[str, float] | None = None
        self._current_meta: dict | None = None
        self._lock = threading.Lock()

    def begin(self, name: str, meta: dict | None = None):
        with self._lock:
            now = time.monotonic()
            if self._current:
                self._end_current(now)
            self._current = (name, now)
            self._current_meta = meta

    def end(self):
        with self._lock:
            if self._current:
                self._end_current(time.monotonic())

    def _end_current(self, now: float):
        name, start = self._current
        elapsed_ms = round((now - start) * 1000)
        entry = {"phase": name, "elapsed_ms": elapsed_ms}
        if self._current_meta:
            entry["meta"] = self._current_meta
            self._current_meta = None
        self.phases.append(entry)
        self._current = None

    def record_phase(self, name: str, elapsed_ms: int, meta: dict | None = None):
        with self._lock:
            entry = {"phase": name, "elapsed_ms": elapsed_ms}
            if meta:
                entry["meta"] = meta
            self.phases.append(entry)

    def finish(self) -> dict:
        self.end()
        total_ms = round((time.monotonic() - self.start) * 1000)
        result = {
            "request_id": self.request_id,
            "exported_at": datetime.now(tz=timezone.utc).isoformat(),
            "total_ms": total_ms,
            "phases": self.phases,
        }
        return result

    def write_log(self, folder: str, result: dict):
        path = os.path.join(folder, f"{self.request_id}_perf.json")
        with open(path, "w") as f:
            json.dump(result, f, indent=2)
