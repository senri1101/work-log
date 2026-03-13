from __future__ import annotations

import re
from datetime import date

from work_log.models import DailyLog, TaskNode, TaskStatus

HEADING_RE = re.compile(r"^(#{1,6})\s+(.*)$")
BULLET_RE = re.compile(r"^(\s*)-\s+(.*)$")
CHECKBOX_RE = re.compile(r"^\[( |/|x|X)\]\s+(.*)$")
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
LEGACY_MEMO_PREFIX = {
    "support": "support",
    "improvements": "improvement",
    "learning": "learning",
    "notes": "note",
}
TASK_SECTIONS = {"must_do", "queued", "pending", "today", "done"}


def parse_outline_text(
    outline_text: str,
    entry_date: date,
    source_page: str | None = None,
) -> DailyLog:
    log = DailyLog(entry_date=entry_date)
    if source_page:
        log.source_pages.append(source_page)

    current_section: str | None = None
    current_stack: list[tuple[int, TaskNode]] = []
    standalone_impacts: list[str] = []

    for raw_line in outline_text.splitlines():
        line = raw_line.rstrip()
        if not line.strip():
            continue

        heading_match = HEADING_RE.match(line)
        if heading_match:
            current_section = normalize_section_name(heading_match.group(2))
            current_stack = []
            continue

        if not current_section:
            continue

        indent, text = parse_content_line(line)
        if not text:
            continue

        if current_section in TASK_SECTIONS:
            if handle_task_line(log, current_section, current_stack, indent, text, standalone_impacts):
                continue
            current_stack = []
            continue

        if current_section == "impact":
            standalone_impacts.append(strip_impact_prefix(text))
            continue

        if current_section == "memo":
            log.memo_lines.append(text)
            continue

        if current_section in LEGACY_MEMO_PREFIX:
            log.memo_lines.append(f"{LEGACY_MEMO_PREFIX[current_section]}: {text}")

    fill_missing_impacts(log, standalone_impacts)
    return log


def normalize_section_name(name: str) -> str | None:
    compact = normalize_whitespace(name)
    if "今日やること" in compact:
        return "tasks"
    if "今日必達" in compact:
        return "must_do"
    if "必達以外" in compact:
        return "queued"
    if "メモ" in compact or "気づき" in compact:
        return "memo"
    if "保留" in compact:
        return "pending"

    key = re.sub(r"[^a-z]+", "", compact.lower())
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


def parse_checkbox(text: str) -> tuple[TaskStatus, str] | None:
    match = CHECKBOX_RE.match(text)
    if not match:
        return None
    token = match.group(1)
    status = {
        " ": TaskStatus.TODO,
        "/": TaskStatus.DOING,
        "x": TaskStatus.DONE,
        "X": TaskStatus.DONE,
    }[token]
    return status, match.group(2).strip()


def handle_task_line(
    log: DailyLog,
    section: str,
    stack: list[tuple[int, TaskNode]],
    indent: int,
    text: str,
    standalone_impacts: list[str],
) -> bool:
    checkbox = parse_checkbox(text)
    if checkbox is not None:
        status, task_text = checkbox
        append_task(log, section, stack, indent, TaskNode(text=task_text, status=status))
        return True

    if section == "done":
        if is_impact_line(text):
            attach_impact(stack, indent, text, standalone_impacts)
            return True
        if indent == 0:
            append_task(
                log,
                section,
                stack,
                indent,
                TaskNode(text=strip_task_prefix(text), status=TaskStatus.DONE),
            )
            return True
        attach_note(stack, indent, text)
        return True

    if section == "today":
        if indent == 0:
            append_task(
                log,
                section,
                stack,
                indent,
                TaskNode(text=strip_task_prefix(text), status=TaskStatus.TODO),
            )
            return True
        attach_note(stack, indent, text)
        return True

    if section in {"must_do", "queued", "pending"}:
        attach_note(stack, indent, text)
        return True

    return False


def append_task(
    log: DailyLog,
    section: str,
    stack: list[tuple[int, TaskNode]],
    indent: int,
    task: TaskNode,
) -> None:
    while stack and stack[-1][0] >= indent:
        stack.pop()

    if stack:
        stack[-1][1].children.append(task)
    else:
        target = {
            "must_do": log.must_do_tasks,
            "queued": log.queued_tasks,
            "pending": log.pending_tasks,
            "today": log.queued_tasks,
            "done": log.queued_tasks,
        }[section]
        target.append(task)

    stack.append((indent, task))


def attach_note(stack: list[tuple[int, TaskNode]], indent: int, text: str) -> None:
    target = note_target(stack, indent)
    if target is None:
        return
    target.notes.append(text)


def attach_impact(
    stack: list[tuple[int, TaskNode]],
    indent: int,
    text: str,
    standalone_impacts: list[str],
) -> None:
    impact_text = strip_impact_prefix(text)
    target = note_target(stack, indent)
    if target is not None:
        target.notes.append(f"impact: {impact_text}")
    else:
        standalone_impacts.append(impact_text)


def note_target(stack: list[tuple[int, TaskNode]], indent: int) -> TaskNode | None:
    if not stack:
        return None
    preferred_indent = max(indent - 1, 0)
    for level, node in reversed(stack):
        if level <= preferred_indent:
            return node
    return stack[0][1]


def is_impact_line(text: str) -> bool:
    return text.lower().startswith("impact:")


def strip_impact_prefix(text: str) -> str:
    return text.split(":", 1)[1].strip() if ":" in text else text.strip()


def strip_task_prefix(text: str) -> str:
    if text.lower().startswith("task:"):
        return text.split(":", 1)[1].strip()
    return text


def fill_missing_impacts(log: DailyLog, standalone_impacts: list[str]) -> None:
    impact_iter = iter(standalone_impacts)
    for _, _, node in log.iter_tasks():
        if node.status != TaskStatus.DONE:
            continue
        if any(note.lower().startswith("impact:") for note in node.notes):
            continue
        try:
            node.notes.append(f"impact: {next(impact_iter)}")
        except StopIteration:
            break
