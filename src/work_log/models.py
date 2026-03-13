from __future__ import annotations

from dataclasses import dataclass, field
from datetime import date
from enum import StrEnum
from typing import Iterator


class TaskStatus(StrEnum):
    TODO = "todo"
    DOING = "doing"
    DONE = "done"


@dataclass
class TaskNode:
    text: str
    status: TaskStatus = TaskStatus.TODO
    children: list["TaskNode"] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def is_empty(self) -> bool:
        return not self.text.strip() and not self.children and not self.notes

    def normalized(self) -> "TaskNode":
        normalized_children = [child.normalized() for child in self.children]
        return TaskNode(
            text=self.text.strip(),
            status=self.status,
            children=[child for child in normalized_children if not child.is_empty()],
            notes=[note.strip() for note in self.notes if note.strip()],
        )

    def iter_nodes(self, prefix: str = "") -> Iterator[tuple[str, "TaskNode"]]:
        path = self.text.strip() if not prefix else f"{prefix} > {self.text.strip()}"
        yield path, self
        for child in self.children:
            yield from child.iter_nodes(path)

    def carry_over(self) -> "TaskNode | None":
        carried_children = [
            child
            for child in (nested.carry_over() for nested in self.children)
            if child is not None
        ]
        if self.status == TaskStatus.DONE and not carried_children:
            return None
        next_status = self.status
        if self.status == TaskStatus.DONE and carried_children:
            next_status = TaskStatus.DOING
        return TaskNode(
            text=self.text.strip(),
            status=next_status,
            children=carried_children,
            notes=[note.strip() for note in self.notes if note.strip()],
        )


@dataclass
class DailyLog:
    entry_date: date
    must_do_tasks: list[TaskNode] = field(default_factory=list)
    queued_tasks: list[TaskNode] = field(default_factory=list)
    pending_tasks: list[TaskNode] = field(default_factory=list)
    memo_lines: list[str] = field(default_factory=list)
    source_pages: list[str] = field(default_factory=list)

    def merge(self, other: "DailyLog") -> None:
        if self.entry_date != other.entry_date:
            raise ValueError("Cannot merge logs for different dates.")
        self.must_do_tasks.extend(other.must_do_tasks)
        self.queued_tasks.extend(other.queued_tasks)
        self.pending_tasks.extend(other.pending_tasks)
        self.memo_lines.extend(other.memo_lines)
        self.source_pages.extend(other.source_pages)

    def iter_tasks(self) -> Iterator[tuple[str, str, TaskNode]]:
        sections = (
            ("must_do", self.must_do_tasks),
            ("queued", self.queued_tasks),
            ("pending", self.pending_tasks),
        )
        for bucket, tasks in sections:
            for task in tasks:
                for path, node in task.iter_nodes():
                    yield bucket, path, node

    def completed_task_paths(self) -> list[str]:
        return [
            path
            for _, path, node in self.iter_tasks()
            if node.status == TaskStatus.DONE and path
        ]

    def active_task_paths(self) -> list[str]:
        return [
            path
            for _, path, node in self.iter_tasks()
            if node.status == TaskStatus.DOING and path
        ]

    def incomplete_tasks_for_next_day(self) -> tuple[list[TaskNode], list[TaskNode], list[TaskNode]]:
        carried: list[list[TaskNode]] = [[], [], []]
        for index, tasks in enumerate(
            (self.must_do_tasks, self.queued_tasks, self.pending_tasks)
        ):
            carried[index] = [
                item
                for item in (task.carry_over() for task in tasks)
                if item is not None
            ]
        return carried[0], carried[1], carried[2]


@dataclass
class NotionPage:
    page_id: str
    entry_date: date
    title: str | None
    outline_text: str
