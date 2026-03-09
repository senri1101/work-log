from __future__ import annotations

from dataclasses import dataclass, field
from datetime import date


@dataclass
class DoneItem:
    task: str
    impact: str | None = None


@dataclass
class DailyLog:
    entry_date: date
    done: list[DoneItem] = field(default_factory=list)
    support: list[str] = field(default_factory=list)
    improvements: list[str] = field(default_factory=list)
    learning: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    source_pages: list[str] = field(default_factory=list)

    def merge(self, other: "DailyLog") -> None:
        if self.entry_date != other.entry_date:
            raise ValueError("Cannot merge logs for different dates.")
        self.done.extend(other.done)
        self.support.extend(other.support)
        self.improvements.extend(other.improvements)
        self.learning.extend(other.learning)
        self.notes.extend(other.notes)
        self.source_pages.extend(other.source_pages)


@dataclass
class NotionPage:
    page_id: str
    entry_date: date
    title: str | None
    outline_text: str
