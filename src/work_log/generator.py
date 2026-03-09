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
        "あなたはエンジニアの実績ログをMarkdownで作成するアシスタントです。"
        "impact を最優先で要約し、done、support、improvements、learning、notes を補足に使ってください。"
        "本文は自然な日本語で書き、Markdown だけを返してください。"
    )
    user_prompt = (
        f"Create an achievement file titled '# {slug}'.\n"
        "Use this exact structure:\n"
        "## problem\n"
        "## solution\n"
        "## impact\n"
        "本文は日本語で書いてください。\n\n"
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
        "あなたはエンジニアの自己評価ドラフトをMarkdownで作成するアシスタントです。"
        "impact の反復テーマを最優先でまとめ、その根拠として done、support、improvements、learning、notes を使ってください。"
        "本文は自然な日本語で書き、Markdown だけを返してください。"
    )
    user_prompt = (
        f"Create a review file titled '# {period}'.\n"
        "Use this exact structure:\n"
        "## summary\n"
        "## key contributions\n"
        "## support\n"
        "## improvement themes\n"
        "## next actions\n"
        "本文は日本語で書いてください。\n\n"
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
