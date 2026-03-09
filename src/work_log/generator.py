from __future__ import annotations

from datetime import date

from work_log.models import DailyLog


def build_achievement_prompts(
    logs: list[DailyLog],
    slug: str,
    start_date: date,
    end_date: date,
) -> tuple[str, str]:
    system_prompt = (
        "You are preparing a concise engineering achievement log in markdown. "
        "Prioritize impact statements first, then use done items, support, "
        "improvements, learning, and notes to explain the work. "
        "Return markdown only."
    )
    user_prompt = (
        f"Create an achievement file titled '# {slug}'.\n"
        "Use this exact structure:\n"
        "## problem\n"
        "## solution\n"
        "## impact\n\n"
        f"Date range: {start_date.isoformat()} to {end_date.isoformat()}\n\n"
        f"{build_context(logs)}"
    )
    return system_prompt, user_prompt


def build_review_prompts(
    logs: list[DailyLog],
    period: str,
    start_date: date,
    end_date: date,
) -> tuple[str, str]:
    system_prompt = (
        "You are preparing a concise self-review draft in markdown for an engineer. "
        "Prioritize recurring impact themes, then support them with done items, "
        "support work, improvements, learning, and notes. Return markdown only."
    )
    user_prompt = (
        f"Create a review file titled '# {period}'.\n"
        "Use this exact structure:\n"
        "## summary\n"
        "## key contributions\n"
        "## support\n"
        "## improvement themes\n"
        "## next actions\n\n"
        f"Date range: {start_date.isoformat()} to {end_date.isoformat()}\n\n"
        f"{build_context(logs)}"
    )
    return system_prompt, user_prompt


def build_context(logs: list[DailyLog]) -> str:
    chunks: list[str] = []
    for log in sorted(logs, key=lambda item: item.entry_date):
        lines = [f"## {log.entry_date.isoformat()}"]
        if log.done:
            lines.append("done:")
            for item in log.done:
                line = f"- {item.task}"
                if item.impact:
                    line += f" | impact: {item.impact}"
                lines.append(line)
        if log.support:
            lines.append("support:")
            lines.extend(f"- {item}" for item in log.support)
        if log.improvements:
            lines.append("improvements:")
            lines.extend(f"- {item}" for item in log.improvements)
        if log.learning:
            lines.append("learning:")
            lines.extend(f"- {item}" for item in log.learning)
        if log.notes:
            lines.append("notes:")
            lines.extend(f"- {item}" for item in log.notes)
        chunks.append("\n".join(lines))
    return "\n\n".join(chunks)
