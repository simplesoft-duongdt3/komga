#!/usr/bin/env python3

import argparse
import json
import sqlite3
import statistics
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, TypeAlias, cast


JsonDict: TypeAlias = dict[str, Any]


TIMESTAMP_FORMATS = (
    "%Y-%m-%d %H:%M:%S.%f",
    "%Y-%m-%d %H:%M:%S",
)


@dataclass
class TaskRow:
    id: str
    priority: int
    group_id: str | None
    class_name: str
    simple_type: str
    payload: str
    owner: str | None
    created_date: str
    last_modified_date: str

    @property
    def created_at(self) -> datetime | None:
        return parse_timestamp(self.created_date)

    @property
    def modified_at(self) -> datetime | None:
        return parse_timestamp(self.last_modified_date)

    @property
    def payload_length(self) -> int:
        return len(self.payload)

    @property
    def payload_json(self) -> JsonDict | None:
        try:
            data = json.loads(self.payload)
            return cast(JsonDict, data) if isinstance(data, dict) else None
        except json.JSONDecodeError:
            return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Analyze a Komga tasks SQLite snapshot and highlight queue bottlenecks.",
    )
    parser.add_argument(
        "db_path",
        nargs="?",
        default="task-db-data/tasks.sqlite",
        help="Path to the tasks.sqlite database file.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=10,
        help="Number of rows to show in detailed sections.",
    )
    parser.add_argument(
        "--stale-hours",
        type=float,
        default=6.0,
        help="Threshold in hours for flagging owned tasks as stale.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON instead of a text report.",
    )
    return parser.parse_args()


def parse_timestamp(value: str | None) -> datetime | None:
    if not value:
        return None
    normalized = value.strip()
    if "." in normalized:
        prefix, suffix = normalized.split(".", 1)
        digits = "".join(ch for ch in suffix if ch.isdigit())
        trimmed_digits = digits[:6]
        normalized = prefix if not trimmed_digits else f"{prefix}.{trimmed_digits}"
    for fmt in TIMESTAMP_FORMATS:
        try:
            dt = datetime.strptime(normalized, fmt)
            return dt.replace(tzinfo=timezone.utc)
        except ValueError:
            continue
    return None


def fetch_tasks(db_path: Path) -> list[TaskRow]:
    connection = sqlite3.connect(db_path)
    connection.row_factory = sqlite3.Row
    try:
        rows = connection.execute(
            """
            SELECT ID, PRIORITY, GROUP_ID, CLASS, SIMPLE_TYPE, PAYLOAD, OWNER, CREATED_DATE, LAST_MODIFIED_DATE
            FROM TASK
            """
        ).fetchall()
    finally:
        connection.close()

    return [
        TaskRow(
            id=row["ID"],
            priority=row["PRIORITY"],
            group_id=row["GROUP_ID"],
            class_name=row["CLASS"],
            simple_type=row["SIMPLE_TYPE"],
            payload=row["PAYLOAD"],
            owner=row["OWNER"],
            created_date=row["CREATED_DATE"],
            last_modified_date=row["LAST_MODIFIED_DATE"],
        )
        for row in rows
    ]


def compute_available_tasks(tasks: list[TaskRow]) -> tuple[list[TaskRow], list[TaskRow]]:
    owned_groups = {task.group_id for task in tasks if task.owner and task.group_id}
    available = [
        task
        for task in tasks
        if task.owner is None and (task.group_id is None or task.group_id not in owned_groups)
    ]
    blocked = [
        task
        for task in tasks
        if task.owner is None and task.group_id is not None and task.group_id in owned_groups
    ]
    return available, blocked


def task_age_hours(task: TaskRow, now: datetime) -> float | None:
    if task.created_at is None:
        return None
    return (now - task.created_at).total_seconds() / 3600.0


def modified_age_hours(task: TaskRow, now: datetime) -> float | None:
    if task.modified_at is None:
        return None
    return (now - task.modified_at).total_seconds() / 3600.0


def summarize_payload_keys(tasks: list[TaskRow]) -> dict[str, Counter[str]]:
    result: dict[str, Counter[str]] = defaultdict(Counter)
    for task in tasks:
        payload = task.payload_json
        if payload:
            result[task.simple_type].update(payload.keys())
    return result


def stats_for_numbers(values: list[float]) -> dict[str, float] | None:
    if not values:
        return None
    sorted_values = sorted(values)
    return {
        "min": round(sorted_values[0], 2),
        "median": round(statistics.median(sorted_values), 2),
        "p95": round(percentile(sorted_values, 95), 2),
        "max": round(sorted_values[-1], 2),
    }


def percentile(sorted_values: list[float], pct: float) -> float:
    if len(sorted_values) == 1:
        return sorted_values[0]
    position = (len(sorted_values) - 1) * (pct / 100.0)
    lower = int(position)
    upper = min(lower + 1, len(sorted_values) - 1)
    weight = position - lower
    return sorted_values[lower] * (1 - weight) + sorted_values[upper] * weight


