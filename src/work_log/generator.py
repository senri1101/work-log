from __future__ import annotations

from datetime import date

from work_log.models import DailyLog, TaskStatus


def build_achievement_prompts(
    logs: list[DailyLog],
    slug: str,
    start_date: date,
    end_date: date,
) -> tuple[str, str]:
    system_prompt = (
        "あなたはエンジニアの実績ログをMarkdownで作成するアシスタントです。"
        "完了済みタスクを主な根拠にし、着手中タスクとメモを補足として使ってください。"
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
        "完了済みタスクの反復テーマを最優先でまとめ、その根拠として着手中タスクとメモを使ってください。"
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
        completed = build_status_lines(log, TaskStatus.DONE)
        doing = build_status_lines(log, TaskStatus.DOING)
        pending = build_status_lines(log, TaskStatus.TODO)

        if completed:
            lines.append("done:")
            lines.extend(completed)
        if doing:
            lines.append("doing:")
            lines.extend(doing)
        if pending:
            lines.append("pending:")
            lines.extend(pending)
        if log.memo_lines:
            lines.append("memo:")
            lines.extend(f"- {item}" for item in log.memo_lines)
        chunks.append("\n".join(lines))
    return "\n\n".join(chunks)


def build_status_lines(log: DailyLog, status: TaskStatus) -> list[str]:
    lines: list[str] = []
    for bucket, path, node in log.iter_tasks():
        if node.status != status:
            continue
        line = f"- [{bucket}] {path}"
        if node.notes:
            line += f" | notes: {' / '.join(node.notes)}"
        lines.append(line)
    return lines
