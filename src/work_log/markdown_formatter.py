from __future__ import annotations

from work_log.models import DailyLog, TaskNode, TaskStatus


def render_daily_log(log: DailyLog) -> str:
    lines = [
        f"# {log.entry_date.isoformat()}",
        "",
        "## ✅ 今日やること",
        "",
        "### 🚨 今日必達",
    ]
    lines.extend(render_task_group(log.must_do_tasks))
    lines.extend(
        [
            "",
            "### 🐻 必達以外",
        ]
    )
    lines.extend(render_task_group(log.queued_tasks))
    lines.extend(
        [
            "",
            "## 📝 メモ / 気づき",
        ]
    )
    if log.memo_lines:
        lines.extend(f"- {item}" for item in log.memo_lines)
    lines.extend(
        [
            "",
            "## 🐕 保留",
        ]
    )
    lines.extend(render_task_group(log.pending_tasks))
    return "\n".join(lines).rstrip() + "\n"


def render_task_group(tasks: list[TaskNode], depth: int = 0) -> list[str]:
    lines: list[str] = []
    for task in tasks:
        token = status_token(task.status)
        prefix = "  " * depth
        lines.append(f"{prefix}- [{token}] {task.text.strip()}")
        for note in task.notes:
            lines.append(f"{prefix}  - {note}")
        lines.extend(render_task_group(task.children, depth + 1))
    return lines


def status_token(status: TaskStatus) -> str:
    return {
        TaskStatus.TODO: " ",
        TaskStatus.DOING: "/",
        TaskStatus.DONE: "x",
    }[status]
