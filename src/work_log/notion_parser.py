from __future__ import annotations

import re
from datetime import date

from work_log.models import DailyLog, DoneItem

HEADING_RE = re.compile(r"^(#{1,6})\s+(.*)$")
BULLET_RE = re.compile(r"^(\s*)-\s+(.*)$")
SECTION_ALIASES = {
    "task": "today",
    "tasks": "today",
    "todo": "today",
    "today": "today",
    "done": "done",
    "impact": "impact",
    "support": "support",
    "improvement": "improvements",
    "improvements": "improvements",
    "learning": "learning",
    "notes": "notes",
}


def parse_outline_text(
    outline_text: str,
    entry_date: date,
    source_page: str | None = None,
) -> DailyLog:
    log = DailyLog(entry_date=entry_date)
    if source_page:
        log.source_pages.append(source_page)

    current_section: str | None = None
    current_done: DoneItem | None = None
    standalone_impacts: list[str] = []

    for raw_line in outline_text.splitlines():
        line = raw_line.rstrip()
        if not line.strip():
            continue

        heading_match = HEADING_RE.match(line)
        if heading_match:
            current_section = normalize_section_name(heading_match.group(2))
            current_done = None
            continue

        if not current_section:
            continue

        indent, text = parse_content_line(line)
        if not text:
            continue

        if current_section == "today":
            continue

        if current_section == "done":
            if indent == 0 and not is_impact_line(text):
                current_done = DoneItem(task=strip_task_prefix(text))
                log.done.append(current_done)
                continue

            if is_impact_line(text):
                impact_text = strip_impact_prefix(text)
                if current_done is not None:
                    current_done.impact = append_text(current_done.impact, impact_text)
                else:
                    standalone_impacts.append(impact_text)
                continue

            if current_done is not None:
                current_done.task = append_text(current_done.task, text)
            continue

        if current_section == "impact":
            standalone_impacts.append(strip_impact_prefix(text))
            continue

        section_items = getattr(log, current_section)
        section_items.append(text)

    fill_missing_impacts(log, standalone_impacts)
    return log


def normalize_section_name(name: str) -> str | None:
    key = re.sub(r"[^a-z]+", "", name.lower())
    return SECTION_ALIASES.get(key)


def parse_content_line(line: str) -> tuple[int, str]:
    bullet_match = BULLET_RE.match(line)
    if bullet_match:
        indent = len(bullet_match.group(1)) // 2
        return indent, normalize_whitespace(bullet_match.group(2))
    indent = (len(line) - len(line.lstrip(" "))) // 2
    return indent, normalize_whitespace(line.strip())


def normalize_whitespace(text: str) -> str:
    return " ".join(text.split())


def is_impact_line(text: str) -> bool:
    return text.lower().startswith("impact:")


def strip_impact_prefix(text: str) -> str:
    return text.split(":", 1)[1].strip() if ":" in text else text.strip()


def strip_task_prefix(text: str) -> str:
    if text.lower().startswith("task:"):
        return text.split(":", 1)[1].strip()
    return text


def append_text(current: str | None, new_text: str) -> str:
    if not current:
        return new_text
    return f"{current} {new_text}".strip()


def fill_missing_impacts(log: DailyLog, standalone_impacts: list[str]) -> None:
    impact_iter = iter(standalone_impacts)
    for item in log.done:
        if item.impact:
            continue
        try:
            item.impact = next(impact_iter)
        except StopIteration:
            break
