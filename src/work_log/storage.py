from __future__ import annotations

from datetime import date
from pathlib import Path

from work_log.models import DailyLog
from work_log.notion_parser import parse_outline_text


def ensure_project_dirs(root: Path) -> None:
    for relative in (
        "daily",
        "weekly",
        "achievements",
        "reviews",
        "tech-notes",
        "templates",
        ".github/workflows",
        "tests",
    ):
        (root / relative).mkdir(parents=True, exist_ok=True)


def daily_log_path(root: Path, entry_date: date) -> Path:
    return root / "daily" / f"{entry_date.year}" / f"{entry_date.isoformat()}.md"


def achievement_path(root: Path, slug: str) -> Path:
    return root / "achievements" / f"{slug}.md"


def review_path(root: Path, period: str) -> Path:
    return root / "reviews" / f"{period}.md"


def write_text(path: Path, content: str, dry_run: bool = False) -> None:
    if dry_run:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def load_daily_logs(root: Path, start_date: date, end_date: date) -> list[DailyLog]:
    logs: list[DailyLog] = []
    current = start_date
    while current <= end_date:
        path = daily_log_path(root, current)
        if path.exists():
            content = path.read_text(encoding="utf-8")
            logs.append(parse_outline_text(content, current, source_page=path.as_posix()))
        current = date.fromordinal(current.toordinal() + 1)
    return logs
