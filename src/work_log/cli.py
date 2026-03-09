from __future__ import annotations

import argparse
import sys
from collections import defaultdict
from datetime import date, datetime, timedelta
from pathlib import Path

from work_log.config import ConfigError, Settings
from work_log.generator import build_achievement_prompts, build_review_prompts
from work_log.markdown_formatter import render_daily_log
from work_log.models import DailyLog
from work_log.notion_client import NotionClient
from work_log.notion_parser import parse_outline_text
from work_log.openai_client import OpenAIClient
from work_log.storage import (
    achievement_path,
    daily_log_path,
    ensure_project_dirs,
    load_daily_logs,
    review_path,
    write_text,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="work-log")
    subparsers = parser.add_subparsers(dest="command", required=True)

    sync_daily = subparsers.add_parser("sync-daily")
    sync_daily.add_argument("--date", dest="entry_date")
    sync_daily.add_argument("--dry-run", action="store_true")

    sync_range = subparsers.add_parser("sync-range")
    sync_range.add_argument("--from", dest="start_date", required=True)
    sync_range.add_argument("--to", dest="end_date", required=True)
    sync_range.add_argument("--dry-run", action="store_true")

    achievement = subparsers.add_parser("generate-achievement")
    achievement.add_argument("--from", dest="start_date", required=True)
    achievement.add_argument("--to", dest="end_date", required=True)
    achievement.add_argument("--slug", required=True)
    achievement.add_argument("--dry-run", action="store_true")

    review = subparsers.add_parser("generate-review")
    review.add_argument("--period", required=True)
    review.add_argument("--from", dest="start_date")
    review.add_argument("--to", dest="end_date")
    review.add_argument("--dry-run", action="store_true")

    return parser


def main(argv: list[str] | None = None, root: Path | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    repo_root = root or Path(__file__).resolve().parents[2]
    ensure_project_dirs(repo_root)
    settings = Settings.from_env(repo_root)

    try:
        if args.command == "sync-daily":
            return handle_sync_daily(args, settings)
        if args.command == "sync-range":
            return handle_sync_range(args, settings)
        if args.command == "generate-achievement":
            return handle_generate_achievement(args, settings)
        if args.command == "generate-review":
            return handle_generate_review(args, settings)
    except (ConfigError, RuntimeError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


def handle_sync_daily(args: argparse.Namespace, settings: Settings) -> int:
    entry_date = parse_date(args.entry_date) if args.entry_date else today_in_timezone(settings)
    return sync_range(entry_date, entry_date, settings, dry_run=args.dry_run)


def handle_sync_range(args: argparse.Namespace, settings: Settings) -> int:
    start_date = parse_date(args.start_date)
    end_date = parse_date(args.end_date)
    return sync_range(start_date, end_date, settings, dry_run=args.dry_run)


def handle_generate_achievement(args: argparse.Namespace, settings: Settings) -> int:
    settings.require_openai()
    start_date = parse_date(args.start_date)
    end_date = parse_date(args.end_date)
    logs = load_daily_logs(settings.root, start_date, end_date)
    if not logs:
        raise RuntimeError("No local daily logs found in the requested range.")

    client = OpenAIClient(settings.openai_api_key or "", settings.openai_model or "")
    system_prompt, user_prompt = build_achievement_prompts(
        logs,
        args.slug,
        start_date,
        end_date,
    )
    content = client.generate_markdown(system_prompt, user_prompt)
    target = achievement_path(settings.root, args.slug)
    emit_result(target, content, args.dry_run)
    return 0


def handle_generate_review(args: argparse.Namespace, settings: Settings) -> int:
    settings.require_openai()
    start_date = parse_date(args.start_date) if args.start_date else infer_period_start(args.period)
    end_date = parse_date(args.end_date) if args.end_date else infer_period_end(args.period)
    logs = load_daily_logs(settings.root, start_date, end_date)
    if not logs:
        raise RuntimeError("No local daily logs found in the requested range.")

    client = OpenAIClient(settings.openai_api_key or "", settings.openai_model or "")
    system_prompt, user_prompt = build_review_prompts(
        logs,
        args.period,
        start_date,
        end_date,
    )
    content = client.generate_markdown(system_prompt, user_prompt)
    target = review_path(settings.root, args.period)
    emit_result(target, content, args.dry_run)
    return 0


def sync_range(
    start_date: date,
    end_date: date,
    settings: Settings,
    dry_run: bool,
) -> int:
    if end_date < start_date:
        raise ValueError("The end date must be on or after the start date.")
    settings.require_notion()
    client = NotionClient(
        settings.notion_token or "",
        settings.notion_database_id or "",
        settings.notion_date_property or "",
    )
    pages = client.query_pages(start_date, end_date)
    logs_by_date: dict[date, DailyLog] = {}

    for page in pages:
        parsed = parse_outline_text(page.outline_text, page.entry_date, source_page=page.page_id)
        if page.entry_date not in logs_by_date:
            logs_by_date[page.entry_date] = parsed
        else:
            logs_by_date[page.entry_date].merge(parsed)

    for entry_date in sorted(logs_by_date):
        log = logs_by_date[entry_date]
        target = daily_log_path(settings.root, entry_date)
        content = render_daily_log(log)
        emit_result(target, content, dry_run)
    return 0


def emit_result(target: Path, content: str, dry_run: bool) -> None:
    if dry_run:
        print(f"--- {target.as_posix()} ---")
        print(content, end="")
        return
    write_text(target, content)
    print(f"Wrote {target.as_posix()}")


def parse_date(value: str) -> date:
    return date.fromisoformat(value)


def today_in_timezone(settings: Settings) -> date:
    return datetime.now(settings.timezone).date()


def infer_period_start(period: str) -> date:
    year_text, half = period.split("-", 1)
    year = int(year_text)
    if half.upper() == "H1":
        return date(year, 1, 1)
    if half.upper() == "H2":
        return date(year, 7, 1)
    raise ValueError("Period must look like YYYY-H1 or YYYY-H2.")


def infer_period_end(period: str) -> date:
    year_text, half = period.split("-", 1)
    year = int(year_text)
    if half.upper() == "H1":
        return date(year, 6, 30)
    if half.upper() == "H2":
        return date(year, 12, 31)
    raise ValueError("Period must look like YYYY-H1 or YYYY-H2.")
