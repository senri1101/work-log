from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from zoneinfo import ZoneInfo


class ConfigError(RuntimeError):
    """Raised when required configuration is missing."""


@dataclass(frozen=True)
class Settings:
    root: Path
    tz_name: str
    notion_token: str | None
    notion_database_id: str | None
    notion_date_property: str | None
    openai_api_key: str | None
    openai_model: str | None

    @property
    def timezone(self) -> ZoneInfo:
        return ZoneInfo(self.tz_name)

    @classmethod
    def from_env(cls, root: Path) -> "Settings":
        return cls(
            root=root,
            tz_name=os.getenv("TZ", "Asia/Tokyo"),
            notion_token=os.getenv("NOTION_TOKEN"),
            notion_database_id=os.getenv("NOTION_DATABASE_ID"),
            notion_date_property=os.getenv("NOTION_DATE_PROPERTY"),
            openai_api_key=os.getenv("OPENAI_API_KEY"),
            openai_model=os.getenv("OPENAI_MODEL"),
        )

    def require_notion(self) -> None:
        missing = [
            name
            for name, value in (
                ("NOTION_TOKEN", self.notion_token),
                ("NOTION_DATABASE_ID", self.notion_database_id),
                ("NOTION_DATE_PROPERTY", self.notion_date_property),
            )
            if not value
        ]
        if missing:
            raise ConfigError(f"Missing Notion configuration: {', '.join(missing)}")

    def require_openai(self) -> None:
        missing = [
            name
            for name, value in (
                ("OPENAI_API_KEY", self.openai_api_key),
                ("OPENAI_MODEL", self.openai_model),
            )
            if not value
        ]
        if missing:
            raise ConfigError(f"Missing OpenAI configuration: {', '.join(missing)}")