def build_report(tasks: list[TaskRow], limit: int, stale_hours: float) -> dict[str, Any]:
    now = datetime.now(timezone.utc)
    available, blocked = compute_available_tasks(tasks)
    owned = [task for task in tasks if task.owner]
    unowned = [task for task in tasks if task.owner is None]

    counts_by_type = Counter(task.simple_type for task in tasks)
    owned_by_type = Counter(task.simple_type for task in owned)
    available_by_type = Counter(task.simple_type for task in available)
    blocked_by_type = Counter(task.simple_type for task in blocked)
    owner_counts = Counter(task.owner for task in owned if task.owner)
    group_counts = Counter(task.group_id for task in tasks if task.group_id)
    blocked_groups = Counter(task.group_id for task in blocked if task.group_id)
    payload_lengths_by_type: defaultdict[str, list[int]] = defaultdict(list)
    for task in tasks:
        payload_lengths_by_type[task.simple_type].append(task.payload_length)

    stale_owned = [
        task
        for task in owned
        if (age := modified_age_hours(task, now)) is not None and age >= stale_hours
    ]
    stale_owned.sort(key=lambda task: modified_age_hours(task, now) or 0.0, reverse=True)

    oldest_created = sorted(tasks, key=lambda task: task.created_at or now)[:limit]
    stalest_modified = sorted(tasks, key=lambda task: task.modified_at or now)[:limit]
    highest_priority_available = sorted(
        available,
        key=lambda task: (-task.priority, task.modified_at or now),
    )[:limit]

    blocking_owners: list[JsonDict] = []
    for group_id, block_count in blocked_groups.most_common(limit):
        owner_samples = sorted(
            [task for task in owned if task.group_id == group_id],
            key=lambda task: task.modified_at or now,
        )
        blocking_owners.append(
            {
                "group_id": group_id,
                "blocked_count": block_count,
                "owners": [
                    {
                        "id": task.id,
                        "simple_type": task.simple_type,
                        "owner": task.owner,
                        "last_modified_age_h": round(modified_age_hours(task, now) or 0.0, 2),
                    }
                    for task in owner_samples[:limit]
                ],
            }
        )

    report: dict[str, Any] = {
        "db_snapshot": {
            "total_tasks": len(tasks),
            "owned_tasks": len(owned),
            "unowned_tasks": len(unowned),
            "available_tasks": len(available),
            "blocked_by_group_tasks": len(blocked),
            "distinct_owners": len(owner_counts),
            "distinct_groups": len(group_counts),
            "stale_owned_threshold_hours": stale_hours,
            "stale_owned_tasks": len(stale_owned),
        },
        "counts_by_type": counts_by_type,
        "owned_by_type": owned_by_type,
        "available_by_type": available_by_type,
        "blocked_by_type": blocked_by_type,
        "owner_counts": owner_counts,
        "largest_groups": group_counts.most_common(limit),
        "blocking_groups": blocking_owners,
        "payload_length_by_type": {
            task_type: stats_for_numbers([float(value) for value in values])
            for task_type, values in payload_lengths_by_type.items()
        },
        "payload_keys_by_type": {
            task_type: counter.most_common(limit)
            for task_type, counter in summarize_payload_keys(tasks).items()
        },
        "task_age_hours": stats_for_numbers(
            [age for task in tasks if (age := task_age_hours(task, now)) is not None]
        ),
        "task_modified_age_hours": stats_for_numbers(
            [age for task in tasks if (age := modified_age_hours(task, now)) is not None]
        ),
        "oldest_created_tasks": [serialize_task(task, now) for task in oldest_created],
        "stalest_modified_tasks": [serialize_task(task, now) for task in stalest_modified],
        "highest_priority_available_tasks": [serialize_task(task, now) for task in highest_priority_available],
        "stale_owned_tasks": [serialize_task(task, now) for task in stale_owned[:limit]],
        "observations": build_observations(tasks, available, blocked, stale_owned),
    }
    return report


def serialize_task(task: TaskRow, now: datetime) -> dict[str, Any]:
    payload = task.payload_json
    return {
        "id": task.id,
        "simple_type": task.simple_type,
        "priority": task.priority,
        "group_id": task.group_id,
        "owner": task.owner,
        "created_date": task.created_date,
        "last_modified_date": task.last_modified_date,
        "created_age_h": round(task_age_hours(task, now) or 0.0, 2),
        "modified_age_h": round(modified_age_hours(task, now) or 0.0, 2),
        "payload_length": task.payload_length,
        "payload_keys": sorted(payload.keys()) if payload else None,
    }


