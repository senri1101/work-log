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
    gemini_api_key: str | None
    gemini_model: str | None

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
            gemini_api_key=os.getenv("GEMINI_API_KEY"),
            gemini_model=os.getenv("GEMINI_MODEL", "gemini-2.5-flash-lite"),
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
            raise ConfigError(f"Notion設定が不足しています: {', '.join(missing)}")

    def require_gemini(self) -> None:
        missing = [
            name
            for name, value in (
                ("GEMINI_API_KEY", self.gemini_api_key),
                ("GEMINI_MODEL", self.gemini_model),
            )
            if not value
        ]
        if missing:
            raise ConfigError(f"Gemini設定が不足しています: {', '.join(missing)}")
