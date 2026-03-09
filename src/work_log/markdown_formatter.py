from __future__ import annotations

from work_log.models import DailyLog


def render_daily_log(log: DailyLog) -> str:
    sections: list[str] = []

    if log.done:
        lines = ["## done"]
        for item in log.done:
            lines.append(f"- task: {item.task}")
            if item.impact:
                lines.append(f"  impact: {item.impact}")
        sections.append("\n".join(lines))

    for section_name, items in (
        ("support", log.support),
        ("improvements", log.improvements),
        ("learning", log.learning),
        ("notes", log.notes),
    ):
        if not items:
            continue
        lines = [f"## {section_name}"]
        lines.extend(f"- {item}" for item in items)
        sections.append("\n".join(lines))

    body = "\n\n".join(sections)
    if body:
        return f"# {log.entry_date.isoformat()}\n\n{body}\n"
    return f"# {log.entry_date.isoformat()}\n"