def build_observations(
    tasks: list[TaskRow],
    available: list[TaskRow],
    blocked: list[TaskRow],
    stale_owned: list[TaskRow],
) -> list[str]:
    observations: list[str] = []
    if not tasks:
        observations.append("Queue is empty in this snapshot.")
        return observations
    if stale_owned:
        observations.append(
            f"Detected {len(stale_owned)} owned tasks older than the stale threshold; these may represent stuck workers or long-running scans."
        )
    if blocked:
        observations.append(
            f"Detected {len(blocked)} unowned tasks blocked by GROUP_ID locking; investigate owners holding the corresponding groups."
        )
    if tasks and not available and all(task.owner for task in tasks):
        observations.append(
            "All tasks in the snapshot are already owned; if queue depth outside this snapshot is still growing, investigate long-running workers rather than claim contention."
        )
    if available and len(available) < len(tasks) / 4:
        observations.append(
            "Only a small share of queued tasks are currently claimable; queue throughput may be constrained by owner/group locking rather than raw task count."
        )
    if any(task.simple_type == "ScanLibrary" for task in tasks):
        observations.append(
            "ScanLibrary tasks are present in the snapshot; compare their modified age with queue drain rate to detect scans that monopolize the queue."
        )
    if not observations:
        observations.append("No obvious queue anomaly detected from this snapshot alone; compare multiple snapshots over time for better signal.")
    return observations


def render_text_report(report: dict[str, Any], limit: int) -> str:
    lines: list[str] = []
    snapshot = report["db_snapshot"]

    lines.append("# Komga Tasks DB Analysis")
    lines.append("")
    lines.append("## Snapshot Summary")
    for key, value in snapshot.items():
        lines.append(f"- {key}: {value}")

    lines.append("")
    lines.append("## Observations")
    for value in report["observations"]:
        lines.append(f"- {value}")

    append_counter_section(lines, "Counts By Type", report["counts_by_type"], limit)
    append_counter_section(lines, "Owned By Type", report["owned_by_type"], limit)
    append_counter_section(lines, "Available By Type", report["available_by_type"], limit)
    append_counter_section(lines, "Blocked By Type", report["blocked_by_type"], limit)
    append_counter_section(lines, "Owner Counts", report["owner_counts"], limit)

    lines.append("")
    lines.append("## Age Statistics")
    lines.append(f"- created_age_hours: {report['task_age_hours']}")
    lines.append(f"- modified_age_hours: {report['task_modified_age_hours']}")

    append_pairs_section(lines, "Largest Groups", report["largest_groups"])

    lines.append("")
    lines.append("## Blocking Groups")
    if not report["blocking_groups"]:
        lines.append("- none")
    else:
        for entry in report["blocking_groups"]:
            lines.append(f"- group_id={entry['group_id']} blocked_count={entry['blocked_count']}")
            for owner in entry["owners"]:
                lines.append(
                    "  owner_task="
                    f"{owner['id']} type={owner['simple_type']} owner={owner['owner']} modified_age_h={owner['last_modified_age_h']}"
                )

    lines.append("")
    lines.append("## Payload Length By Type")
    for task_type, stats in sorted(report["payload_length_by_type"].items()):
        lines.append(f"- {task_type}: {stats}")

    lines.append("")
    lines.append("## Payload Keys By Type")
    for task_type, keys in sorted(report["payload_keys_by_type"].items()):
        lines.append(f"- {task_type}: {keys}")

    append_task_table(lines, "Oldest Created Tasks", report["oldest_created_tasks"])
    append_task_table(lines, "Stalest Modified Tasks", report["stalest_modified_tasks"])
    append_task_table(lines, "Highest Priority Available Tasks", report["highest_priority_available_tasks"])
    append_task_table(lines, "Stale Owned Tasks", report["stale_owned_tasks"])

    return "\n".join(lines)


def append_counter_section(lines: list[str], title: str, counter: Counter[str], limit: int) -> None:
    lines.append("")
    lines.append(f"## {title}")
    if not counter:
        lines.append("- none")
        return
    for name, count in counter.most_common(limit):
        lines.append(f"- {name}: {count}")


def append_pairs_section(lines: list[str], title: str, values: list[tuple[Any, Any]]) -> None:
    lines.append("")
    lines.append(f"## {title}")
    if not values:
        lines.append("- none")
        return
    for left, right in values:
        lines.append(f"- {left}: {right}")


def append_task_table(lines: list[str], title: str, tasks: list[dict[str, Any]]) -> None:
    lines.append("")
    lines.append(f"## {title}")
    if not tasks:
        lines.append("- none")
        return
    for task in tasks:
        lines.append(
            "- "
            f"id={task['id']} type={task['simple_type']} priority={task['priority']} "
            f"owner={task['owner']} group={task['group_id']} created_age_h={task['created_age_h']} "
            f"modified_age_h={task['modified_age_h']} payload_length={task['payload_length']}"
        )


def main() -> int:
    args = parse_args()
    db_path = Path(args.db_path)
    if not db_path.exists():
        raise SystemExit(f"Database file not found: {db_path}")

    tasks = fetch_tasks(db_path)
    report = build_report(tasks, args.limit, args.stale_hours)

    if args.json:
        print(json.dumps(report, indent=2, default=list))
    else:
        print(render_text_report(report, args.limit))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())